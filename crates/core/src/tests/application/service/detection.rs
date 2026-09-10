//! Tests for `service/detection.rs`.

use super::*;

#[test]
fn refresh_all_or_nothing_leaves_stored_trackers_on_detector_failure() {
    use crate::detectors::{Detector, DetectorRunner};
    use crate::error::DetectorError;
    struct Boom;
    impl Detector for Boom {
        fn kind(&self) -> &'static str {
            "boom"
        }
        fn detect(&self, _: &std::path::Path) -> Result<Option<Tracker>, DetectorError> {
            Err(DetectorError::Other("boom".into()))
        }
    }
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());
    let svc = ProjectService::new(
        repo.clone(),
        Arc::new(FakeLauncher::default()),
        Arc::new(DetectorRunner::new(vec![Box::new(Boom)])),
        repo,
    );
    let p = svc
        .create("R".into(), tmpdir("refresh"), None, None)
        .unwrap();
    let before = svc.get(&p.id).unwrap().trackers.len();
    assert!(svc.refresh_trackers(&p.id).is_err());
    assert_eq!(svc.get(&p.id).unwrap().trackers.len(), before);
}

#[test]
fn inspect_reports_bad_directory_without_erroring() {
    let svc = service(Arc::new(FakeLauncher::default()));
    let dir = tmpdir("inspect-gone");
    let p = svc.create("I".into(), dir.clone(), None, None).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let ins = svc.inspect(&p.id, None).unwrap();
    assert!(!ins.directory_status.ok);
    assert!(ins.results.is_empty());
}

// ---------------------------------------------------------------- sweep ----

use crate::detectors::git::GitInfo;
use crate::detectors::unreal::UnrealInfo;
use crate::detectors::{Detector, DetectorRunner};
use crate::error::DetectorError;
use std::path::Path;

/// Always matches. Stands in for a detector that existed when the project was
/// first registered.
struct AlwaysGit;
impl Detector for AlwaysGit {
    fn kind(&self) -> &'static str {
        "git"
    }
    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        Ok(Some(Tracker::Git(GitInfo {
            repo_root: path.display().to_string(),
            dirty: false,
            detached_head: false,
            repo_url: None,
            web_url: None,
            contributors: Vec::new(),
            curr_branch: Some("main".into()),
            branches: None,
            commit_hash: None,
        })))
    }
}

/// Always matches. Stands in for the *newly added* detector the sweep exists
/// to backfill.
struct AlwaysUnreal;
impl Detector for AlwaysUnreal {
    fn kind(&self) -> &'static str {
        "unreal"
    }
    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        Ok(Some(Tracker::Unreal(UnrealInfo {
            project_root: path.display().to_string(),
            project_name: "Game".into(),
            uproject_path: format!("{}/Game.uproject", path.display()),
            engine_association: None,
            category: None,
            description: None,
            modules: Vec::new(),
            plugins: Vec::new(),
            vcs_provider: None,
        })))
    }
}

/// Fails on every directory, to exercise the per-project failure path.
struct BrokenUnreal;
impl Detector for BrokenUnreal {
    fn kind(&self) -> &'static str {
        "unreal"
    }
    fn detect(&self, _: &Path) -> Result<Option<Tracker>, DetectorError> {
        Err(DetectorError::Other("detector blew up".into()))
    }
}

/// A service over a caller-supplied repository, so a test can build two
/// services with different detector sets over the *same* database — which is
/// exactly what shipping a new detector looks like.
fn service_over(repo: Arc<SqliteRepository>, detectors: Vec<Box<dyn Detector>>) -> ProjectService {
    ProjectService::new(
        repo.clone(),
        Arc::new(FakeLauncher::default()),
        Arc::new(DetectorRunner::new(detectors)),
        repo,
    )
}

fn only(detector: Box<dyn Detector>) -> ProjectService {
    service_over(
        Arc::new(SqliteRepository::in_memory().unwrap()),
        vec![detector],
    )
}

