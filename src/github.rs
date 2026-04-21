use octocrab::Octocrab;

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct OrgRepo {
    pub name: String,
    pub clone_url_ssh: String,
    #[allow(dead_code)]
    pub clone_url_https: Option<String>,
    pub archived: bool,
    #[allow(dead_code)]
    pub default_branch: Option<String>,
}

pub async fn list_org_repos(
    org: &str,
    token: Option<&str>,
    include_archived: bool,
) -> Result<Vec<OrgRepo>> {
    let mut builder = Octocrab::builder();
    if let Some(t) = token {
        builder = builder.personal_token(t.to_string());
    }
    let client = builder.build()?;

    let mut out = Vec::new();
    let mut page = client
        .orgs(org)
        .list_repos()
        .repo_type(octocrab::params::repos::Type::All)
        .per_page(100)
        .send()
        .await?;
    loop {
        for r in &page.items {
            let archived = r.archived.unwrap_or(false);
            if !include_archived && archived {
                continue;
            }
            let ssh = r
                .ssh_url
                .clone()
                .unwrap_or_else(|| format!("git@github.com:{}/{}.git", org, r.name));
            out.push(OrgRepo {
                name: r.name.clone(),
                clone_url_ssh: ssh,
                clone_url_https: r.clone_url.as_ref().map(|u| u.to_string()),
                archived,
                default_branch: r.default_branch.clone(),
            });
        }
        let Some(next) = client
            .get_page::<octocrab::models::Repository>(&page.next)
            .await?
        else {
            break;
        };
        page = next;
    }

    Ok(out)
}

pub fn resolve_token(config_token: Option<&str>) -> Option<String> {
    config_token
        .map(|s| s.to_string())
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .filter(|s| !s.is_empty())
}
