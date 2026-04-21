use crate::error::{PowError, Result};
use crate::paths;
use crate::workspace::{active_workspace, Workspace};

const SHELL_HINT: &str =
    "shell integration not detected. Add `eval \"$(pow init)\"` to your ~/.zshrc.";

pub fn use_workspace(_name: &str) -> Result<()> {
    Err(PowError::Message(SHELL_HINT.to_string()))
}

pub fn cd(_name: Option<&str>, _entry: Option<&str>) -> Result<()> {
    Err(PowError::Message(SHELL_HINT.to_string()))
}

pub fn current(json: bool) -> Result<()> {
    match active_workspace() {
        Some(name) => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"active": name})).unwrap()
                );
            } else {
                println!("{name}");
            }
        }
        None => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string(&serde_json::json!({"active": null})).unwrap()
                );
            } else {
                println!("no active workspace");
            }
        }
    }
    Ok(())
}

/// Hidden: validate the workspace exists, echo its name.
pub fn resolve_use(name: &str) -> Result<()> {
    let _ = Workspace::scan(name)?;
    println!("{name}");
    Ok(())
}

/// Hidden: resolve a cd target to an absolute path.
pub fn resolve_cd(args: &[String]) -> Result<()> {
    match args.len() {
        0 => {
            let name = active_workspace().ok_or_else(|| {
                PowError::Message(
                    "no active workspace. `pow use <name>` or pass an argument to `pow cd`."
                        .to_string(),
                )
            })?;
            let path = paths::workspace_path(&name)?;
            if !path.exists() {
                return Err(PowError::WorkspaceNotFound(name));
            }
            println!("{}", path.display());
        }
        1 => {
            let name = &args[0];
            let path = paths::workspace_path(name)?;
            if !path.exists() {
                return Err(PowError::WorkspaceNotFound(name.clone()));
            }
            println!("{}", path.display());
        }
        2 => {
            let name = &args[0];
            let entry = &args[1];
            let path = paths::workspace_path(name)?.join(entry);
            if !path.exists() {
                return Err(PowError::RepoNotFound(format!(
                    "no entry '{entry}' in workspace '{name}'"
                )));
            }
            println!("{}", path.display());
        }
        _ => {
            return Err(PowError::Message(
                "usage: pow cd [<workspace>] [<entry>]".to_string(),
            ));
        }
    }
    Ok(())
}
