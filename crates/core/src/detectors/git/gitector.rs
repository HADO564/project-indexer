use crate::detectors::detector::Detector;
use git2::{BranchType, ErrorCode, Repository, StatusOptions};
use std::path::Path;

use crate::domain::git::GitInfo;
use crate::domain::tracker::Tracker;
use crate::error::{DetectorError, GitError};

pub struct Gitector;

impl Detector for Gitector {
    fn kind(&self) -> &'static str {
        "git"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let repo = match open_repo(path) {
            Ok(repo) => repo,
            Err(GitError::NotRepository(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        // Bare repositories have no work tree, so there's nothing to be dirty.
        let dirty = if repo.is_bare() {
            false
        } else {
            is_dirty(&repo)?
        };

        let root = repo_root(&repo)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| path.display().to_string());

        // A repo that made it past `open_repo` has at least one branch in
        // the ordinary case; `None` is reserved for the genuine edge case of
        // zero local branches (e.g. a freshly-initialized repo with no
        // commits yet) rather than standing in for an empty list.
        let branches = list_branches(&repo)?;
        let branches = (!branches.is_empty()).then_some(branches);

        let repo_url = remote_url(&repo, "origin")?;
        let web_url = repo_url.as_deref().and_then(web_url);

        Ok(Some(Tracker::Git(GitInfo {
            repo_root: root,
            dirty,
            detached_head: is_detached(&repo)?,
            repo_url,
            web_url,
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

/// Browser-openable form of a git remote URL, or `None` if it isn't a
/// recognizable http(s)/ssh git remote (a bare local path, say).
///
/// `git@host:owner/repo.git` and `ssh://git@host/owner/repo.git` and
/// `https://host/owner/repo.git` all normalize to `https://host/owner/repo`.
fn web_url(remote: &str) -> Option<String> {
    let remote = remote.trim();
    if remote.is_empty() {
        return None;
    }

    let is_ssh = remote.starts_with("git@") || remote.starts_with("ssh://");

    let (host, path) = remote
        .strip_prefix("git@")
        .and_then(|rest| rest.split_once(':'))
        .or_else(|| {
            remote
                .strip_prefix("ssh://git@")
                .or_else(|| remote.strip_prefix("ssh://"))
                .and_then(|rest| rest.split_once('/'))
        })
        .or_else(|| {
            remote
                .strip_prefix("https://")
                .or_else(|| remote.strip_prefix("http://"))
                .and_then(|rest| rest.split_once('/'))
        })?;

    // Drop any `user[:token]@` userinfo prefix (e.g. an `x-access-token` in an
    // https remote), then, for an ssh URL, a `:port` suffix on the host.
    let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
    let host = if is_ssh {
        host.rsplit_once(':')
            .filter(|(_, port)| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
            .map_or(host, |(h, _)| h)
    } else {
        host
    };

    let path = path.strip_suffix('/').unwrap_or(path);
    let path = path.strip_suffix(".git").unwrap_or(path);
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("https://{host}/{path}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{IndexAddOption, RepositoryInitOptions, Signature};
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("project-indexer-tests-git-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("should create temp dir");
        dir
    }

    /// Inits a repo with a fixed initial branch name (`main`), independent
    /// of the host's `init.defaultBranch` config — otherwise these tests'
    /// branch-name assertions would depend on machine-local git config.
    fn init_repo(dir: &Path) -> Repository {
        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("main");
        Repository::init_opts(dir, &opts).expect("should init repo")
    }

    /// Stages everything in the work tree and commits it, independent of
    /// any user.name/user.email git config the test machine may or may not
    /// have set.
    fn commit_all(repo: &Repository, message: &str) {
        let signature =
            Signature::now("Test User", "test@example.com").expect("should build signature");

        let mut index = repo.index().expect("should open index");
        index
            .add_all(["*"], IndexAddOption::DEFAULT, None)
            .expect("should stage files");
        index.write().expect("should write index");
        let tree = repo
            .find_tree(index.write_tree().expect("should write tree"))
            .expect("should find tree");

        let parents = match repo.head() {
            Ok(head) => vec![head.peel_to_commit().expect("should peel HEAD to a commit")],
            Err(_) => Vec::new(),
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )
        .expect("should commit");
    }

    #[test]
    fn detect_recognizes_a_git_repository() {
        let dir = temp_dir("detect-true");
        init_repo(&dir);

        let result = Gitector.detect(&dir).expect("should detect");

        assert!(matches!(result, Some(Tracker::Git(_))));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_returns_none_for_a_plain_directory() {
        let dir = temp_dir("detect-false");

        let result = Gitector.detect(&dir).expect("should detect");

        assert!(result.is_none());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_reports_a_freshly_initialized_repo_with_no_commits() {
        let dir = temp_dir("get-info-unborn");
        init_repo(&dir);

        let tracker = Gitector
            .detect(&dir)
            .expect("should get info")
            .expect("should recognize the repo");

        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        // No commits yet, so HEAD is "unborn" — current branch still reads
        // from the symbolic ref name rather than a resolved commit.
        assert_eq!(info.curr_branch.as_deref(), Some("main"));
        assert_eq!(info.commit_hash, None);
        assert_eq!(info.branches, None);
        assert!(!info.dirty);
        assert!(!info.detached_head);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_reports_a_repo_with_a_commit() {
        let dir = temp_dir("get-info-committed");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("README.md"), "hello").expect("should write file");
        commit_all(&repo, "initial commit");

        let tracker = Gitector
            .detect(&dir)
            .expect("should get info")
            .expect("should recognize the repo");

        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        assert_eq!(info.curr_branch.as_deref(), Some("main"));
        assert!(info.commit_hash.is_some());
        assert_eq!(info.branches, Some(vec!["main".to_string()]));
        assert!(!info.dirty);
        assert_eq!(info.repo_url, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_reports_dirty_state_for_untracked_files() {
        let dir = temp_dir("get-info-dirty");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("README.md"), "hello").expect("should write file");
        commit_all(&repo, "initial commit");

        std::fs::write(dir.join("scratch.txt"), "wip").expect("should write untracked file");

        let tracker = Gitector
            .detect(&dir)
            .expect("should get info")
            .expect("should recognize the repo");

        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        assert!(info.dirty);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_reports_the_origin_remote_url() {
        let dir = temp_dir("get-info-remote");
        let repo = init_repo(&dir);
        repo.remote("origin", "https://example.com/user/repo.git")
            .expect("should add remote");

        let tracker = Gitector
            .detect(&dir)
            .expect("should get info")
            .expect("should recognize the repo");

        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        assert_eq!(
            info.repo_url.as_deref(),
            Some("https://example.com/user/repo.git")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_reports_every_local_branch() {
        let dir = temp_dir("get-info-branches");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("README.md"), "hello").expect("should write file");
        commit_all(&repo, "initial commit");

        let head_commit = repo
            .head()
            .expect("should read HEAD")
            .peel_to_commit()
            .expect("should peel HEAD to a commit");
        repo.branch("feature", &head_commit, false)
            .expect("should create branch");

        let tracker = Gitector
            .detect(&dir)
            .expect("should get info")
            .expect("should recognize the repo");

        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        let mut branches = info.branches.expect("should have branches");
        branches.sort();
        assert_eq!(branches, vec!["feature".to_string(), "main".to_string()]);
        // Creating a branch doesn't check it out — HEAD stays on "main".
        assert_eq!(info.curr_branch.as_deref(), Some("main"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_reports_a_detached_head() {
        let dir = temp_dir("get-info-detached");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("README.md"), "hello").expect("should write file");
        commit_all(&repo, "initial commit");

        let head_oid = repo
            .head()
            .expect("should read HEAD")
            .target()
            .expect("should have a target");
        repo.set_head_detached(head_oid)
            .expect("should detach HEAD");

        let tracker = Gitector
            .detect(&dir)
            .expect("should get info")
            .expect("should recognize the repo");

        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        assert!(info.detached_head);
        assert_eq!(info.curr_branch, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn kind_is_git() {
        assert_eq!(Gitector.kind(), "git");
    }

    #[test]
    fn web_url_normalizes_common_remote_forms() {
        assert_eq!(
            web_url("git@github.com:acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            web_url("ssh://git@gitlab.com/acme/repo.git").as_deref(),
            Some("https://gitlab.com/acme/repo")
        );
        assert_eq!(
            web_url("https://github.com/acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            web_url("https://github.com/acme/repo").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            web_url("https://x-access-token:ghp_SECRET@github.com/acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(
            web_url("ssh://git@github.com:2222/acme/repo.git").as_deref(),
            Some("https://github.com/acme/repo")
        );
        assert_eq!(web_url("/srv/git/repo.git"), None);
        assert_eq!(web_url(""), None);
    }

    #[test]
    fn get_info_derives_web_url_from_an_ssh_remote() {
        let dir = temp_dir("web-url");
        let repo = init_repo(&dir);
        repo.remote("origin", "git@github.com:acme/friction-engine.git")
            .expect("should add remote");

        let tracker = Gitector
            .detect(&dir)
            .expect("should detect")
            .expect("should recognize the repo");
        let Tracker::Git(info) = tracker else {
            panic!("expected Tracker::Git");
        };

        assert_eq!(
            info.web_url.as_deref(),
            Some("https://github.com/acme/friction-engine")
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
