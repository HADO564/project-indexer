use std::path::Path;

use crate::detectors::detector::Detector;
use crate::detectors::git::Gitector;
use crate::detectors::unreal::UnrealDetector;
use crate::errors::DetectorError;
use crate::models::tracker::Tracker;

/// Runs every registered [`Detector`] against a path and collects whichever
/// ones recognize it. Detectors are held as `Box<dyn Detector>` rather than a
/// fixed set of concrete types, so registering Unity/Godot/Unreal/MATLAB
/// support later is a matter of pushing another detector into the list, not
/// touching this orchestration.
pub struct DetectorRunner {
    detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRunner {
    pub fn new(detectors: Vec<Box<dyn Detector>>) -> Self {
        Self { detectors }
    }

    /// Runs `path` through every registered detector, collecting a
    /// [`Tracker`] from each one that recognizes it. A detector that doesn't
    /// recognize `path` contributes nothing — matching none of the
    /// registered detectors is a normal outcome, not an error.
    pub fn detect(&self, path: &Path) -> Result<Vec<Tracker>, DetectorError> {
        let mut trackers = Vec::new();
        for detector in &self.detectors {
            if let Some(tracker) = detector.get_info(path)? {
                trackers.push(tracker);
            }
        }
        Ok(trackers)
    }
}

impl Default for DetectorRunner {
    /// The detector set the app runs by default. Register a new detector's
    /// `Box::new(...)` here as it's built.
    fn default() -> Self {
        Self::new(vec![Box::new(Gitector), Box::new(UnrealDetector)])
    }
}

/// Runs `path` through the app's full default detector set. A convenience
/// wrapper around [`DetectorRunner::default`] for the common one-off case;
/// build a [`DetectorRunner`] directly to control which detectors run (as
/// tests do, to exercise one detector in isolation).
pub fn detect_project(path: &Path) -> Result<Vec<Tracker>, DetectorError> {
    DetectorRunner::default().detect(path)
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let trackers = runner.detect(&dir).expect("detection should succeed");

        assert!(matches!(trackers.as_slice(), [Tracker::Git(_)]));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn returns_no_trackers_for_a_directory_no_detector_recognizes() {
        let dir = temp_dir("plain");

        let runner = DetectorRunner::new(vec![Box::new(Gitector)]);
        let trackers = runner.detect(&dir).expect("detection should succeed");

        assert!(trackers.is_empty());

        std::fs::remove_dir_all(&dir).ok();
    }
}
