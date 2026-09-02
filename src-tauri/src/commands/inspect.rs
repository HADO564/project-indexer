use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::detectors::{Detection, DetectorOutcome, DetectorRunner};
use crate::errors::ProjectError;
use crate::models::{Project, Tracker};
use crate::store::ProjectStore;

/// Read-only snapshot of a project plus a live detection pass. Nothing is
/// persisted — `refresh_project_trackers` is the write path.
#[derive(Serialize)]
pub struct ProjectInspection {
    pub project: Project,
    pub directory_status: DirectoryStatusDto,
    pub results: Vec<DetectorResult>,
}

/// Whether the project's directory is currently usable. When `ok` is false
/// `results` is empty and `message` carries the reason.
#[derive(Serialize)]
pub struct DirectoryStatusDto {
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

fn results_from(detection: Detection) -> Vec<DetectorResult> {
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

/// Loads a project and runs detection against its directory **without
/// persisting**. A missing/inaccessible directory is reported via
/// `directory_status` (with empty `results`), not as a command error, so the
/// view can still render the project's identity. `only = Some(kind)` re-runs
/// just that one detector.
#[tauri::command]
pub fn inspect_project(
    app: AppHandle,
    detectors: State<'_, DetectorRunner>,
    id: String,
    only: Option<String>,
) -> Result<ProjectInspection, ProjectError> {
    let store = ProjectStore::new(&app)?;
    let project = store
        .get_project(&id)?
        .ok_or_else(|| ProjectError::NotFound(id.clone()))?;

    let (directory_status, results) = match Project::check_directory_health(&project.directory) {
        Ok(()) => {
            let detection = detectors.inspect(Path::new(&project.directory), only.as_deref());
            (
                DirectoryStatusDto {
                    ok: true,
                    message: None,
                },
                results_from(detection),
            )
        }
        Err(error) => (
            DirectoryStatusDto {
                ok: false,
                message: Some(error.to_string()),
            },
            Vec::new(),
        ),
    };

    Ok(ProjectInspection {
        project,
        directory_status,
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detectors::{Detection, DetectorOutcome};
    use crate::errors::DetectorError;
    use crate::models::git::GitInfo;
    use crate::models::tracker::Tracker;

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
