use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::error::{PowError, Result};

/// Return true if `path` is a git repo (has `.git` as dir OR file).
pub fn is_git_repo(path: &Path) -> bool {
    let git = path.join(".git");
    git.is_dir() || git.is_file()
}

/// List immediate subdirectories of `source_path` that are git repos.
pub fn list_repos_in(source_path: &Path) -> Result<Vec<PathBuf>> {
    if !source_path.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(source_path)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if is_git_repo(&path) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Run a `git` command with `-C path` and collect stdout (trimmed).
pub fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| PowError::GitFailed(format!("failed to run git: {e}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(PowError::GitFailed(format!(
            "git {} (in {}): {stderr}",
            args.join(" "),
            repo.display()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a `git` command with `-C path`, return stdout+stderr+status without error mapping.
pub fn git_raw(repo: &Path, args: &[&str]) -> Result<std::process::Output> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| PowError::GitFailed(format!("failed to run git: {e}")))
}

/// Current branch name (empty string if detached HEAD).
pub fn current_branch(repo: &Path) -> Result<String> {
    let out = git_output(repo, &["symbolic-ref", "--quiet", "--short", "HEAD"]);
    match out {
        Ok(s) => Ok(s),
        Err(_) => Ok(String::new()), // detached
    }
}

pub fn branch_exists(source_repo: &Path, branch: &str) -> Result<bool> {
    let out = git_raw(
        source_repo,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ],
    )?;
    Ok(out.status.success())
}

pub fn branch_delete(source_repo: &Path, branch: &str, force: bool) -> Result<()> {
    let flag = if force { "-D" } else { "-d" };
    let out = git_raw(source_repo, &["branch", flag, branch])?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(PowError::GitFailed(stderr));
    }
    Ok(())
}

pub fn worktree_prune(source_repo: &Path) -> Result<()> {
    let _ = git_raw(source_repo, &["worktree", "prune"])?;
    Ok(())
}

/// `git worktree add -b <branch> <dest> [<base>]` in `source_repo`.
pub fn worktree_add(
    source_repo: &Path,
    dest: &Path,
    branch: &str,
    base: Option<&str>,
) -> Result<()> {
    let dest_s = dest.to_string_lossy().to_string();
    let mut args: Vec<&str> = vec!["worktree", "add", "-b", branch, &dest_s];
    if let Some(b) = base {
        args.push(b);
    }
    let out = git_raw(source_repo, &args)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(PowError::GitFailed(stderr));
    }
    Ok(())
}

/// `git worktree add <dest> <branch>` — branch must already exist locally.
pub fn worktree_add_existing(source_repo: &Path, dest: &Path, branch: &str) -> Result<()> {
    let dest_s = dest.to_string_lossy().to_string();
    let out = git_raw(source_repo, &["worktree", "add", &dest_s, branch])?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(PowError::GitFailed(stderr));
    }
    Ok(())
}

pub fn worktree_remove(source_repo: &Path, dest: &Path, force: bool) -> Result<()> {
    let dest_s = dest.to_string_lossy().to_string();
    let mut args: Vec<&str> = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&dest_s);
    let out = git_raw(source_repo, &args)?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(PowError::GitFailed(stderr));
    }
    Ok(())
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: Option<String>,
}

#[allow(dead_code)]
pub fn worktree_list(source_repo: &Path) -> Result<Vec<Worktree>> {
    let out = git_output(source_repo, &["worktree", "list", "--porcelain"])?;
    let mut worktrees = Vec::new();
    let mut cur: Option<Worktree> = None;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            if let Some(prev) = cur.take() {
                worktrees.push(prev);
            }
            cur = Some(Worktree {
                path: PathBuf::from(rest),
                branch: None,
                head: None,
            });
        } else if let Some(rest) = line.strip_prefix("HEAD ") {
            if let Some(w) = cur.as_mut() {
                w.head = Some(rest.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("branch ") {
            if let Some(w) = cur.as_mut() {
                // 'branch refs/heads/foo' -> 'foo'
                let b = rest.strip_prefix("refs/heads/").unwrap_or(rest);
                w.branch = Some(b.to_string());
            }
        }
    }
    if let Some(prev) = cur {
        worktrees.push(prev);
    }
    Ok(worktrees)
}

/// Parse `.git` file inside a worktree to extract the source repo path.
///
/// The file looks like: `gitdir: /path/to/source/.git/worktrees/<name>`
/// We return `/path/to/source` (the directory containing `.git`).
pub fn worktree_source_repo(worktree_dir: &Path) -> Result<PathBuf> {
    let git_file = worktree_dir.join(".git");
    if git_file.is_dir() {
        // This is a regular repo, not a worktree.
        return Ok(worktree_dir.to_path_buf());
    }
    let contents = std::fs::read_to_string(&git_file)?;
    let line = contents.trim();
    let gitdir = line
        .strip_prefix("gitdir:")
        .map(|s| s.trim())
        .ok_or_else(|| {
            PowError::GitFailed(format!(
                "{}: unexpected .git file contents",
                git_file.display()
            ))
        })?;
    let gitdir_path = PathBuf::from(gitdir);
    // Expected: .../<source>/.git/worktrees/<name>
    // Walk up: worktrees -> .git -> <source>
    let wt_name_parent = gitdir_path
        .parent()
        .ok_or_else(|| PowError::GitFailed(format!("unexpected gitdir path: {gitdir}")))?;
    let dot_git = wt_name_parent
        .parent()
        .ok_or_else(|| PowError::GitFailed(format!("unexpected gitdir path: {gitdir}")))?;
    let source = dot_git
        .parent()
        .ok_or_else(|| PowError::GitFailed(format!("unexpected gitdir path: {gitdir}")))?;
    Ok(source.to_path_buf())
}
