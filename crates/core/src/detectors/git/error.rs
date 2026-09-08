use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Failed to discover repository")]
    RepositoryDiscovery(#[source] git2::Error),

    #[error("This path is not inside a git repository: {0}")]
    NotRepository(String),

    #[error("Failed to determine current branch")]
    Branch(#[source] git2::Error),

    #[error("Failed to determine repository status: {0}")]
    Status(#[source] git2::Error),

    #[error("Failed to read remote information")]
    Remote(#[source] git2::Error),
}

/// Lets `?` lift a [`GitError`] into the shared [`DetectorError`] without
/// that enum carrying a variant per detector. Living here is what keeps
/// a new detector's blast radius inside its own directory.
impl From<GitError> for crate::error::DetectorError {
    fn from(e: GitError) -> Self {
        crate::error::DetectorError::Other(Box::new(e))
    }
}
