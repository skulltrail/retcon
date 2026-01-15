use thiserror::Error;

/// All possible errors that can occur in retcon
#[derive(Error, Debug)]
pub enum RetconError {
    #[error("Not a git repository: {0}")]
    NotARepository(String),

    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Invalid email format: {0}")]
    InvalidEmail(String),

    #[error("Invalid date format: {0}. Expected: YYYY-MM-DD HH:MM:SS [+/-]HHMM")]
    InvalidDate(String),

    #[error("No commits found in repository")]
    NoCommits,

    #[error("Cannot rewrite history: {0}")]
    RewriteFailed(String),

    #[error("Rebase in progress - complete or abort first")]
    RebaseInProgress,

    #[error("Merge in progress - complete or abort first")]
    MergeInProgress,

    #[error("Uncommitted changes detected - commit or stash first")]
    DirtyWorkingTree,

    #[allow(dead_code)]
    #[error("Cannot modify commits that have been pushed to remote without --force")]
    RemoteCommits,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Terminal error: {0}")]
    Terminal(String),

    #[error("Commit not found: {0}")]
    CommitNotFound(String),

    #[allow(dead_code)]
    #[error("Invalid commit range: {0}")]
    InvalidRange(String),

    #[allow(dead_code)]
    #[error("Operation cancelled by user")]
    Cancelled,
}

/// Alias for backwards compatibility
pub type HistError = RetconError;

pub type Result<T> = std::result::Result<T, RetconError>;
