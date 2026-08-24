use thiserror::Error;

#[derive(Error, Debug)]
pub enum GitError{
    #[error("Failed to discover repository at {0}")]
    RepositoryDiscovery(String),

    #[error("This path is not inside a git repository: {0}")]
    NotRepository(String),

    #[error("Failed to determine current branch: {0}")]
    Branch(String),

    #[error("Failed to determine repository status: {0}")]
    Status(String),
}
