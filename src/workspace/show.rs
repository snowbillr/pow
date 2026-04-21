use std::io::{self, Write};

use serde::Serialize;

use crate::config::Config;
use crate::error::{PowError, Result};
use crate::git;
use crate::workspace::{resolve_workspace_name, Workspace};

pub fn list(json: bool) -> Result<()> {
    let cfg = Config::load().unwrap_or_default();
    let workspaces = Workspace::list_all(&cfg)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&workspaces)
                .map_err(|e| PowError::Config(e.to_string()))?
        );
        return Ok(());
    }
    if workspaces.is_empty() {
        println!("no workspaces. Create one with `pow new <name>`.");
        return Ok(());
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let name_w = workspaces
        .iter()
        .map(|w| w.name.len())
        .max()
        .unwrap_or(0)
        .max(4);
    writeln!(out, "{:<nw$}  {:>5}  PATH", "NAME", "N", nw = name_w)?;
    for w in &workspaces {
        let marker = if w.active { "*" } else { " " };
        writeln!(
            out,
            "{marker}{:<nw$}  {:>5}  {}",
            w.name,
            w.entry_count,
            w.path.display(),
            nw = name_w - 1
        )?;
    }
    Ok(())
}

#[derive(Serialize)]
struct EntryStatus {
    name: String,
    path: String,
    source: Option<String>,
    branch: String,
    modified: Option<usize>,
    ahead: Option<usize>,
    behind: Option<usize>,
}

pub fn show(name: Option<&str>, json: bool, no_status: bool) -> Result<()> {
    let ws_name = resolve_workspace_name(name)?;
    let ws = Workspace::scan(&ws_name)?;

    let mut rows = Vec::with_capacity(ws.entries.len());
    for entry in &ws.entries {
        let branch = git::current_branch(&entry.path).unwrap_or_default();
        let (modified, ahead, behind) = if no_status {
            (None, None, None)
        } else {
            let m = count_modified(&entry.path).ok();
            let (a, b) = tracking_counts(&entry.path);
            (m, a, b)
        };
        rows.push(EntryStatus {
            name: entry.name.clone(),
            path: entry.path.display().to_string(),
            source: entry.source_name.clone(),
            branch,
            modified,
            ahead,
            behind,
        });
    }

    if json {
        #[derive(Serialize)]
        struct Out<'a> {
            name: &'a str,
            path: String,
            entries: Vec<EntryStatus>,
        }
        let out = Out {
            name: &ws.name,
            path: ws.path.display().to_string(),
            entries: rows,
        };
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| PowError::Config(e.to_string()))?
        );
        return Ok(());
    }

    println!("workspace: {} ({})", ws.name, ws.path.display());
    if rows.is_empty() {
        println!("  (no entries)");
        return Ok(());
    }
    let name_w = rows.iter().map(|r| r.name.len()).max().unwrap_or(0).max(4);
    let branch_w = rows
        .iter()
        .map(|r| r.branch.len())
        .max()
        .unwrap_or(0)
        .max(6);
    let src_w = rows
        .iter()
        .map(|r| r.source.as_deref().unwrap_or("-").len())
        .max()
        .unwrap_or(0)
        .max(6);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(
        out,
        "  {:<nw$}  {:<sw$}  {:<bw$}  STATUS",
        "REPO",
        "SOURCE",
        "BRANCH",
        nw = name_w,
        sw = src_w,
        bw = branch_w,
    )?;
    for r in &rows {
        let status = if no_status {
            String::from("—")
        } else {
            let mut parts = Vec::new();
            if let Some(m) = r.modified {
                if m > 0 {
                    parts.push(format!("*{m}"));
                } else {
                    parts.push("clean".into());
                }
            }
            if let Some(a) = r.ahead {
                if a > 0 {
                    parts.push(format!("↑{a}"));
                }
            }
            if let Some(b) = r.behind {
                if b > 0 {
                    parts.push(format!("↓{b}"));
                }
            }
            parts.join(" ")
        };
        writeln!(
            out,
            "  {:<nw$}  {:<sw$}  {:<bw$}  {}",
            r.name,
            r.source.clone().unwrap_or_else(|| "-".to_string()),
            r.branch,
            status,
            nw = name_w,
            sw = src_w,
            bw = branch_w,
        )?;
    }
    Ok(())
}

pub(crate) fn count_modified(repo: &std::path::Path) -> Result<usize> {
    let out = git::git_output(repo, &["status", "--porcelain"])?;
    Ok(out.lines().filter(|l| !l.trim().is_empty()).count())
}

/// (ahead, behind) relative to upstream; (None, None) if no upstream.
pub(crate) fn tracking_counts(repo: &std::path::Path) -> (Option<usize>, Option<usize>) {
    let raw = match git::git_raw(
        repo,
        &["rev-list", "--left-right", "--count", "HEAD...@{upstream}"],
    ) {
        Ok(o) if o.status.success() => o,
        _ => return (None, None),
    };
    let s = String::from_utf8_lossy(&raw.stdout).trim().to_string();
    let mut it = s.split_whitespace();
    let ahead = it.next().and_then(|x| x.parse().ok());
    let behind = it.next().and_then(|x| x.parse().ok());
    (ahead, behind)
}
