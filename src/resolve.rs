use std::path::PathBuf;

use crate::config::{Config, Source};
use crate::error::{PowError, Result};
use crate::git;

#[derive(Debug, Clone)]
pub struct ResolvedRepo {
    pub source_name: String,
    #[allow(dead_code)]
    pub source_path: PathBuf,
    pub repo_path: PathBuf,
    pub repo_name: String,
}

/// Given a user-supplied repo spec, locate it across the configured sources.
///
/// - `source/name` → exactly that source + repo.
/// - Bare `name`   → look across all sources; error on ambiguity.
pub fn resolve_repo(config: &Config, spec: &str) -> Result<ResolvedRepo> {
    if let Some((src_name, repo_name)) = spec.split_once('/') {
        let src = config
            .find_source(src_name)
            .ok_or_else(|| PowError::SourceNotFound(src_name.to_string()))?;
        return resolve_in_source(src, repo_name);
    }

    // Bare name: search all sources.
    let mut matches: Vec<ResolvedRepo> = Vec::new();
    for src in &config.sources {
        if let Ok(r) = resolve_in_source(src, spec) {
            matches.push(r);
        }
    }

    match matches.len() {
        0 => Err(PowError::RepoNotFound(format!(
            "repo '{spec}' not found in any configured source"
        ))),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => {
            let alts = matches
                .iter()
                .map(|r| format!("{}/{}", r.source_name, r.repo_name))
                .collect::<Vec<_>>()
                .join(", ");
            Err(PowError::RepoNotFound(format!(
                "repo '{spec}' is ambiguous; found in: {alts}. Disambiguate with <source>/<repo>."
            )))
        }
    }
}

fn resolve_in_source(src: &Source, repo_name: &str) -> Result<ResolvedRepo> {
    let source_path = src.expanded_path()?;
    let repo_path = source_path.join(repo_name);
    if !repo_path.is_dir() || !git::is_git_repo(&repo_path) {
        return Err(PowError::RepoNotFound(format!(
            "repo '{repo_name}' not found in source '{}'",
            src.name
        )));
    }
    Ok(ResolvedRepo {
        source_name: src.name.clone(),
        source_path,
        repo_path,
        repo_name: repo_name.to_string(),
    })
}
