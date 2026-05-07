use std::io::{self, Write};
use std::path::Path;

use crate::config::Config;
use crate::error::{PowError, Result};
use crate::git;
use crate::paths;
use crate::repo_setup;
use crate::resolve;
use crate::workspace::{resolve_workspace_name, Workspace};

pub fn new(
    name: &str,
    force: bool,
    template: Option<&str>,
    from: Option<&str>,
    no_setup: bool,
) -> Result<()> {
    if name.is_empty() || name.contains('/') || name.contains(std::path::MAIN_SEPARATOR) {
        return Err(PowError::Message(format!(
            "invalid workspace name '{name}'"
        )));
    }
    let cfg = Config::load()?;

    // Resolve template up-front so we don't create the directory if it's bogus.
    let tmpl = match template {
        Some(t) => Some(
            cfg.find_template(t)
                .cloned()
                .ok_or_else(|| PowError::Message(format!("template '{t}' not found")))?,
        ),
        None => None,
    };

    let root = paths::workspaces_root()?;
    std::fs::create_dir_all(&root)?;
    let path = root.join(name);
    if path.exists() {
        if !force {
            return Err(PowError::Message(format!(
                "workspace directory already exists at {}. Use --force to overwrite.",
                path.display()
            )));
        }
        std::fs::remove_dir_all(&path)?;
    }
    std::fs::create_dir(&path)?;
    println!("Created workspace '{name}' at {}.", path.display());

    let Some(tmpl) = tmpl else {
        return Ok(());
    };

    if tmpl.repos.is_empty() {
        println!("Template '{}' has no repos.", tmpl.name);
        return Ok(());
    }

    let total = tmpl.repos.len();
    let mut failures: Vec<(String, String)> = Vec::new();
    for repo in &tmpl.repos {
        match add_repo_to_workspace(&cfg, name, &path, repo, None, from, no_setup) {
            Ok(()) => {}
            Err(e) => failures.push((repo.clone(), e.to_string())),
        }
    }

    let succeeded = total - failures.len();
    println!("Added {succeeded} of {total} repos to workspace '{name}'.");
    if !failures.is_empty() {
        eprintln!("Failed:");
        for (repo, msg) in &failures {
            eprintln!("  {repo} — {msg}");
        }
        return Err(PowError::Message(format!(
            "{} repo(s) failed to add to workspace '{name}'",
            failures.len()
        )));
    }
    Ok(())
}

pub fn add(
    repos: &[String],
    workspace: Option<&str>,
    branch: Option<&str>,
    from: Option<&str>,
    no_setup: bool,
) -> Result<()> {
    let cfg = Config::load()?;
    let ws_name = resolve_workspace_name(workspace)?;
    let ws_path = paths::workspace_path(&ws_name)?;
    if !ws_path.exists() {
        return Err(PowError::WorkspaceNotFound(ws_name));
    }

    // Single-repo path preserves the original error type (and exit code).
    if repos.len() == 1 {
        return add_repo_to_workspace(
            &cfg, &ws_name, &ws_path, &repos[0], branch, from, no_setup,
        );
    }

    let total = repos.len();
    let mut failures: Vec<(String, String)> = Vec::new();
    for repo in repos {
        if let Err(e) =
            add_repo_to_workspace(&cfg, &ws_name, &ws_path, repo, branch, from, no_setup)
        {
            failures.push((repo.clone(), e.to_string()));
        }
    }

    let succeeded = total - failures.len();
    println!("Added {succeeded} of {total} repos to workspace '{ws_name}'.");
    if !failures.is_empty() {
        eprintln!("Failed:");
        for (repo, msg) in &failures {
            eprintln!("  {repo} — {msg}");
        }
        return Err(PowError::Message(format!(
            "{} repo(s) failed to add to workspace '{ws_name}'",
            failures.len()
        )));
    }
    Ok(())
}

/// Add a single repo to an already-validated workspace. Used by both
/// `pow add` and `pow new --template`.
fn add_repo_to_workspace(
    cfg: &Config,
    ws_name: &str,
    ws_path: &Path,
    repo: &str,
    branch: Option<&str>,
    from: Option<&str>,
    no_setup: bool,
) -> Result<()> {
    let resolved = resolve::resolve_repo(cfg, repo)?;
    let branch_name = branch.unwrap_or(ws_name).to_string();

    let dest = ws_path.join(&resolved.repo_name);
    if dest.exists() {
        return Err(PowError::Message(format!(
            "destination already exists: {}. Use `pow forget {}` first.",
            dest.display(),
            resolved.repo_name
        )));
    }

    // Self-heal stale worktree metadata first.
    let _ = git::worktree_prune(&resolved.repo_path);

    let branch_already_exists = git::branch_exists(&resolved.repo_path, &branch_name)?;

    let default_base = cfg
        .find_source(&resolved.source_name)
        .map(|s| s.base_branch.clone())
        .unwrap_or_else(|| "main".to_string());
    let base_branch = from.map(|s| s.to_string()).unwrap_or(default_base);

    if branch_already_exists {
        if let Err(e) = git::worktree_add_existing(&resolved.repo_path, &dest, &branch_name) {
            return Err(augment_worktree_error(e, &branch_name, &resolved.repo_path));
        }
    } else if let Err(e) =
        git::worktree_add(&resolved.repo_path, &dest, &branch_name, Some(&base_branch))
    {
        return Err(augment_worktree_error(e, &branch_name, &resolved.repo_path));
    }

    if !no_setup {
        if let Some(setup) = repo_setup::load(&dest)? {
            repo_setup::run_commands(&dest, &setup.commands);
            repo_setup::copy_files(&resolved.repo_path, &dest, &setup.copy);
        }
    }

    println!(
        "Added {} to workspace '{}' on branch '{}'.",
        resolved.repo_name, ws_name, branch_name
    );
    Ok(())
}

