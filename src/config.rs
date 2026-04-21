use crate::error::{PowError, Result};

pub fn cmd_print(_json: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn cmd_get(_key: &str) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn cmd_set(_key: &str, _value: &str) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
