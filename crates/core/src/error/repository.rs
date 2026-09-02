use thiserror::Error;

#[derive(Debug, Error)]
pub enum RepositoryError {
    #[error("project store is unavailable: {0}")]
    Backend(String),
    #[error("project store holds a record that can't be read: {0}")]
    Corrupt(String),
}
