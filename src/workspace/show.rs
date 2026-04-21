use crate::error::{PowError, Result};

pub fn list(_json: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn show(_name: Option<&str>, _json: bool, _no_status: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
