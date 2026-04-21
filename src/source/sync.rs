use std::collections::HashSet;
use std::io::{self, Write};

use crate::config::Config;
use crate::error::{PowError, Result};
use crate::git;
use crate::github;
use crate::source::add_github;
use crate::source::filter;

pub async fn run(name: &str, dry_run: bool, prune: bool, parallel: Option<usize>) -> Result<()> {
    let cfg = Config::load()?;
    let source = cfg
        .find_source(name)
        .ok_or_else(|| PowError::SourceNotFound(name.to_string()))?
        .clone();
    let org = source.github_org.as_deref().ok_or_else(|| {
        PowError::Message(format!(
            "source '{name}' has no github_org; nothing to sync from."
        ))
    })?;
    let path = source.expanded_path()?;
    std::fs::create_dir_all(&path)?;

    let token = github::resolve_token(cfg.github.token.as_deref());

    eprintln!("Fetching repos from GitHub org '{org}'...");
    let all_repos = github::list_org_repos(org, token.as_deref(), !source.skip_archived).await?;
    let filtered = filter::apply_filters(
        &all_repos,
        &source.include,
        &source.exclude,
        source.skip_archived,
    )?;
    let target_names: HashSet<String> = filtered.iter().map(|r| r.name.clone()).collect();

    let local_repos = git::list_repos_in(&path)?;
    let local_names: HashSet<String> = local_repos
        .iter()
        .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .collect();

    let to_clone: Vec<&github::OrgRepo> = filtered
        .iter()
        .copied()
        .filter(|r| !local_names.contains(&r.name))
        .collect();

    let to_prune: Vec<String> = local_names
        .iter()
        .filter(|n| !target_names.contains(n.as_str()))
        .cloned()
        .collect();

    if dry_run {
        println!("Would clone:");
        for r in &to_clone {
            println!("  + {}", r.name);
        }
        if prune {
            println!("Would remove:");
            for n in &to_prune {
                println!("  - {n}");
            }
        }
        return Ok(());
    }

    let par = parallel.unwrap_or(cfg.settings.parallel);

    if !to_clone.is_empty() {
        add_github::clone_all(&to_clone, &path, par).await?;
    } else {
        println!("No new repos to clone.");
    }

    if prune && !to_prune.is_empty() {
        for n in &to_prune {
            eprint!(
                "Remove {}/{} (no longer in org set)? [y/N] ",
                path.display(),
                n
            );
            io::stderr().flush()?;
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            let ans = buf.trim().to_lowercase();
            if ans == "y" || ans == "yes" {
                let dir = path.join(n);
                std::fs::remove_dir_all(&dir)?;
                println!("Removed {}.", dir.display());
            } else {
                eprintln!("skipped {n}.");
            }
        }
    }

    Ok(())
}
