use crate::error::{PowError, Result};

pub fn new(_name: &str, _force: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn add(
    _repo: &str,
    _workspace: Option<&str>,
    _branch: Option<&str>,
    _from: Option<&str>,
) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn forget(_repo: &str, _workspace: Option<&str>, _prune_branch: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn rm(_name: &str, _prune_branches: bool, _force: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
