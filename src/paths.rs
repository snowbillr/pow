use std::path::PathBuf;

use crate::error::{PowError, Result};

/// Root config directory. Honors `POW_CONFIG` (which is a file path) and otherwise
/// uses `$XDG_CONFIG_HOME/pow` (falling back to `$HOME/.config/pow`).
pub fn config_dir() -> Result<PathBuf> {
    if let Some(path) = config_path_override() {
        if let Some(parent) = path.parent() {
            return Ok(parent.to_path_buf());
        }
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Ok(PathBuf::from(xdg).join("pow"));
        }
    }
    let home = home_dir()?;
    Ok(home.join(".config").join("pow"))
}

pub fn config_path() -> Result<PathBuf> {
    if let Some(path) = config_path_override() {
        return Ok(path);
    }
    Ok(config_dir()?.join("config.toml"))
}

fn config_path_override() -> Option<PathBuf> {
    std::env::var_os("POW_CONFIG").map(PathBuf::from)
}

/// `~/workspaces/`.
pub fn workspaces_root() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("POW_WORKSPACES_ROOT") {
        if !root.is_empty() {
            return Ok(PathBuf::from(root));
        }
    }
    Ok(home_dir()?.join("workspaces"))
}

pub fn workspace_path(name: &str) -> Result<PathBuf> {
    Ok(workspaces_root()?.join(name))
}

pub fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| PowError::Config("HOME not set".into()))
}

/// Expand `~` and `$VAR` references in a path string.
pub fn expand_path(raw: &str) -> Result<PathBuf> {
    let expanded = shellexpand::full(raw)
        .map_err(|e| PowError::Config(format!("failed to expand path '{raw}': {e}")))?;
    Ok(PathBuf::from(expanded.into_owned()))
}
