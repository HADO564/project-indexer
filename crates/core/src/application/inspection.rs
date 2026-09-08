use serde::Serialize;

use crate::detectors::{Detection, DetectorOutcome};
use crate::domain::{Project, Tracker};

/// Read-only snapshot of a project plus a live detection pass. Nothing is
/// persisted — `refresh_trackers` is the write path.
#[derive(Serialize)]
pub struct ProjectInspection {
    pub project: Project,
    pub directory_status: DirectoryState,
    pub results: Vec<DetectorResult>,
}

/// Whether the project's directory is currently usable. When `ok` is false
/// `results` is empty and `message` carries the reason.
#[derive(Serialize)]
pub struct DirectoryState {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// One registered detector's outcome for this project's directory.
#[derive(Serialize)]
pub struct DetectorResult {
    pub kind: String,
    pub status: DetectorStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracker: Option<Tracker>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DetectorStatus {
    Detected,
    NotDetected,
    Failed,
}

pub fn results_from(detection: Detection) -> Vec<DetectorResult> {
    detection
        .outcomes
        .into_iter()
        .map(|outcome| match outcome {
            DetectorOutcome::Detected { kind, tracker } => DetectorResult {
                kind: kind.to_string(),
                status: DetectorStatus::Detected,
                tracker: Some(tracker),
                error: None,
            },
            DetectorOutcome::NotDetected { kind } => DetectorResult {
                kind: kind.to_string(),
                status: DetectorStatus::NotDetected,
                tracker: None,
                error: None,
            },
            DetectorOutcome::Failed { kind, error } => DetectorResult {
                kind: kind.to_string(),
                status: DetectorStatus::Failed,
                tracker: None,
                error: Some(error.to_string()),
            },
        })
        .collect()
}
