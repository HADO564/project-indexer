//! Tests for [`crate::domain::naming`].

use crate::domain::naming::*;
use crate::domain::{GitInfo, Tracker};

fn git_tracker(repo_url: Option<&str>) -> Tracker {
    Tracker::Git(GitInfo {
        repo_root: "/tmp/x".into(),
        dirty: false,
        detached_head: false,
        repo_url: repo_url.map(str::to_string),
        web_url: None,
        contributors: vec![],
        curr_branch: None,
        branches: None,
        commit_hash: None,
    })
}

#[test]
fn repo_name_from_https_url() {
    assert_eq!(
        repo_name_from_url("https://github.com/user/my-repo.git").as_deref(),
        Some("my-repo")
    );
    assert_eq!(
        repo_name_from_url("https://github.com/user/my-repo").as_deref(),
        Some("my-repo")
    );
}

#[test]
fn repo_name_from_ssh_url() {
    assert_eq!(
        repo_name_from_url("git@github.com:user/my-repo.git").as_deref(),
        Some("my-repo")
    );
}

#[test]
fn repo_name_ignores_trailing_slash() {
    assert_eq!(
        repo_name_from_url("https://github.com/user/my-repo/").as_deref(),
        Some("my-repo")
    );
}

#[test]
fn folder_name_from_windows_path() {
    assert_eq!(
        folder_name_from_directory("D:\\Projects\\Friction\\").as_deref(),
        Some("Friction")
    );
}

#[test]
fn folder_name_from_unix_path() {
    assert_eq!(
        folder_name_from_directory("/home/user/friction").as_deref(),
        Some("friction")
    );
}

#[test]
fn suggest_prefers_git_remote_name() {
    let t = [git_tracker(Some("https://github.com/user/cool-thing.git"))];
    assert_eq!(
        suggest_project_name(&t, "/home/user/local-dir").as_deref(),
        Some("cool-thing")
    );
}

#[test]
fn suggest_falls_back_to_folder_name() {
    let t = [git_tracker(None)];
    assert_eq!(
        suggest_project_name(&t, "/home/user/local-dir").as_deref(),
        Some("local-dir")
    );
    assert_eq!(
        suggest_project_name(&[], "/home/user/local-dir").as_deref(),
        Some("local-dir")
    );
}

#[test]
fn suggest_returns_none_for_empty_directory_and_no_trackers() {
    assert_eq!(suggest_project_name(&[], ""), None);
}

fn taken(names: &[&str]) -> std::collections::HashSet<String> {
    taken_names_from(names.iter().map(|s| s.to_string()))
}

#[test]
fn disambiguate_keeps_a_free_name() {
    assert_eq!(disambiguate("api", "/home/user/work", &taken(&[])), "api");
}

#[test]
fn disambiguate_qualifies_by_parent_on_collision() {
    assert_eq!(
        disambiguate("api", "/home/user/work", &taken(&["api"])),
        "work/api"
    );
}

#[test]
fn disambiguate_suffixes_when_the_qualified_name_also_collides() {
    assert_eq!(
        disambiguate("api", "/home/user/work", &taken(&["api", "work/api"])),
        "work/api (2)"
    );
    assert_eq!(
        disambiguate(
            "api",
            "/home/user/work",
            &taken(&["api", "work/api", "work/api (2)"])
        ),
        "work/api (3)"
    );
}

/// The collision rule is case-insensitive, matching
/// `check_for_duplicate_name_or_dir`, which compares with
/// `eq_ignore_ascii_case`. A scanner that produced `API` next to an
/// existing `api` would be rejected by `create` at commit time.
#[test]
fn disambiguate_matches_case_insensitively() {
    assert_eq!(
        disambiguate("API", "/home/user/work", &taken(&["api"])),
        "work/API"
    );
}

/// Windows paths reach this function too — the parent segment has to come
/// off a backslash path as readily as a forward-slash one.
#[test]
fn disambiguate_reads_a_windows_parent() {
    assert_eq!(
        disambiguate("api", "D:\\work", &taken(&["api"])),
        "work/api"
    );
}

/// A parent that yields no usable segment (a drive root, an empty string)
/// must still terminate, falling straight through to the suffix.
#[test]
fn disambiguate_falls_back_to_a_suffix_without_a_parent() {
    assert_eq!(disambiguate("api", "", &taken(&["api"])), "api (2)");
}
