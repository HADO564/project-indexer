use std::path::Path;

use crate::detectors::detector::Detector;
use crate::detectors::registry::default_detectors;
use crate::errors::DetectorError;
use crate::models::tracker::Tracker;

/// The outcome of running the registered detectors against a path: one
/// [`DetectorOutcome`] per detector consulted, in registration order.
///
/// Detectors are isolated from one another — a detector that errors is
/// recorded as [`DetectorOutcome::Failed`] without discarding what the others
/// produced. Best-effort callers use [`trackers`](Self::trackers) /
/// [`errors`](Self::errors); a caller that needs all-or-nothing uses
/// [`into_result`](Self::into_result).
#[derive(Debug, Default)]
pub struct Detection {
    pub outcomes: Vec<DetectorOutcome>,
}

/// What one detector reported for one path.
#[derive(Debug)]
pub enum DetectorOutcome {
    /// The detector recognized the path.
    Detected {
        kind: &'static str,
        tracker: Tracker,
    },
    /// The detector ran cleanly and did not recognize the path — a normal
    /// outcome, not a failure.
    NotDetected { kind: &'static str },
    /// The detector hit a genuine problem inspecting the path.
    Failed {
        kind: &'static str,
        error: DetectorError,
    },
}

impl Detection {
    /// Trackers from the detectors that matched, in registration order.
    pub fn trackers(&self) -> Vec<Tracker> {
        self.outcomes
            .iter()
            .filter_map(|o| match o {
                DetectorOutcome::Detected { tracker, .. } => Some(tracker.clone()),
                _ => None,
            })
            .collect()
    }

    /// Errors from the detectors that failed.
    pub fn errors(&self) -> Vec<&DetectorError> {
        self.outcomes
            .iter()
            .filter_map(|o| match o {
                DetectorOutcome::Failed { error, .. } => Some(error),
                _ => None,
            })
            .collect()
    }

    /// The all-or-nothing view: `Ok(trackers)` only if no detector failed,
    /// otherwise `Err` with the first failure and the partial trackers
    /// **discarded**.
    ///
    /// Deliberate domain decision (see `docs/architecture.md`). Detection
    /// results are persisted verbatim, so `refresh_project_trackers` — an
    /// explicit, user-triggered "re-scan everything" — either fully succeeds
    /// or changes nothing: a stored tracker set silently missing whatever the
    /// failing detector would have produced is worse than a visible "refresh
    /// failed, try again".
    ///
    /// The alternative, once detectors are numerous and truly independent, is
    /// to persist the successes and surface per-detector errors separately.
    /// That's a real change with UI implications — make it on purpose.
    /// `into_result_discards_partial_trackers_on_any_error` guards this.
    pub fn into_result(self) -> Result<Vec<Tracker>, DetectorError> {
        let mut trackers = Vec::new();
        for outcome in self.outcomes {
            match outcome {
                DetectorOutcome::Detected { tracker, .. } => trackers.push(tracker),
                DetectorOutcome::NotDetected { .. } => {}
                DetectorOutcome::Failed { error, .. } => return Err(error),
            }
        }
        Ok(trackers)
    }
}

/// Runs the registered [`Detector`]s against a path. Detectors are held as
/// `Box<dyn Detector>` rather than concrete types, so registering
/// Unity/Godot/MATLAB support later is a matter of adding one to
/// [`default_detectors`](crate::detectors::registry::default_detectors).
///
/// The app builds one of these at startup into Tauri managed state
/// (`App::manage`); commands pull it out with `State<DetectorRunner>`.
pub struct DetectorRunner {
    detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRunner {
    pub fn new(detectors: Vec<Box<dyn Detector>>) -> Self {
        Self { detectors }
    }

    /// The canonical detection operation: run `path` through every registered
    /// detector. Infallible by construction — see [`Detection`].
    pub fn detect_project(&self, path: &Path) -> Detection {
        self.inspect(path, None)
    }

    /// Like [`detect_project`](Self::detect_project), but when `only` is
    /// `Some(kind)` only the detector whose [`Detector::kind`] equals `kind`
    /// runs (for per-tracker re-detect). An unknown `kind` matches nothing
    /// and yields an empty [`Detection`].
    pub fn inspect(&self, path: &Path, only: Option<&str>) -> Detection {
        let mut outcomes = Vec::new();
        for detector in &self.detectors {
            let kind = detector.kind();
            if only.is_some_and(|k| k != kind) {
                continue;
            }
            outcomes.push(match detector.detect(path) {
                Ok(Some(tracker)) => DetectorOutcome::Detected { kind, tracker },
                Ok(None) => DetectorOutcome::NotDetected { kind },
                Err(error) => DetectorOutcome::Failed { kind, error },
            });
        }
        Detection { outcomes }
    }
}

impl Default for DetectorRunner {
    /// The detector set the app runs by default — see
    /// [`default_detectors`](crate::detectors::registry::default_detectors).
    fn default() -> Self {
        Self::new(default_detectors())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::git::Gitector;
    use crate::models::git::GitInfo;
    use std::path::PathBuf;

    fn sample_git_tracker() -> Tracker {
        Tracker::Git(GitInfo {
            repo_root: "/tmp/x".to_string(),
            dirty: false,
            detached_head: false,
            repo_url: None,
            contributors: Vec::new(),
            curr_branch: Some("main".to_string()),
            branches: None,
            commit_hash: None,
        })
    }

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
}
