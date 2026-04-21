use crate::error::{PowError, Result};

pub fn switch(
    _repo: &str,
    _target: &str,
    _new: bool,
    _workspace: Option<&str>,
) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub async fn sync(
    _repo: Option<&str>,
    _all: bool,
    _workspace: Option<&str>,
) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn status(_name: Option<&str>, _dirty_only: bool, _short: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub async fn exec(
    _command: &str,
    _workspace: Option<&str>,
    _parallel: Option<usize>,
    _dry_run: bool,
) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
