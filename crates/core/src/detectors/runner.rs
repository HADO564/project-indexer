use std::path::Path;

use crate::detectors::default_detectors;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

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
    ///
    /// Kept alongside [`inspect_kinds`](Self::inspect_kinds) because the
    /// re-detect sweep genuinely wants exactly one detector, and expressing
    /// that as a one-element slice at every call site reads worse.
    pub fn inspect(&self, path: &Path, only: Option<&str>) -> Detection {
        match only {
            Some(kind) => self.inspect_kinds(path, Some(&[kind])),
            None => self.inspect_kinds(path, None),
        }
    }

    /// Runs the detectors whose [`Detector::kind`] appears in `only`, or every
    /// registered detector when `only` is `None`.
    ///
    /// This is the folder scanner's entry point. The distinction it encodes:
    /// the selection decides **whether a directory is worth registering**, not
    /// what gets recorded about it — a directory that survives the scan is
    /// registered through `create`, which runs every installed detector and
    /// stores every tracker that matched.
    ///
    /// `Some(&[])` runs nothing, which is deliberate: a scan with no detectors
    /// ticked finds nothing rather than everything. An unrecognized kind
    /// matches nothing rather than erroring, so remembered settings naming a
    /// detector that has since been removed degrade to finding less.
    pub fn inspect_kinds(&self, path: &Path, only: Option<&[&str]>) -> Detection {
        let mut outcomes = Vec::new();
        for detector in &self.detectors {
            let kind = detector.kind();
            if only.is_some_and(|kinds| !kinds.contains(&kind)) {
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

    /// The [`Detector::kind`] of every registered detector, in registration
    /// order. Lets a caller present the detector set without knowing which
    /// concrete detectors were compiled in.
    pub fn kinds(&self) -> Vec<String> {
        self.detectors
            .iter()
            .map(|d| d.kind().to_string())
            .collect()
    }
}

impl Default for DetectorRunner {
    /// The detector set the app runs by default — see
    /// [`default_detectors`](crate::detectors::registry::default_detectors).
    fn default() -> Self {
        Self::new(default_detectors())
    }
}
