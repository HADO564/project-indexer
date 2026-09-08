//! The list endpoints behind each sidebar view.

use std::sync::Arc;

use tauri::State;

use indexer_core::application::ProjectService;
use indexer_core::domain::sorting::SortOptions;
use indexer_core::domain::Project;
use indexer_core::error::ProjectError;

/// Returns non-deleted projects for the main list view, ordered per
/// `options` (default: alphabetical, ascending — see [`SortOptions::default`]).
#[tauri::command]
pub fn get_all_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list(options.unwrap_or_default())
}

/// Returns soft-deleted projects for the bin view, ordered per `options`
/// (default: alphabetical, ascending — see [`SortOptions::default`]).
#[tauri::command]
pub fn get_deleted_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list_deleted(options.unwrap_or_default())
}

/// Returns favorited, non-deleted projects, ordered per `options` (default:
/// alphabetical, ascending).
#[tauri::command]
pub fn get_favorite_projects(
    service: State<'_, Arc<ProjectService>>,
    options: Option<SortOptions>,
) -> Result<Vec<Project>, ProjectError> {
    service.list_favorites(options.unwrap_or_default())
}
