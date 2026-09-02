use std::sync::Arc;

use tauri::State;

use indexer_core::application::{ProjectInspection, ProjectService};
use indexer_core::error::ProjectError;

/// Loads a project and runs detection against its directory **without
/// persisting**. A missing/inaccessible directory is reported via
/// `directory_status` (with empty `results`), not as a command error, so the
/// view can still render the project's identity. `only = Some(kind)` re-runs
/// just that one detector.
#[tauri::command]
pub fn inspect_project(
    service: State<'_, Arc<ProjectService>>,
    id: String,
    only: Option<String>,
) -> Result<ProjectInspection, ProjectError> {
    service.inspect(&id, only.as_deref())
}
