use crate::error::{PowError, Result};

pub fn use_workspace(_name: &str) -> Result<()> {
    Err(PowError::Message(
        "`pow use` requires shell integration. Add `eval \"$(pow init)\"` to your ~/.zshrc.".into(),
    ))
}

pub fn cd(_name: Option<&str>, _entry: Option<&str>) -> Result<()> {
    Err(PowError::Message(
        "`pow cd` requires shell integration. Add `eval \"$(pow init)\"` to your ~/.zshrc.".into(),
    ))
}

pub fn current(_json: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn resolve_use(_name: &str) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn resolve_cd(_args: &[String]) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
