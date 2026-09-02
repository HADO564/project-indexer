use std::path::Path;

use tauri::{AppHandle, State};

use crate::store::ProjectStore;
#[allow(unused_imports)]
use indexer_core::application::inspection::{
    results_from, DetectorResult, DetectorStatus, DirectoryState, ProjectInspection,
};
use indexer_core::detectors::DetectorRunner;
use indexer_core::domain::Project;
use indexer_core::error::ProjectError;

/// Loads a project and runs detection against its directory **without
/// persisting**. A missing/inaccessible directory is reported via
/// `directory_status` (with empty `results`), not as a command error, so the
/// view can still render the project's identity. `only = Some(kind)` re-runs
/// just that one detector.
///
/// The orchestration here is lifted verbatim from the pre-`indexer-core`
/// command; Task 7 replaces the body with `service.inspect(&id, only)`.
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
                DirectoryState {
                    ok: true,
                    message: None,
                },
                results_from(detection),
            )
        }
        Err(error) => (
            DirectoryState {
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
