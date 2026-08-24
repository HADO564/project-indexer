use git2::Repository;
use std::path::Path;

use crate::errors::GitError;

pub fn is_repo(path: &Path) -> Result<bool, GitError> {
    Repository::discover(path)
        .map(|_| true)
        .map_err(|e| GitError::RepositoryDiscovery(e.to_string()))
}
