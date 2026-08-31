use std::path::Path;

use crate::detectors::detector::Detector;
use crate::detectors::registry::default_detectors;
use crate::errors::DetectorError;
use crate::models::tracker::Tracker;

/// The outcome of running every registered detector against a path.
///
/// Detectors are isolated from one another: a detector that errors doesn't
/// discard the trackers other detectors already produced. Best-effort callers
/// take [`trackers`](Self::trackers) and log [`errors`](Self::errors); a
/// caller that needs detection to be all-or-nothing uses
/// [`into_result`](Self::into_result).
#[derive(Debug, Default)]
pub struct Detection {
    /// One entry per detector that recognized the path, in registration order.
    pub trackers: Vec<Tracker>,
    /// One entry per detector that failed to inspect the path. A detector that
    /// simply didn't recognize the path is neither here nor in `trackers`.
    pub errors: Vec<DetectorError>,
}

impl Detection {
    /// `Ok(trackers)` only if every detector ran cleanly; otherwise `Err` with
    /// the first failure and the partial trackers discarded.
    ///
    /// For a persisted, user-triggered detection run where a half-applied
    /// result would be more confusing than a clear failure.
    pub fn into_result(self) -> Result<Vec<Tracker>, DetectorError> {
        match self.errors.into_iter().next() {
            Some(error) => Err(error),
            None => Ok(self.trackers),
        }
    }
}

/// Runs every registered [`Detector`] against a path and collects whichever
/// ones recognize it. Detectors are held as `Box<dyn Detector>` rather than a
/// fixed set of concrete types, so registering Unity/Godot/MATLAB support
/// later is a matter of adding another detector to
/// [`default_detectors`](crate::detectors::registry::default_detectors), not
/// touching this orchestration.
///
/// The app builds one of these at startup and keeps it in Tauri's managed
/// state (`App::manage`); commands pull it back out with `State<DetectorRunner>`
/// rather than constructing detectors per call.
pub struct DetectorRunner {
    detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRunner {
    pub fn new(detectors: Vec<Box<dyn Detector>>) -> Self {
        Self { detectors }
    }

    /// The canonical detection operation: run `path` through every registered
    /// detector and collect a [`Tracker`] from each one that recognizes it.
    ///
    /// Infallible by construction — a detector hitting a genuine problem
    /// (an unreadable directory, a corrupt repo) lands in
    /// [`Detection::errors`] without stopping the others or losing the
    /// trackers that already succeeded. Recognizing no detectors at all is a
    /// normal outcome: an empty [`Detection`].
    pub fn detect_project(&self, path: &Path) -> Detection {
        let mut detection = Detection::default();
        for detector in &self.detectors {
            match detector.detect(path) {
                Ok(Some(tracker)) => detection.trackers.push(tracker),
                Ok(None) => {}
                Err(error) => detection.errors.push(error),
            }
        }
        detection
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
    use std::path::PathBuf;

    /// `DetectorRunner` must stay usable in Tauri's managed app state
    /// (`App::manage`, which requires `Send + Sync + 'static`). The check
    /// itself is the type bound on `assert_send_sync` — if a future detector
    /// implementation makes `DetectorRunner` stop being `Send + Sync`, this
    /// fails to compile rather than the app finding out at `.manage()`.
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
    fn collects_a_tracker_from_a_recognized_directory() {
        let dir = temp_dir("git-repo");
        git2::Repository::init(&dir).expect("should init a git repo");

        let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
        let detection = runner.detect_project(&dir);

        assert!(matches!(detection.trackers.as_slice(), [Tracker::Git(_)]));
        assert!(detection.errors.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn returns_no_trackers_for_a_directory_no_detector_recognizes() {
        let dir = temp_dir("plain");

        let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
        let detection = runner.detect_project(&dir);

        assert!(detection.trackers.is_empty());
        assert!(detection.errors.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A detector blowing up must not discard trackers other detectors
    /// produced, and must not stop later detectors from running.
    struct Boom;
    impl Detector for Boom {
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

        assert!(matches!(detection.trackers.as_slice(), [Tracker::Git(_)]));
        assert_eq!(detection.errors.len(), 1);

        // ...and the all-or-nothing view still reports the failure.
        assert!(detection.into_result().is_err());

        std::fs::remove_dir_all(&dir).ok();
    }
}
