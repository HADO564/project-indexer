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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::{Detection, DetectorOutcome};
    use crate::domain::git::GitInfo;
    use crate::domain::tracker::Tracker;
    use crate::error::DetectorError;

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

    #[test]
    fn results_from_maps_every_outcome_variant() {
        let detection = Detection {
            outcomes: vec![
                DetectorOutcome::Detected {
                    kind: "git",
                    tracker: sample_git_tracker(),
                },
                DetectorOutcome::NotDetected { kind: "unreal" },
                DetectorOutcome::Failed {
                    kind: "unity",
                    error: DetectorError::Other("boom".into()),
                },
            ],
        };

        let results = results_from(detection);

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].kind, "git");
        assert!(matches!(results[0].status, DetectorStatus::Detected));
        assert!(results[0].tracker.is_some());
        assert!(matches!(results[1].status, DetectorStatus::NotDetected));
        assert!(results[1].tracker.is_none());
        assert!(matches!(results[2].status, DetectorStatus::Failed));
        assert_eq!(results[2].error.as_deref(), Some("boom"));
    }
}
