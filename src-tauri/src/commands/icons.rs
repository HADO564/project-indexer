use std::sync::Arc;

use tauri::State;

use indexer_core::error::ProjectError;
use indexer_core::infra::{IconStore, StoredIcon};

#[tauri::command]
pub fn list_custom_icons(
    store: State<'_, Arc<IconStore>>,
) -> Result<Vec<StoredIcon>, ProjectError> {
    Ok(store.list()?)
}

/// Reads an SVG from `path`, sanitizes it, and stores the result. The original
/// is never copied — only the sanitized form reaches the store.
#[tauri::command]
pub fn import_custom_icon(
    store: State<'_, Arc<IconStore>>,
    path: String,
) -> Result<StoredIcon, ProjectError> {
    Ok(store.import(std::path::Path::new(&path))?)
}

#[tauri::command]
pub fn delete_custom_icon(
    store: State<'_, Arc<IconStore>>,
    name: String,
) -> Result<(), ProjectError> {
    Ok(store.delete(&name)?)
}
