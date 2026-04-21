use crate::error::{PowError, Result};

pub async fn run(
    _name: &str,
    _dry_run: bool,
    _prune: bool,
    _parallel: Option<usize>,
) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