pub fn forget(repo: &str, workspace: Option<&str>, prune_branch: bool) -> Result<()> {
    let ws_name = resolve_workspace_name(workspace)?;
    let ws = Workspace::scan(&ws_name)?;

    // locate entry by name
    let entry = ws
        .entries
        .iter()
        .find(|e| {
            e.name == repo
                || format!("{}/{}", e.source_name.clone().unwrap_or_default(), e.name) == repo
        })
        .ok_or_else(|| {
            PowError::RepoNotFound(format!("no entry '{repo}' in workspace '{ws_name}'"))
        })?;

    let source_repo = &entry.source_repo_path;
    // Capture current branch of the worktree before removing.
    let branch = git::current_branch(&entry.path).unwrap_or_default();

    // Self-heal stale metadata.
    let _ = git::worktree_prune(source_repo);

    if let Err(e) = git::worktree_remove(source_repo, &entry.path, false) {
        return Err(PowError::GitFailed(format!(
            "{e}\nThe worktree may have uncommitted changes. Re-run with `--force` is not yet \
             supported; commit/stash changes or remove the directory manually."
        )));
    }

    if prune_branch {
        if branch.is_empty() {
            eprintln!("warning: worktree was in detached HEAD; no branch to prune.");
        } else {
            match git::branch_delete(source_repo, &branch, false) {
                Ok(()) => println!("Deleted branch '{branch}'."),
                Err(e) => eprintln!(
                    "warning: could not delete branch '{branch}' (likely has unmerged commits): {e}"
                ),
            }
        }
    }

    println!("Removed {} from workspace '{ws_name}'.", entry.name);
    Ok(())
}

pub fn rm(name: &str, prune_branches: bool, force: bool) -> Result<()> {
    let ws = Workspace::scan(name)?;

    if !force {
        eprint!(
            "Tear down workspace '{name}' ({} {})? [y/N] ",
            ws.entries.len(),
            if ws.entries.len() == 1 {
                "entry"
            } else {
                "entries"
            }
        );
        io::stderr().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let ans = buf.trim().to_lowercase();
        if ans != "y" && ans != "yes" {
            eprintln!("aborted.");
            return Ok(());
        }
    }

    for entry in &ws.entries {
        let branch = git::current_branch(&entry.path).unwrap_or_default();
        let _ = git::worktree_prune(&entry.source_repo_path);
        if let Err(e) = git::worktree_remove(&entry.source_repo_path, &entry.path, force) {
            eprintln!(
                "warning: could not remove worktree {}: {e}",
                entry.path.display()
            );
        }
        if prune_branches && !branch.is_empty() {
            match git::branch_delete(&entry.source_repo_path, &branch, false) {
                Ok(()) => println!(
                    "Deleted branch '{branch}' in {}.",
                    entry.source_repo_path.display()
                ),
                Err(_) => {
                    if force {
                        match git::branch_delete(&entry.source_repo_path, &branch, true) {
                            Ok(()) => println!(
                                "Force-deleted branch '{branch}' in {}.",
                                entry.source_repo_path.display()
                            ),
                            Err(e) => eprintln!("warning: could not delete branch '{branch}': {e}"),
                        }
                    } else {
                        eprintln!(
                            "warning: branch '{branch}' in {} has unmerged commits; skipping. Re-run with --force to delete.",
                            entry.source_repo_path.display()
                        );
                    }
                }
            }
        }
    }

    if ws.path.exists() {
        std::fs::remove_dir_all(&ws.path)?;
    }
    println!("Removed workspace '{name}'.");
    Ok(())
}

fn augment_worktree_error(err: PowError, branch: &str, source_repo: &Path) -> PowError {
    let msg = err.to_string();
    if msg.contains("already checked out") || msg.contains("already used by worktree") {
        PowError::GitFailed(format!(
            "{msg}\nhint: branch '{branch}' is checked out in another worktree of {}. \
             Remove that worktree first or pick a different branch with `-b`.",
            source_repo.display()
        ))
    } else {
        err
    }
}
