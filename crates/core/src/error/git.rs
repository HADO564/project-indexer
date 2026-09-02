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
