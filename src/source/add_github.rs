use std::path::{Path, PathBuf};
use std::process::Stdio;

use dialoguer::MultiSelect;
use futures::stream::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::config::{Config, Source};
use crate::error::{PowError, Result};
use crate::github::{self, OrgRepo};
use crate::source::filter;

#[allow(clippy::too_many_arguments)]
pub async fn run_with_github(
    cfg: &mut Config,
    name: &str,
    stored_path: &str,
    org: &str,
    base_branch: &str,
    include_flag: &[String],
    exclude_flag: &[String],
    all: bool,
    skip_archived: bool,
) -> Result<()> {
    let path_buf = PathBuf::from(stored_path);
    let token = github::resolve_token(cfg.github.token.as_deref());

    eprintln!("Fetching repos from GitHub org '{org}'...");
    let all_repos = github::list_org_repos(org, token.as_deref(), !skip_archived).await?;
    if all_repos.is_empty() {
        return Err(PowError::Message(format!(
            "no repos visible in org '{org}'. Check GITHUB_TOKEN or org name."
        )));
    }

    // Resolve filters. If neither --all, --include nor --exclude was passed, run the picker.
    let (include, exclude): (Vec<String>, Vec<String>) =
        if all || !include_flag.is_empty() || !exclude_flag.is_empty() {
            (include_flag.to_vec(), exclude_flag.to_vec())
        } else {
            let selected = pick_repos(&all_repos, skip_archived)?;
            (selected, Vec::new())
        };

    // Save source immediately so it's persisted even if cloning partially fails.
    let source = Source {
        name: name.to_string(),
        path: stored_path.to_string(),
        github_org: Some(org.to_string()),
        base_branch: base_branch.to_string(),
        skip_archived,
        include: include.clone(),
        exclude: exclude.clone(),
    };
    cfg.add_source(source)?;
    cfg.save()?;

    let targets = filter::apply_filters(&all_repos, &include, &exclude, skip_archived)?;
    if targets.is_empty() {
        eprintln!("No repos match the configured filters. Source registered with no clones.");
        return Ok(());
    }

    std::fs::create_dir_all(&path_buf)?;

    clone_all(&targets, &path_buf, cfg.settings.parallel).await?;
    println!(
        "Registered source '{name}' at {} ({} repos cloned).",
        path_buf.display(),
        targets.len()
    );
    Ok(())
}

pub(crate) fn pick_repos(repos: &[OrgRepo], skip_archived: bool) -> Result<Vec<String>> {
    let visible: Vec<&OrgRepo> = repos
        .iter()
        .filter(|r| !(skip_archived && r.archived))
        .collect();

    if visible.is_empty() {
        return Ok(Vec::new());
    }

    let items: Vec<String> = visible
        .iter()
        .map(|r| {
            if r.archived {
                format!("{} (archived)", r.name)
            } else {
                r.name.clone()
            }
        })
        .collect();

    let selections = MultiSelect::new()
        .with_prompt("Select repos to clone (space to toggle, enter to confirm)")
        .items(&items)
        .interact()
        .map_err(|e| PowError::Message(format!("picker failed: {e}")))?;

    Ok(selections
        .into_iter()
        .map(|i| visible[i].name.clone())
        .collect())
}

pub(crate) async fn clone_all(repos: &[&OrgRepo], dest_dir: &Path, parallel: usize) -> Result<()> {
    let mp = MultiProgress::new();
    let style = ProgressStyle::with_template("{spinner} {prefix:20} {wide_msg}")
        .unwrap_or_else(|_| ProgressStyle::default_spinner());

    let owned: Vec<(String, String, PathBuf)> = repos
        .iter()
        .map(|r| {
            let target = dest_dir.join(&r.name);
            (r.name.clone(), r.clone_url_ssh.clone(), target)
        })
        .collect();

    let mut stream = futures::stream::iter(owned.into_iter().map(|(name, url, target)| {
        let pb = mp.add(ProgressBar::new_spinner());
        pb.set_style(style.clone());
        pb.set_prefix(name.clone());
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb.set_message("cloning...");
        async move {
            if target.exists() {
                pb.finish_with_message("already present; skipped");
                return (name, Ok::<(), String>(()));
            }
            let out = tokio::process::Command::new("git")
                .args(["clone", "--", &url])
                .arg(&target)
                .stdin(Stdio::null())
                .output()
                .await;
            match out {
                Ok(o) if o.status.success() => {
                    pb.finish_with_message("done");
                    (name, Ok(()))
                }
                Ok(o) => {
                    let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
                    pb.finish_with_message(format!("failed: {err}"));
                    (name, Err(err))
                }
                Err(e) => {
                    pb.finish_with_message(format!("failed: {e}"));
                    (name, Err(e.to_string()))
                }
            }
        }
    }))
    .buffer_unordered(parallel.max(1));

    let mut errors = Vec::new();
    while let Some((name, result)) = stream.next().await {
        if let Err(e) = result {
            errors.push(format!("{name}: {e}"));
        }
    }

    if !errors.is_empty() {
        return Err(PowError::Message(format!(
            "one or more clones failed:\n  {}",
            errors.join("\n  ")
        )));
    }
    Ok(())
}
