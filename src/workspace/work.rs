use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use futures::stream::StreamExt;

use crate::config::Config;
use crate::error::{PowError, Result};
use crate::git;
use crate::repo_setup;
use crate::workspace::{resolve_workspace_name, show, Entry, Workspace};

pub fn switch(repo: &str, target: &str, new: bool, workspace: Option<&str>) -> Result<()> {
    let ws_name = resolve_workspace_name(workspace)?;
    let ws = Workspace::scan(&ws_name)?;
    let entry = ws.entries.iter().find(|e| e.name == repo).ok_or_else(|| {
        PowError::RepoNotFound(format!("no entry '{repo}' in workspace '{ws_name}'"))
    })?;

    let args: Vec<&str> = if new {
        vec!["checkout", "-b", target]
    } else {
        vec!["checkout", target]
    };
    let out = git::git_raw(&entry.path, &args)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        if stderr.contains("already used by worktree") || stderr.contains("already checked out") {
            return Err(PowError::GitFailed(format!(
                "{stderr}\nhint: '{target}' is checked out in another worktree of this repo."
            )));
        }
        return Err(PowError::GitFailed(stderr));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    for line in stdout.lines().chain(stderr.lines()) {
        println!("{line}");
    }
    Ok(())
}

pub async fn sync(repo: Option<&str>, all: bool, workspace: Option<&str>) -> Result<()> {
    let cfg = Config::load()?;
    let parallel = cfg.settings.parallel.max(1);

    let (targets, ws): (Vec<(String, PathBuf)>, Option<Workspace>) = if all {
        let mut out = Vec::new();
        for s in &cfg.sources {
            let p = match s.expanded_path() {
                Ok(p) => p,
                Err(_) => continue,
            };
            for repo_path in git::list_repos_in(&p).unwrap_or_default() {
                let repo_name = repo_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                out.push((format!("{}/{}", s.name, repo_name), repo_path));
            }
        }
        (out, None)
    } else {
        let ws_name = resolve_workspace_name(workspace)?;
        let ws = Workspace::scan(&ws_name)?;
        let targets = match repo {
            Some(r) => {
                let entry = ws.entries.iter().find(|e| e.name == r).ok_or_else(|| {
                    PowError::RepoNotFound(format!("no entry '{r}' in workspace '{ws_name}'"))
                })?;
                vec![(entry.name.clone(), entry.source_repo_path.clone())]
            }
            None => {
                let mut seen = HashSet::new();
                ws.entries
                    .iter()
                    .filter_map(|e| {
                        if seen.insert(e.source_repo_path.clone()) {
                            Some((e.name.clone(), e.source_repo_path.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            }
        };
        (targets, Some(ws))
    };

    if targets.is_empty() {
        println!("nothing to sync.");
        return Ok(());
    }

    let mut stream = futures::stream::iter(targets.into_iter().map(|(label, path)| {
        tokio::spawn(async move {
            let result = tokio::process::Command::new("git")
                .arg("-C")
                .arg(&path)
                .args(["fetch", "--all", "--prune"])
                .stdin(Stdio::null())
                .output()
                .await;
            (label, path, result)
        })
    }))
    .buffer_unordered(parallel);

    let mut had_err = false;
    let mut fetched: HashSet<PathBuf> = HashSet::new();
    while let Some(joined) = stream.next().await {
        let (label, path, res) = match joined {
            Ok(v) => v,
            Err(e) => {
                eprintln!("task failed: {e}");
                had_err = true;
                continue;
            }
        };
        match res {
            Ok(out) if out.status.success() => {
                println!("[{label}] fetched in {}", path.display());
                fetched.insert(path);
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
                eprintln!("[{label}] fetch failed: {err}");
                had_err = true;
            }
            Err(e) => {
                eprintln!("[{label}] failed to run git: {e}");
                had_err = true;
            }
        }
    }

    if let Some(ws) = ws {
        for entry in &ws.entries {
            if !fetched.contains(&entry.source_repo_path) {
                continue;
            }
            match repo_setup::load(&entry.path) {
                Ok(Some(setup)) => {
                    repo_setup::copy_files(
                        &entry.source_repo_path,
                        &entry.path,
                        &setup.copy,
                    );
                }
                Ok(None) => {}
                Err(e) => eprintln!("warning: [{}] reading .pow.toml: {e}", entry.name),
            }
        }
    }

    if had_err {
        Err(PowError::GitFailed("one or more fetches failed".into()))
    } else {
        Ok(())
    }
}

pub fn status(name: Option<&str>, dirty_only: bool, short: bool) -> Result<()> {
    let ws_name = resolve_workspace_name(name)?;
    let ws = Workspace::scan(&ws_name)?;

    let mut shown = 0usize;
    for entry in &ws.entries {
        let branch = git::current_branch(&entry.path).unwrap_or_default();
        let modified = show::count_modified(&entry.path).unwrap_or(0);
        let (ahead, behind) = show::tracking_counts(&entry.path);

        if dirty_only && modified == 0 {
            continue;
        }
        shown += 1;

        if short {
            let ab = format_ahead_behind(ahead, behind);
            println!("{:<20}  {:<20}  {:>3}M{ab}", entry.name, branch, modified);
        } else {
            println!(
                "=== {} [{branch}] ({} modified{}){}",
                entry.name,
                modified,
                match (ahead, behind) {
                    (Some(a), Some(b)) => format!(", ahead {a}, behind {b}"),
                    _ => String::new(),
                },
                if ahead.is_none() { ", no upstream" } else { "" },
            );
            if modified > 0 {
                let out = git::git_output(&entry.path, &["status", "--short"]).unwrap_or_default();
                for line in out.lines() {
                    println!("  {line}");
                }
            }
        }
    }
    if shown == 0 {
        println!("no entries to show.");
    }
    Ok(())
}

fn format_ahead_behind(ahead: Option<usize>, behind: Option<usize>) -> String {
    match (ahead, behind) {
        (Some(a), Some(b)) if a > 0 || b > 0 => format!(" ↑{a}↓{b}"),
        _ => String::new(),
    }
}

pub async fn exec(
    command: &str,
    workspace: Option<&str>,
    parallel: Option<usize>,
    dry_run: bool,
) -> Result<()> {
    let cfg = Config::load()?;
    let ws_name = resolve_workspace_name(workspace)?;
    let ws = Workspace::scan(&ws_name)?;
    let par = parallel.unwrap_or(cfg.settings.parallel).max(1);

    if ws.entries.is_empty() {
        println!("workspace '{ws_name}' has no entries.");
        return Ok(());
    }

    if dry_run {
        for entry in &ws.entries {
            println!(
                "[{}] (in {}) $ {}",
                entry.name,
                entry.path.display(),
                command
            );
        }
        return Ok(());
    }

    let entries: Vec<Entry> = ws.entries.clone();
    let cmd = command.to_string();

    let mut stream = futures::stream::iter(entries.into_iter().map(|entry| {
        let cmd = cmd.clone();
        tokio::spawn(async move {
            let result = run_in(&entry.path, &cmd).await;
            (entry, result)
        })
    }))
    .buffer_unordered(par);

    let mut had_err = false;
    let mut collected: Vec<(Entry, ExecOutcome)> = Vec::new();
    while let Some(joined) = stream.next().await {
        match joined {
            Ok(v) => collected.push(v),
            Err(e) => {
                eprintln!("task join error: {e}");
                had_err = true;
            }
        }
    }

    // Sort by repo name for deterministic output.
    collected.sort_by(|a, b| a.0.name.cmp(&b.0.name));
    for (entry, outcome) in collected {
        match outcome {
            ExecOutcome::Ok {
                stdout,
                stderr,
                status,
            } => {
                for line in stdout.lines() {
                    println!("[{}] {line}", entry.name);
                }
                for line in stderr.lines() {
                    eprintln!("[{}] {line}", entry.name);
                }
                if !status {
                    had_err = true;
                }
            }
            ExecOutcome::SpawnError(e) => {
                eprintln!("[{}] failed to run: {e}", entry.name);
                had_err = true;
            }
        }
    }

    if had_err {
        Err(PowError::Message("one or more commands failed".into()))
    } else {
        Ok(())
    }
}

enum ExecOutcome {
    Ok {
        stdout: String,
        stderr: String,
        status: bool,
    },
    SpawnError(String),
}

async fn run_in(cwd: &Path, command: &str) -> ExecOutcome {
    let out = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .await;
    match out {
        Ok(o) => ExecOutcome::Ok {
            stdout: String::from_utf8_lossy(&o.stdout).to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).to_string(),
            status: o.status.success(),
        },
        Err(e) => ExecOutcome::SpawnError(e.to_string()),
    }
}
