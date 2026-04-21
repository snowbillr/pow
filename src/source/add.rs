use crate::error::{PowError, Result};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    _name: &str,
    _path: &str,
    _github_org: Option<&str>,
    _base_branch: &str,
    _include: &[String],
    _exclude: &[String],
    _all: bool,
    _skip_archived: bool,
) -> Result<()> {
    Err(PowError::Message("not yet implemented".into()))
}
