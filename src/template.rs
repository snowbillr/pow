use std::io::{self, Write};

use crate::config::Config;
use crate::error::{PowError, Result};

pub fn list(json: bool) -> Result<()> {
    let cfg = Config::load()?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&cfg.templates)
                .map_err(|e| PowError::Config(e.to_string()))?
        );
        return Ok(());
    }

    if cfg.templates.is_empty() {
        println!(
            "no templates configured. Add a [[templates]] block to your config (see `pow config`)."
        );
        return Ok(());
    }

    struct Row {
        name: String,
        count: String,
        repos: String,
    }
    let rows: Vec<Row> = cfg
        .templates
        .iter()
        .map(|t| Row {
            name: t.name.clone(),
            count: t.repos.len().to_string(),
            repos: t.repos.join(", "),
        })
        .collect();

    let header = Row {
        name: "NAME".into(),
        count: "REPOS".into(),
        repos: "ENTRIES".into(),
    };
    let name_w = rows
        .iter()
        .chain([&header])
        .map(|r| r.name.len())
        .max()
        .unwrap();
    let count_w = rows
        .iter()
        .chain([&header])
        .map(|r| r.count.len())
        .max()
        .unwrap();

    let stdout = io::stdout();
    let mut out = stdout.lock();
    writeln!(
        out,
        "{:<nw$}  {:>cw$}  {}",
        header.name,
        header.count,
        header.repos,
        nw = name_w,
        cw = count_w,
    )?;
    for r in &rows {
        writeln!(
            out,
            "{:<nw$}  {:>cw$}  {}",
            r.name,
            r.count,
            r.repos,
            nw = name_w,
            cw = count_w,
        )?;
    }
    Ok(())
}