/// **The one that matters.** Assigning `project.trackers = detection.trackers()`
/// instead of merging would delete the git tracker while adding the unreal one
/// — silent data loss that every other test here would still pass.
#[test]
fn sweep_adds_the_new_tracker_and_keeps_the_existing_ones() {
    let repo = Arc::new(SqliteRepository::in_memory().unwrap());

    // Registered when only the git detector existed.
    let before = service_over(repo.clone(), vec![Box::new(AlwaysGit)]);
    let project = before
        .create("Merged".into(), tmpdir("sweep-merge"), None, None)
        .unwrap();
    assert_eq!(project.trackers.len(), 1);

    // The unreal detector ships; the sweep backfills it.
    let after = service_over(
        repo.clone(),
        vec![Box::new(AlwaysGit), Box::new(AlwaysUnreal)],
    );
    let report = after.redetect_kind("unreal").unwrap();

    assert_eq!(report.updated, 1);
    let stored = after.get(&project.id).unwrap();
    assert_eq!(stored.trackers.len(), 2, "the git tracker must survive");
    assert!(stored.trackers.iter().any(|t| t.is("git")));
    assert!(stored.trackers.iter().any(|t| t.is("unreal")));
}

#[test]
fn sweep_replaces_rather_than_duplicating_a_kind_it_already_has() {
    let svc = only(Box::new(AlwaysUnreal));
    let project = svc
        .create("Twice".into(), tmpdir("sweep-replace"), None, None)
        .unwrap();

    svc.redetect_kind("unreal").unwrap();
    svc.redetect_kind("unreal").unwrap();

    assert_eq!(
        svc.get(&project.id)
            .unwrap()
            .trackers
            .iter()
            .filter(|t| t.is("unreal"))
            .count(),
        1,
        "a second sweep must replace, not append"
    );
}

/// Saving unconditionally would bump `updated_at` on every project on every
/// sweep, silently reordering anyone's recently-updated sort.
#[test]
fn sweep_does_not_resave_a_project_that_did_not_change() {
    let svc = only(Box::new(AlwaysUnreal));
    let project = svc
        .create("Stable".into(), tmpdir("sweep-nochange"), None, None)
        .unwrap();
    let first = svc.get(&project.id).unwrap().updated_at;

    let report = svc.redetect_kind("unreal").unwrap();

    assert_eq!(report.updated, 0, "create already stored this tracker");
    assert_eq!(svc.get(&project.id).unwrap().updated_at, first);
}

#[test]
fn sweep_skips_a_project_whose_directory_is_gone() {
    let svc = only(Box::new(AlwaysUnreal));
    let dir = tmpdir("sweep-vanishes");
    svc.create("Gone".into(), dir.clone(), None, None).unwrap();
    std::fs::remove_dir_all(&dir).unwrap();

    let report = svc.redetect_kind("unreal").unwrap();

    assert_eq!(report.scanned, 1);
    assert_eq!(report.skipped, 1);
    assert_eq!(report.updated, 0);
    assert!(report.failures.is_empty(), "missing is skipped, not failed");
}

#[test]
fn a_failing_detector_is_recorded_and_the_sweep_continues() {
    let svc = only(Box::new(BrokenUnreal));
    svc.create("A".into(), tmpdir("sweep-fail-a"), None, None)
        .unwrap();
    svc.create("B".into(), tmpdir("sweep-fail-b"), None, None)
        .unwrap();

    let report = svc.redetect_kind("unreal").unwrap();

    assert_eq!(report.scanned, 2);
    assert_eq!(report.failures.len(), 2, "both recorded, neither aborted");
    assert_eq!(report.updated, 0);
}

#[test]
fn sweeping_a_kind_no_detector_provides_changes_nothing() {
    let svc = only(Box::new(AlwaysUnreal));
    let project = svc
        .create("Untouched".into(), tmpdir("sweep-unknown"), None, None)
        .unwrap();
    let before = svc.get(&project.id).unwrap().trackers.len();

    let report = svc.redetect_kind("nonexistent").unwrap();

    assert_eq!(report.updated, 0);
    assert_eq!(svc.get(&project.id).unwrap().trackers.len(), before);
}
