use glob::Pattern;

use crate::error::{PowError, Result};
use crate::github::OrgRepo;

/// Returns the subset of `repos` passing the configured filters.
///
/// Rules:
/// - If `skip_archived`, drop any archived repo.
/// - If `include` is empty, include everything (subject to excludes).
/// - Otherwise, include only repos matching at least one include pattern.
/// - Any repo matching any `exclude` pattern is dropped.
pub fn apply_filters<'a>(
    repos: &'a [OrgRepo],
    include: &[String],
    exclude: &[String],
    skip_archived: bool,
) -> Result<Vec<&'a OrgRepo>> {
    let inc_pats = compile(include)?;
    let exc_pats = compile(exclude)?;

    let mut out = Vec::new();
    for r in repos {
        if skip_archived && r.archived {
            continue;
        }
        let inc_match = inc_pats.is_empty() || inc_pats.iter().any(|p| p.matches(&r.name));
        if !inc_match {
            continue;
        }
        if exc_pats.iter().any(|p| p.matches(&r.name)) {
            continue;
        }
        out.push(r);
    }
    Ok(out)
}

fn compile(patterns: &[String]) -> Result<Vec<Pattern>> {
    patterns
        .iter()
        .map(|p| Pattern::new(p).map_err(|e| PowError::Config(format!("bad glob '{p}': {e}"))))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo(name: &str, archived: bool) -> OrgRepo {
        OrgRepo {
            name: name.to_string(),
            clone_url_ssh: format!("git@github.com:x/{name}.git"),
            clone_url_https: None,
            archived,
            default_branch: Some("main".into()),
        }
    }

    #[test]
    fn empty_include_passes_all_non_excluded() {
        let repos = vec![
            repo("web", false),
            repo("api", false),
            repo("legacy-ui", false),
        ];
        let got = apply_filters(&repos, &[], &["legacy-*".into()], true).unwrap();
        assert_eq!(
            got.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["web", "api"]
        );
    }

    #[test]
    fn include_glob_matches() {
        let repos = vec![
            repo("api-users", false),
            repo("api-orders", false),
            repo("web", false),
        ];
        let got = apply_filters(&repos, &["api-*".into()], &[], true).unwrap();
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn archived_skipped_by_default() {
        let repos = vec![repo("a", true), repo("b", false)];
        let got = apply_filters(&repos, &[], &[], true).unwrap();
        assert_eq!(
            got.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["b"]
        );
    }

    #[test]
    fn archived_included_when_flag_false() {
        let repos = vec![repo("a", true), repo("b", false)];
        let got = apply_filters(&repos, &[], &[], false).unwrap();
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn include_and_exclude_combined() {
        let repos = vec![repo("api-users", false), repo("api-legacy", false)];
        let got = apply_filters(&repos, &["api-*".into()], &["*-legacy".into()], true).unwrap();
        assert_eq!(
            got.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["api-users"]
        );
    }
}
