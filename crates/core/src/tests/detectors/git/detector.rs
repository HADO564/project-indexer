//! Tests for [`crate::detectors::git::detector`].

use crate::detectors::detector::Detector;
use crate::domain::Tracker;
use git2::Repository;
use std::path::Path;

use crate::detectors::git::detector::*;
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
