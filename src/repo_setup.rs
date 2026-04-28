use std::path::{Component, Path};
use std::process::Command;

use serde::Deserialize;

use crate::error::{PowError, Result};

const FILE_NAME: &str = ".pow.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(default)]
    setup: Setup,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Setup {
    #[serde(default)]
    commands: Vec<String>,
    #[serde(default)]
    copy: Vec<String>,
}

#[derive(Debug, Default)]
pub struct RepoSetup {
    pub commands: Vec<String>,
    pub copy: Vec<String>,
}

/// Read `.pow.toml` from `worktree_dir`. Returns `Ok(None)` when the file is
/// absent or declares no setup work; returns `Err` only when the file exists
/// but is malformed.
pub fn load(worktree_dir: &Path) -> Result<Option<RepoSetup>> {
    let path = worktree_dir.join(FILE_NAME);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|e| PowError::Config(format!("reading {}: {e}", path.display())))?;
    let parsed: File = toml::from_str(&text)
        .map_err(|e| PowError::Config(format!("{}: {e}", path.display())))?;
    if parsed.setup.commands.is_empty() && parsed.setup.copy.is_empty() {
        return Ok(None);
    }
    Ok(Some(RepoSetup {
        commands: parsed.setup.commands,
        copy: parsed.setup.copy,
    }))
}

/// Run each shell command sequentially in `worktree_dir`, inheriting stdio so
/// the user sees progress live. Failures are printed as warnings and do not
/// abort the sequence.
pub fn run_commands(worktree_dir: &Path, commands: &[String]) {
    for cmd in commands {
        println!("==> running: {cmd}");
        let status = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(worktree_dir)
            .status();
        match status {
            Ok(s) if s.success() => {}
            Ok(s) => eprintln!("warning: setup command '{cmd}' exited with {s}"),
            Err(e) => eprintln!("warning: failed to run setup command '{cmd}': {e}"),
        }
    }
}

/// Copy each path from the source clone working directory into the worktree.
/// Missing source files are skipped silently (a `.env` may legitimately not
/// exist yet). Other failures print a warning and continue.
pub fn copy_files(source_clone: &Path, worktree_dir: &Path, files: &[String]) {
    for rel in files {
        if !is_safe_relative(Path::new(rel)) {
            eprintln!(
                "warning: skipping unsafe copy path '{rel}' (must be relative, no '..')"
            );
            continue;
        }
        let src = source_clone.join(rel);
        if !src.exists() {
            continue;
        }
        let dest = worktree_dir.join(rel);
        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                eprintln!("warning: could not create parent dir for '{rel}': {e}");
                continue;
            }
        }
        match std::fs::copy(&src, &dest) {
            Ok(_) => println!("==> copied: {rel}"),
            Err(e) => eprintln!("warning: failed to copy '{rel}': {e}"),
        }
    }
}

fn is_safe_relative(p: &Path) -> bool {
    if p.is_absolute() {
        return false;
    }
    for comp in p.components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_relative_paths() {
        assert!(is_safe_relative(Path::new(".env")));
        assert!(is_safe_relative(Path::new("config/local.yml")));
        assert!(is_safe_relative(Path::new("./.env")));
        assert!(!is_safe_relative(Path::new("/etc/passwd")));
        assert!(!is_safe_relative(Path::new("../escape")));
        assert!(!is_safe_relative(Path::new("a/../b")));
    }

    #[test]
    fn load_missing_file_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn load_empty_setup_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".pow.toml"), "[setup]\n").unwrap();
        assert!(load(dir.path()).unwrap().is_none());
    }

    #[test]
    fn load_parses_commands_and_copy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".pow.toml"),
            "[setup]\ncommands = [\"npm i\"]\ncopy = [\".env\"]\n",
        )
        .unwrap();
        let setup = load(dir.path()).unwrap().expect("expected Some");
        assert_eq!(setup.commands, vec!["npm i".to_string()]);
        assert_eq!(setup.copy, vec![".env".to_string()]);
    }

    #[test]
    fn load_malformed_errors() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".pow.toml"), "not valid toml [").unwrap();
        assert!(load(dir.path()).is_err());
    }
}
