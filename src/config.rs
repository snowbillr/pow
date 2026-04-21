use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{PowError, Result};
use crate::paths;

fn default_parallel() -> usize {
    4
}

fn default_base_branch() -> String {
    "main".to_string()
}

fn default_skip_archived() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_source: Option<String>,
    #[serde(default = "default_parallel")]
    pub parallel: usize,
}

impl Settings {
    fn defaulted() -> Self {
        Self {
            default_source: None,
            parallel: default_parallel(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GithubConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_org: Option<String>,
    #[serde(default = "default_base_branch")]
    pub base_branch: String,
    #[serde(default = "default_skip_archived")]
    pub skip_archived: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

impl Source {
    /// Expand `~` / env vars in `path`.
    pub fn expanded_path(&self) -> Result<PathBuf> {
        paths::expand_path(&self.path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub github: GithubConfig,
    #[serde(default, rename = "sources", skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings::defaulted(),
            github: GithubConfig::default(),
            sources: Vec::new(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = paths::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)
            .map_err(|e| PowError::Config(format!("reading {}: {e}", path.display())))?;
        let mut cfg: Config = toml::from_str(&text)?;
        if cfg.settings.parallel == 0 {
            cfg.settings.parallel = default_parallel();
        }
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = paths::config_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        let tmp = path.with_extension("toml.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &path)?;
        Ok(())
    }

    pub fn find_source(&self, name: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.name == name)
    }

    pub fn find_source_mut(&mut self, name: &str) -> Option<&mut Source> {
        self.sources.iter_mut().find(|s| s.name == name)
    }

    pub fn add_source(&mut self, source: Source) -> Result<()> {
        if self.find_source(&source.name).is_some() {
            return Err(PowError::Config(format!(
                "source '{}' already exists",
                source.name
            )));
        }
        self.sources.push(source);
        Ok(())
    }

    pub fn remove_source(&mut self, name: &str) -> Result<Source> {
        let idx = self
            .sources
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| PowError::SourceNotFound(name.to_string()))?;
        Ok(self.sources.remove(idx))
    }
}

// --------------------------------------------------------------------------
// pow config / config get / config set
// --------------------------------------------------------------------------

pub fn cmd_print(json: bool) -> Result<()> {
    let cfg = Config::load()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&cfg).map_err(|e| PowError::Config(e.to_string()))?);
    } else {
        print!("{}", toml::to_string_pretty(&cfg)?);
    }
    Ok(())
}

pub fn cmd_get(key: &str) -> Result<()> {
    let cfg = Config::load()?;
    let value = get_value(&cfg, key)?;
    println!("{value}");
    Ok(())
}

pub fn cmd_set(key: &str, value: &str) -> Result<()> {
    let mut cfg = Config::load()?;
    set_value(&mut cfg, key, value)?;
    cfg.save()?;
    Ok(())
}

fn get_value(cfg: &Config, key: &str) -> Result<String> {
    match key {
        "settings.default_source" => Ok(cfg.settings.default_source.clone().unwrap_or_default()),
        "settings.parallel" => Ok(cfg.settings.parallel.to_string()),
        "github.token" => Ok(cfg.github.token.clone().unwrap_or_default()),
        _ => Err(PowError::Config(format!(
            "unknown config key '{key}'. Valid keys: settings.default_source, settings.parallel, github.token"
        ))),
    }
}

fn set_value(cfg: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
        "settings.default_source" => {
            if value.is_empty() {
                cfg.settings.default_source = None;
            } else {
                cfg.settings.default_source = Some(value.to_string());
            }
        }
        "settings.parallel" => {
            let n: usize = value.parse().map_err(|_| {
                PowError::Config(format!("settings.parallel must be a positive integer, got '{value}'"))
            })?;
            if n == 0 {
                return Err(PowError::Config("settings.parallel must be >= 1".into()));
            }
            cfg.settings.parallel = n;
        }
        "github.token" => {
            if value.is_empty() {
                cfg.github.token = None;
            } else {
                cfg.github.token = Some(value.to_string());
            }
        }
        _ => {
            return Err(PowError::Config(format!(
                "unknown config key '{key}'. Valid keys: settings.default_source, settings.parallel, github.token"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrips_empty() {
        let cfg = Config::default();
        let text = toml::to_string(&cfg).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.settings.parallel, 4);
        assert!(back.sources.is_empty());
    }

    #[test]
    fn roundtrips_with_source() {
        let mut cfg = Config::default();
        cfg.sources.push(Source {
            name: "babylist".into(),
            path: "~/src/Babylist".into(),
            github_org: Some("babylist".into()),
            base_branch: "main".into(),
            skip_archived: true,
            include: vec!["web".into(), "api-*".into()],
            exclude: vec!["legacy-*".into()],
        });
        let text = toml::to_string_pretty(&cfg).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.sources.len(), 1);
        assert_eq!(back.sources[0].name, "babylist");
        assert_eq!(back.sources[0].include, vec!["web".to_string(), "api-*".into()]);
    }

    #[test]
    fn get_and_set_values() {
        let mut cfg = Config::default();
        set_value(&mut cfg, "settings.parallel", "8").unwrap();
        assert_eq!(cfg.settings.parallel, 8);
        assert_eq!(get_value(&cfg, "settings.parallel").unwrap(), "8");

        set_value(&mut cfg, "settings.default_source", "babylist").unwrap();
        assert_eq!(get_value(&cfg, "settings.default_source").unwrap(), "babylist");

        set_value(&mut cfg, "github.token", "ghp_test").unwrap();
        assert_eq!(get_value(&cfg, "github.token").unwrap(), "ghp_test");

        assert!(set_value(&mut cfg, "bogus.key", "x").is_err());
        assert!(set_value(&mut cfg, "settings.parallel", "0").is_err());
        assert!(set_value(&mut cfg, "settings.parallel", "abc").is_err());
    }
}
