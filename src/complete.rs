//! Print completion candidates for the `pow __complete` hidden helper.
//!
//! Each routine writes newline-separated names to stdout and never returns an
//! error: shell completion must stay quiet, so any failure (missing config,
//! unresolved workspace, unreadable directory) just produces no candidates.

use std::collections::BTreeSet;

use crate::cli::CompleteKind;
use crate::config::Config;
use crate::git;
use crate::workspace::{self, Workspace};

pub const CONFIG_KEYS: &[&str] = &[
    "settings.default_source",
    "settings.parallel",
    "github.token",
];

pub fn run(kind: CompleteKind) {
    match kind {
        CompleteKind::Workspaces => print_workspaces(),
        CompleteKind::Entries { workspace } => print_entries(workspace.as_deref()),
        CompleteKind::Repos { source } => print_repos(source.as_deref()),
        CompleteKind::Sources => print_sources(),
        CompleteKind::ConfigKeys => print_config_keys(),
    }
}

fn print_workspaces() {
    let cfg = Config::load().unwrap_or_default();
    if let Ok(list) = Workspace::list_all(&cfg) {
        for ws in list {
            println!("{}", ws.name);
        }
    }
}

fn print_entries(workspace_arg: Option<&str>) {
    let Ok(name) = workspace::resolve_workspace_name(workspace_arg) else {
        return;
    };
    if let Ok(ws) = Workspace::scan(&name) {
        for entry in ws.entries {
            println!("{}", entry.name);
        }
    }
}

fn print_repos(source_filter: Option<&str>) {
    let Ok(cfg) = Config::load() else {
        return;
    };
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for source in &cfg.sources {
        if let Some(want) = source_filter {
            if source.name != want {
                continue;
            }
        }
        let Ok(path) = source.expanded_path() else {
            continue;
        };
        let Ok(repos) = git::list_repos_in(&path) else {
            continue;
        };
        for repo in repos {
            if let Some(name) = repo.file_name().and_then(|n| n.to_str()) {
                seen.insert(name.to_string());
            }
        }
    }
    for name in seen {
        println!("{name}");
    }
}

fn print_sources() {
    let Ok(cfg) = Config::load() else {
        return;
    };
    for source in &cfg.sources {
        println!("{}", source.name);
    }
}

fn print_config_keys() {
    for key in CONFIG_KEYS {
        println!("{key}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_keys_match_config_module() {
        assert_eq!(
            CONFIG_KEYS,
            &[
                "settings.default_source",
                "settings.parallel",
                "github.token",
            ]
        );
    }
}
