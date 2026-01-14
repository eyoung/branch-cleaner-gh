use thiserror::Error;

/// Centralized error types for the branch cleaner application
#[derive(Error, Debug)]
pub enum BranchCleanerError {
    #[error("Git error: {0}")]
    GitError(#[from] git2::Error),

    #[error("GitHub API error: {0}")]
    GitHubError(#[from] octocrab::Error),

    #[error("Remote URL parsing error: {0}")]
    RemoteParseError(String),

    #[error("GitHub token not found in environment")]
    TokenNotFound,

    #[error("No origin remote found in repository")]
    NoOriginRemote,
}

pub type Result<T> = std::result::Result<T, BranchCleanerError>;
