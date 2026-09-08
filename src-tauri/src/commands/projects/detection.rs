//! Re-running detectors against a project, and previewing a directory.

use std::sync::Arc;

use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::domain::{Project, Tracker};
use indexer_core::error::ProjectError;

/// Re-runs project-type detection (git, and whatever else is registered)
/// against an existing project's directory and persists the result.
///
/// Unlike the best-effort detection in [`create_project`], this is
/// all-or-nothing: any detector failure is returned to the caller and the
/// stored trackers are left untouched. It's an explicit, user-triggered
/// retry, so a half-applied refresh — a persisted tracker set silently
/// missing whatever the failing detector produces — is worse than a visible
/// failure. This is a recorded decision, not incidental; the alternative
/// (persist successes, surface per-detector errors) is documented in
/// `docs/architecture.md` and guarded by a runner test.
#[tauri::command]
pub fn refresh_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    id: String,
) -> Result<Project, ProjectError> {
    service.refresh_trackers(&id)
}

/// Runs detection against a directory that isn't a project yet — nothing is
/// read from or written to the store. Lets the frontend preview what a
/// directory looks like (e.g. to suggest a name from its git remote) before
/// the user commits to [`create_project`].
///
/// Advisory, so it's best-effort: a detector that fails just contributes
/// nothing to the preview rather than failing the whole call.
#[tauri::command]
pub fn detect_project_trackers(
    service: State<'_, Arc<ProjectService>>,
    directory: String,
) -> Vec<Tracker> {
    service.preview_detection(&directory)
}
