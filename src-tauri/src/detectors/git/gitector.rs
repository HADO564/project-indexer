use git2::{ErrorCode, Repository, StatusOptions};
use std::path::Path;

use crate::errors::GitError;

/// Opens the repository that owns `path`.
///
/// Discovery walks upwards, so any file or nested directory inside a work tree
/// resolves to the repository containing it — the caller doesn't have to know
/// where the `.git` directory actually lives.
///
/// A path that simply isn't under version control is reported as
/// `NotRepository` rather than `RepositoryDiscovery`, so callers can treat "not
/// a project we track" as an ordinary outcome instead of a failure.
pub fn open_repo(path: &Path) -> Result<Repository, GitError> {
    Repository::discover(path).map_err(|e| match e.code() {
        ErrorCode::NotFound => GitError::NotRepository(path.display().to_string()),
        _ => GitError::RepositoryDiscovery(e),
    })
}

/// Whether `path` sits inside a git work tree.
///
/// Only a genuine discovery failure (an unreadable directory, a corrupt
/// repository) surfaces as an error; a path that is merely outside any
/// repository is `Ok(false)`.
pub fn is_repo(path: &Path) -> Result<bool, GitError> {
    match open_repo(path) {
        Ok(_) => Ok(true),
        Err(GitError::NotRepository(_)) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Name of the branch HEAD is on, or `None` when HEAD names a commit rather
/// than a branch.
///
/// `None` means there is no current branch — a detached HEAD, or a HEAD that
/// isn't symbolic. That is a normal state, not a failure, so it is reported in
/// the success case; only a repository we genuinely can't read is an `Err`.
pub fn get_current_branch(repo: &Repository) -> Result<Option<String>, GitError> {
    // Checked up front because a detached HEAD still resolves happily — it just
    // resolves to a commit, whose shorthand is the literal "HEAD".
    if is_detached(repo)? {
        return Ok(None);
    }

    match repo.head() {
        // Fails only when the ref name isn't valid UTF-8, which is worth
        // surfacing rather than papering over.
        Ok(head) => Ok(Some(
            head.shorthand().map_err(GitError::Branch)?.to_string(),
        )),

        // A repository with no commits yet has HEAD pointing at a branch that
        // does not exist as a ref, and libgit2 refuses to resolve it. Git still
        // considers that branch current (`git branch --show-current` prints it),
        // so read the name straight off the symbolic HEAD instead of treating a
        // freshly initialised project as a detection failure.
        Err(e) if e.code() == ErrorCode::UnbornBranch => unborn_branch(repo),

        Err(e) => Err(GitError::Branch(e)),
    }
}

/// Reads the branch name out of an unresolvable HEAD, e.g. `refs/heads/main`
/// in a repository that has no commits. `None` if HEAD isn't symbolic.
fn unborn_branch(repo: &Repository) -> Result<Option<String>, GitError> {
    let head = repo.find_reference("HEAD").map_err(GitError::Branch)?;

    Ok(head
        .symbolic_target()
        .map_err(GitError::Branch)?
        .map(|target| {
            target
                .strip_prefix("refs/heads/")
                .unwrap_or(target)
                .to_string()
        }))
}

/// Whether HEAD points straight at a commit rather than a branch.
pub fn is_detached(repo: &Repository) -> Result<bool, GitError> {
    repo.head_detached().map_err(GitError::Branch)
}

/// Root of the work tree. `None` for a bare repository, which has no checkout.
pub fn repo_root(repo: &Repository) -> Option<&Path> {
    repo.workdir()
}

/// Whether the work tree has uncommitted changes.
///
/// Untracked files count as dirty — a project with new, unsaved work in it is
/// not in a clean state — but ignored files do not.
pub fn is_dirty(repo: &Repository) -> Result<bool, GitError> {
    let mut options = StatusOptions::new();
    options.include_untracked(true).include_ignored(false);

    let statuses = repo
        .statuses(Some(&mut options))
        .map_err(GitError::Status)?;

    Ok(!statuses.is_empty())
}
