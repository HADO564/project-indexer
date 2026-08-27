use git2::{BranchType, ErrorCode, Repository, StatusOptions};
use std::path::Path;
use crate::detectors::detector::Detector;

use crate::errors::{DetectorError, GitError};
use crate::models::git::GitInfo;
use crate::models::tracker::Tracker;

pub struct Gitector;

impl Detector for Gitector {
    fn detect(&self, path: &Path) -> Result<bool, DetectorError> {
        Ok(is_repo(path)?)
    }

    fn get_info(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let repo = match open_repo(path) {
            Ok(repo) => repo,
            Err(GitError::NotRepository(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        // Bare repositories have no work tree, so there's nothing to be dirty.
        let dirty = if repo.is_bare() { false } else { is_dirty(&repo)? };

        let root = repo_root(&repo)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| path.display().to_string());

        // A repo that made it past `open_repo` has at least one branch in
        // the ordinary case; `None` is reserved for the genuine edge case of
        // zero local branches (e.g. a freshly-initialized repo with no
        // commits yet) rather than standing in for an empty list.
        let branches = list_branches(&repo)?;
        let branches = (!branches.is_empty()).then_some(branches);

        Ok(Some(Tracker::Git(GitInfo {
            repo_root: root,
            dirty,
            detached_head: is_detached(&repo)?,
            repo_url: remote_url(&repo, "origin")?,
            // Walking full commit history for authors is a separate feature;
            // left empty until that's built.
            contributors: Vec::new(),
            curr_branch: get_current_branch(&repo)?,
            branches,
            commit_hash: head_commit_hash(&repo)?,
        })))
    }
}

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

/// Names of every local branch in the repository, current branch included —
/// unlike [`get_current_branch`], which reports only the one HEAD is on.
pub fn list_branches(repo: &Repository) -> Result<Vec<String>, GitError> {
    let mut names = Vec::new();

    for entry in repo
        .branches(Some(BranchType::Local))
        .map_err(GitError::Branch)?
    {
        let (branch, _) = entry.map_err(GitError::Branch)?;
        if let Some(name) = branch.name().map_err(GitError::Branch)? {
            names.push(name.to_string());
        }
    }

    Ok(names)
}

/// Hash of the commit HEAD currently points at, or `None` for a repository
/// with no commits yet (an unborn HEAD) — the same normal-not-a-failure case
/// [`get_current_branch`] handles.
pub fn head_commit_hash(repo: &Repository) -> Result<Option<String>, GitError> {
    match repo.head() {
        Ok(head) => Ok(head.target().map(|oid| oid.to_string())),
        Err(e) if e.code() == ErrorCode::UnbornBranch => Ok(None),
        Err(e) => Err(GitError::Branch(e)),
    }
}

/// URL of the remote named `name` (typically `"origin"`), or `None` if no
/// such remote is configured — a normal state for a purely local repository.
pub fn remote_url(repo: &Repository, name: &str) -> Result<Option<String>, GitError> {
    match repo.find_remote(name) {
        // An empty URL means the remote has none configured (git2 reports
        // this as `Ok("")` rather than `None`), which is a normal state, not
        // a failure — same treatment as a missing remote below.
        Ok(remote) => {
            let url = remote.url().map_err(GitError::Remote)?;
            Ok((!url.is_empty()).then(|| url.to_string()))
        }
        Err(e) if e.code() == ErrorCode::NotFound => Ok(None),
        Err(e) => Err(GitError::Remote(e)),
    }
}
