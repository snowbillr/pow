use crate::config::Config;
use crate::error::{PowError, Result};

#[allow(clippy::too_many_arguments)]
pub async fn run_with_github(
    _cfg: &mut Config,
    _name: &str,
    _stored_path: &str,
    _org: &str,
    _base_branch: &str,
    _include: &[String],
    _exclude: &[String],
    _all: bool,
    _skip_archived: bool,
) -> Result<()> {
    Err(PowError::Message(
        "--github-org support is implemented in Phase 6.".into(),
    ))
}
