#[derive(thiserror::Error, Debug)]
pub enum PowError {
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("{0}")]
    RepoNotFound(String),
    #[error("source not found: {0}")]
    SourceNotFound(String),
    #[error("git operation failed: {0}")]
    GitFailed(String),
    #[error("github API error: {0}")]
    GithubError(#[from] octocrab::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config error: {0}")]
    Config(String),
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Other(#[from] color_eyre::eyre::Error),
}

impl PowError {
    pub fn exit_code(&self) -> i32 {
        match self {
            PowError::WorkspaceNotFound(_) => 2,
            PowError::RepoNotFound(_) => 3,
            PowError::SourceNotFound(_) => 4,
            PowError::GitFailed(_) => 5,
            PowError::GithubError(_) => 6,
            _ => 1,
        }
    }
}

impl From<toml::de::Error> for PowError {
    fn from(e: toml::de::Error) -> Self {
        PowError::Config(e.to_string())
    }
}

impl From<toml::ser::Error> for PowError {
    fn from(e: toml::ser::Error) -> Self {
        PowError::Config(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PowError>;
