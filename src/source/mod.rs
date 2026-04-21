pub mod add;
pub mod filter;
pub mod sync;

use crate::error::{PowError, Result};

pub fn list(_json: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}

pub fn remove(_name: &str, _force: bool) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
