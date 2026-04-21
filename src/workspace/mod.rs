pub mod lifecycle;
pub mod nav;
pub mod show;
pub mod work;

use std::path::PathBuf;

use serde::Serialize;

use crate::config::Config;
use crate::error::{PowError, Result};
use crate::git;
use crate::paths;

#[derive(Debug, Clone, Serialize)]
pub struct Workspace {
    pub name: String,
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub source_name: Option<String>,
    pub source_repo_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceSummary {
    pub name: String,
    pub path: PathBuf,
    pub entry_count: usize,
    pub active: bool,
}

impl Workspace {
    pub fn scan(name: &str) -> Result<Self> {
        let path = paths::workspace_path(name)?;
        if !path.exists() {
            return Err(PowError::WorkspaceNotFound(name.to_string()));
        }
        let cfg = Config::load().unwrap_or_default();
        let mut entries = Vec::new();
        for ent in std::fs::read_dir(&path)? {
            let ent = ent?;
            let p = ent.path();
            if !p.is_dir() {
                continue;
            }
            if !git::is_git_repo(&p) {
                continue;
            }
            let repo_name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let src_repo = git::worktree_source_repo(&p).unwrap_or_else(|_| p.clone());
            // match source by parent dir
            let src_name = src_repo.parent().and_then(|parent| {
                cfg.sources
                    .iter()
                    .find(|s| {
                        s.expanded_path()
                            .map(|ep| {
                                ep == parent || ep.canonicalize().ok() == parent.canonicalize().ok()
                            })
                            .unwrap_or(false)
                    })
                    .map(|s| s.name.clone())
            });
            entries.push(Entry {
                name: repo_name,
                path: p,
                source_name: src_name,
                source_repo_path: src_repo,
            });
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Workspace {
            name: name.to_string(),
            path,
            entries,
        })
    }

    pub fn list_all(_cfg: &Config) -> Result<Vec<WorkspaceSummary>> {
        let root = paths::workspaces_root()?;
        let active = active_workspace();
        if !root.exists() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for ent in std::fs::read_dir(&root)? {
            let ent = ent?;
            let p = ent.path();
            if !p.is_dir() {
                continue;
            }
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.is_empty() || name.starts_with('.') {
                continue;
            }
            let entry_count = match std::fs::read_dir(&p) {
                Ok(rd) => rd
                    .filter_map(|r| r.ok())
                    .filter(|e| e.path().is_dir() && git::is_git_repo(&e.path()))
                    .count(),
                Err(_) => 0,
            };
            out.push(WorkspaceSummary {
                name: name.clone(),
                path: p,
                entry_count,
                active: active.as_deref() == Some(name.as_str()),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

/// Name of the active workspace per `$POW_ACTIVE`, if any.
pub fn active_workspace() -> Option<String> {
    std::env::var("POW_ACTIVE").ok().filter(|s| !s.is_empty())
}

/// Resolve a workspace name: explicit > active env > error.
pub fn resolve_workspace_name(arg: Option<&str>) -> Result<String> {
    if let Some(name) = arg {
        return Ok(name.to_string());
    }
    active_workspace().ok_or_else(|| {
        PowError::Message(
            "no workspace specified and $POW_ACTIVE is unset. Pass -w <name> or run `pow use <name>`."
                .to_string(),
        )
    })
}
