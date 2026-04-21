pub mod add;
pub mod add_github;
pub mod filter;
pub mod sync;

use std::io::{self, Write};

use crate::config::Config;
use crate::error::{PowError, Result};
use crate::git;

pub fn list(json: bool) -> Result<()> {
    let cfg = Config::load()?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&cfg.sources)
                .map_err(|e| PowError::Config(e.to_string()))?
        );
        return Ok(());
    }

    if cfg.sources.is_empty() {
        println!("no sources configured. Use `pow source add <name> <path>`.");
        return Ok(());
    }

    // Collect rows first so we can compute column widths.
    struct Row {
        name: String,
        path: String,
        count: String,
        org: String,
        base: String,
    }
    let mut rows = Vec::with_capacity(cfg.sources.len());
    for src in &cfg.sources {
        let path = src.expanded_path()?;
        let count = git::list_repos_in(&path).map(|v| v.len()).unwrap_or(0);
        rows.push(Row {
            name: src.name.clone(),
            path: path.display().to_string(),
            count: count.to_string(),
            org: src.github_org.clone().unwrap_or_else(|| "—".to_string()),
            base: src.base_branch.clone(),
        });
    }

    let header = Row {
        name: "NAME".into(),
        path: "PATH".into(),
        count: "REPOS".into(),
        org: "GITHUB ORG".into(),
        base: "BASE".into(),
    };
    let name_w = rows.iter().chain([&header]).map(|r| r.name.len()).max().unwrap();
    let path_w = rows.iter().chain([&header]).map(|r| r.path.len()).max().unwrap();
    let count_w = rows.iter().chain([&header]).map(|r| r.count.len()).max().unwrap();
    let org_w = rows.iter().chain([&header]).map(|r| r.org.len()).max().unwrap();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(
        out,
        "{:<nw$}  {:<pw$}  {:>cw$}  {:<ow$}  {}",
        header.name,
        header.path,
        header.count,
        header.org,
        header.base,
        nw = name_w,
        pw = path_w,
        cw = count_w,
        ow = org_w,
    )?;
    for r in &rows {
        writeln!(
            out,
            "{:<nw$}  {:<pw$}  {:>cw$}  {:<ow$}  {}",
            r.name,
            r.path,
            r.count,
            r.org,
            r.base,
            nw = name_w,
            pw = path_w,
            cw = count_w,
            ow = org_w,
        )?;
    }
    Ok(())
}

pub fn remove(name: &str, force: bool) -> Result<()> {
    let mut cfg = Config::load()?;
    if cfg.find_source(name).is_none() {
        return Err(PowError::SourceNotFound(name.to_string()));
    }
    if !force {
        eprint!("Unregister source '{name}'? [y/N] ");
        io::stderr().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        let ans = buf.trim().to_lowercase();
        if ans != "y" && ans != "yes" {
            eprintln!("aborted.");
            return Ok(());
        }
    }
    let src = cfg.remove_source(name)?;
    cfg.save()?;
    println!(
        "Source '{name}' unregistered. Files at {} not touched.",
        src.path
    );
    Ok(())
}
