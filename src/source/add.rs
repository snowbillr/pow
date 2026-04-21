use crate::config::{Config, Source};
use crate::error::{PowError, Result};
use crate::paths;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    name: &str,
    path: &str,
    github_org: Option<&str>,
    base_branch: &str,
    include: &[String],
    exclude: &[String],
    all: bool,
    skip_archived: bool,
) -> Result<()> {
    let expanded = paths::expand_path(path)?;
    if !expanded.exists() {
        return Err(PowError::Config(format!(
            "path does not exist: {}. Create it first (e.g. `mkdir -p {}`).",
            expanded.display(),
            expanded.display()
        )));
    }
    let canonical = expanded
        .canonicalize()
        .map_err(|e| PowError::Config(format!("canonicalizing {}: {e}", expanded.display())))?;

    let mut cfg = Config::load()?;
    if cfg.find_source(name).is_some() {
        return Err(PowError::Config(format!("source '{name}' already exists")));
    }

    let stored_path = canonical.to_string_lossy().to_string();

    if let Some(org) = github_org {
        crate::source::add_github::run_with_github(
            &mut cfg,
            name,
            &stored_path,
            org,
            base_branch,
            include,
            exclude,
            all,
            skip_archived,
        )
        .await?;
    } else {
        let source = Source {
            name: name.to_string(),
            path: stored_path,
            github_org: None,
            base_branch: base_branch.to_string(),
            skip_archived,
            include: include.to_vec(),
            exclude: exclude.to_vec(),
        };
        cfg.add_source(source)?;
        cfg.save()?;
        println!("Registered source '{name}' at {}.", canonical.display());
    }
    Ok(())
}
