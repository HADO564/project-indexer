//! Tests for [`crate::detectors::runner`].

use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;
use std::path::Path;

use crate::detectors::git::GitInfo;
use crate::detectors::git::Gitector;
use crate::detectors::runner::*;
use std::path::PathBuf;

fn sample_git_tracker() -> Tracker {
    Tracker::Git(GitInfo {
        repo_root: "/tmp/x".to_string(),
        dirty: false,
        detached_head: false,
        repo_url: None,
        web_url: None,
        contributors: Vec::new(),
        curr_branch: Some("main".to_string()),
        branches: None,
        commit_hash: None,
    })
}

/// `DetectorRunner` must stay usable in Tauri's managed app state
/// (`App::manage`, which requires `Send + Sync + 'static`). The check is
/// the type bound here — if a future detector implementation makes
/// `DetectorRunner` stop being `Send + Sync`, this fails to compile
/// rather than the app finding out at `.manage()`.
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn detector_runner_is_send_and_sync() {
    assert_send_sync::<DetectorRunner>();
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("project-indexer-tests-runner-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("should create temp dir");
    dir
}

#[test]
fn detected_directory_yields_a_detected_outcome() {
    let dir = temp_dir("git-repo");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
    let detection = runner.detect_project(&dir);

    assert!(matches!(
        detection.outcomes.as_slice(),
        [DetectorOutcome::Detected { kind: "git", .. }]
    ));
    assert!(matches!(detection.trackers().as_slice(), [Tracker::Git(_)]));
    assert!(detection.errors().is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn unrecognized_directory_yields_a_not_detected_outcome() {
    let dir = temp_dir("plain");

    let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
    let detection = runner.detect_project(&dir);

    assert!(matches!(
        detection.outcomes.as_slice(),
        [DetectorOutcome::NotDetected { kind: "git" }]
    ));
    assert!(detection.trackers().is_empty());

    std::fs::remove_dir_all(&dir).ok();
}

/// A detector that always errors. Used to prove one detector blowing up
/// doesn't discard the trackers other detectors produced, and doesn't
/// stop the detectors registered after it from running.
struct Boom;
impl Detector for Boom {
    fn kind(&self) -> &'static str {
        "boom"
    }
    fn detect(&self, _path: &Path) -> Result<Option<Tracker>, DetectorError> {
        Err(DetectorError::Other("boom".into()))
    }
}

#[test]
fn one_detector_failing_keeps_the_others_results() {
    let dir = temp_dir("resilient");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Boom), Box::new(Gitector)]);
    let detection = runner.detect_project(&dir);

    assert!(matches!(detection.trackers().as_slice(), [Tracker::Git(_)]));
    assert_eq!(detection.errors().len(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn inspect_runs_only_the_named_detector() {
    let dir = temp_dir("only");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector)]);

    assert_eq!(runner.inspect(&dir, Some("git")).outcomes.len(), 1);
    assert_eq!(runner.inspect(&dir, Some("unreal")).outcomes.len(), 0);
    assert_eq!(runner.inspect(&dir, Some("nonsense")).outcomes.len(), 0);
    assert_eq!(runner.inspect(&dir, None).outcomes.len(), 1);

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn into_result_returns_every_tracker_when_no_detector_failed() {
    let detection = Detection {
        outcomes: vec![DetectorOutcome::Detected {
            kind: "git",
            tracker: sample_git_tracker(),
        }],
    };

    let trackers = detection.into_result().expect("clean detection is Ok");
    assert!(matches!(trackers.as_slice(), [Tracker::Git(_)]));
}

/// The deliberate all-or-nothing contract behind `refresh_project_trackers`
/// (see `Detection::into_result` and `docs/architecture.md`): a partial
/// success is reported as a failure, never half-persisted. If this test is
/// changed, the persistence behaviour is changing — do it on purpose.
#[test]
fn into_result_discards_partial_trackers_on_any_error() {
    let detection = Detection {
        outcomes: vec![
            DetectorOutcome::Detected {
                kind: "git",
                tracker: sample_git_tracker(),
            },
            DetectorOutcome::Failed {
                kind: "unity",
                error: DetectorError::Other("unity detector blew up".into()),
            },
        ],
    };

    assert!(detection.into_result().is_err());
}

#[test]
fn inspect_kinds_runs_only_the_named_detectors() {
    let dir = temp_dir("kinds");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector), Box::new(Boom)]);

    assert_eq!(runner.inspect_kinds(&dir, Some(&["git"])).outcomes.len(), 1);
    assert_eq!(
        runner
            .inspect_kinds(&dir, Some(&["git", "boom"]))
            .outcomes
            .len(),
        2
    );
    assert_eq!(runner.inspect_kinds(&dir, None).outcomes.len(), 2);

    std::fs::remove_dir_all(&dir).ok();
}

/// An empty selection is "no detectors", not "all detectors" — the walk
/// relies on this to treat a scan with nothing ticked as finding nothing,
/// rather than importing the entire disk.
#[test]
fn inspect_kinds_with_an_empty_selection_runs_nothing() {
    let dir = temp_dir("kinds-empty");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
    assert_eq!(runner.inspect_kinds(&dir, Some(&[])).outcomes.len(), 0);

    std::fs::remove_dir_all(&dir).ok();
}

/// Remembered settings can name a detector that no longer exists. That must
/// degrade to finding less, never to an error.
#[test]
fn inspect_kinds_ignores_unknown_kinds() {
    let dir = temp_dir("kinds-unknown");
    git2::Repository::init(&dir).expect("should init a git repo");

    let runner = DetectorRunner::new(vec![Box::new(Gitector)]);

    assert_eq!(
        runner
            .inspect_kinds(&dir, Some(&["nonsense"]))
            .outcomes
            .len(),
        0
    );
    assert_eq!(
        runner
            .inspect_kinds(&dir, Some(&["nonsense", "git"]))
            .outcomes
            .len(),
        1
    );

    std::fs::remove_dir_all(&dir).ok();
}
