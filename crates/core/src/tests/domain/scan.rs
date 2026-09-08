//! Tests for [`crate::domain::scan`].

use crate::detectors::DetectorRunner;

use crate::detectors::git::Gitector;
use crate::domain::scan::*;
use std::path::{Path, PathBuf};

fn runner() -> DetectorRunner {
    DetectorRunner::new(vec![Box::new(Gitector)])
}

fn temp_tree(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-scan-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

fn repo_at(root: &Path, relative: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(&path).expect("should create dir");
    git2::Repository::init(&path).expect("should init a git repo");
    path
}

fn plain_at(root: &Path, relative: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(&path).expect("should create dir");
    path
}

fn request(root: &Path, mode: ScanMode) -> ScanRequest {
    ScanRequest {
        scan_root: root.to_string_lossy().to_string(),
        mode,
        detectors: vec!["git".to_string()],
        include_ignored: false,
    }
}

fn directories(report: &ScanReport) -> Vec<String> {
    report
        .candidates
        .iter()
        .map(|c| {
            Path::new(&c.directory)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        })
        .collect()
}

#[test]
fn quick_scan_finds_children_and_stops() {
    let root = temp_tree("quick");
    repo_at(&root, "api");
    repo_at(&root, "site");
    repo_at(&root, "nested/deep-repo");

    let report = walk(&request(&root, ScanMode::Quick), &runner());

    assert_eq!(directories(&report), vec!["api", "site"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn deep_scan_descends_to_the_requested_depth() {
    let root = temp_tree("deep");
    // Not named "target": that collides with the Rust build directory in
    // `CONVENTIONALLY_IGNORED` and would be pruned before ever being
    // visited, regardless of depth.
    repo_at(&root, "one/two/leaf");

    let shallow = walk(&request(&root, ScanMode::Deep { depth: 2 }), &runner());
    assert!(shallow.candidates.is_empty());

    let deep = walk(&request(&root, ScanMode::Deep { depth: 3 }), &runner());
    assert_eq!(directories(&deep), vec!["leaf"]);

    std::fs::remove_dir_all(&root).ok();
}

/// A repository inside a repository is vendored or a submodule, not a
/// separate thing to track — so a matched directory ends that branch.
#[test]
fn a_project_is_a_boundary_and_its_children_are_not_visited() {
    let root = temp_tree("boundary");
    let outer = repo_at(&root, "outer");
    repo_at(&outer, "vendor/inner");

    let report = walk(&request(&root, ScanMode::Deep { depth: 5 }), &runner());

    assert_eq!(directories(&report), vec!["outer"]);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn ignored_directories_are_pruned_by_default_and_included_on_request() {
    let root = temp_tree("prune");
    repo_at(&root, "node_modules/some-dep");

    let pruned = walk(&request(&root, ScanMode::Deep { depth: 3 }), &runner());
    assert!(pruned.candidates.is_empty());

    let mut with_ignored = request(&root, ScanMode::Deep { depth: 3 });
    with_ignored.include_ignored = true;
    let included = walk(&with_ignored, &runner());
    assert_eq!(directories(&included), vec!["some-dep"]);

    std::fs::remove_dir_all(&root).ok();
}

/// The correctness rule, not a performance one: a repository's submodules
/// live under `.git/modules` and are real repositories. Reaching them would
/// offer to import every submodule as its own project, so `.git` is pruned
/// even when the user asked to include ignored directories.
#[test]
fn vcs_metadata_is_pruned_even_when_ignored_directories_are_included() {
    let root = temp_tree("vcs-metadata");
    let outer = plain_at(&root, "outer");
    repo_at(&outer, ".git/modules/sub");

    let mut req = request(&root, ScanMode::Deep { depth: 6 });
    req.include_ignored = true;
    let report = walk(&req, &runner());

    assert!(
        report.candidates.is_empty(),
        "submodule repositories under .git must never be offered: {:?}",
        directories(&report)
    );
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn nothing_ticked_finds_nothing() {
    let root = temp_tree("none-ticked");
    repo_at(&root, "api");

    let mut req = request(&root, ScanMode::Quick);
    req.detectors.clear();
    let report = walk(&req, &runner());

    assert!(report.candidates.is_empty());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn candidates_carry_a_suggested_name_and_matched_kinds() {
    let root = temp_tree("names");
    repo_at(&root, "api");

    let report = walk(&request(&root, ScanMode::Quick), &runner());

    let candidate = &report.candidates[0];
    assert_eq!(candidate.suggested_name, "api");
    assert_eq!(candidate.matched_kinds, vec!["git".to_string()]);
    assert!(!candidate.already_tracked);
    assert!(!candidate.disambiguated);

    std::fs::remove_dir_all(&root).ok();
}

/// Name assignment depends on candidate order, so the order has to be
/// stable — otherwise the same tree could produce `work/api` on one run
/// and `code/api` on the next.
#[test]
fn the_same_tree_scanned_twice_produces_the_same_report() {
    let root = temp_tree("determinism");
    repo_at(&root, "zebra");
    repo_at(&root, "alpha");
    repo_at(&root, "middle");

    let first = walk(&request(&root, ScanMode::Quick), &runner());
    let second = walk(&request(&root, ScanMode::Quick), &runner());

    assert_eq!(directories(&first), vec!["alpha", "middle", "zebra"]);
    assert_eq!(directories(&first), directories(&second));

    std::fs::remove_dir_all(&root).ok();
}

/// A root that does not exist is an empty report, not a panic. The user
/// can type a path by hand, and it can vanish between typing and scanning.
#[test]
fn a_missing_root_yields_an_empty_report() {
    let report = walk(
        &request(Path::new("/definitely/not/a/real/path"), ScanMode::Quick),
        &runner(),
    );

    assert!(report.candidates.is_empty());
    assert!(!report.stopped_early);
}

/// The bound that makes a blocking scan acceptable. Tested through
/// `walk_limited` with a tiny limit rather than by building 50,000
/// directories — the behaviour at the ceiling is what matters, not the
/// specific number.
#[test]
fn reaching_the_limit_stops_the_walk_and_says_so() {
    let root = temp_tree("limit");
    plain_at(&root, "a");
    plain_at(&root, "b");
    plain_at(&root, "c");

    let report = walk_limited(&request(&root, ScanMode::Quick), &runner(), 2);

    assert_eq!(report.visited, 2);
    assert!(report.stopped_early);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_walk_that_finishes_is_not_marked_stopped_early() {
    let root = temp_tree("limit-not-hit");
    plain_at(&root, "a");

    let report = walk_limited(&request(&root, ScanMode::Quick), &runner(), 50);

    assert!(!report.stopped_early);

    std::fs::remove_dir_all(&root).ok();
}
