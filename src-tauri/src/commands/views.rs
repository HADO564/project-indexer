//! The sidebar's views, their counts and the search box, answered by
//! `indexer_core::domain::views` so the GUI and the CLI share one set of rules.
//!
//! Each command reads the current lists and hands them to core. Ordering is
//! not applied here: the frontend filters the list it already holds, sorted,
//! by the ids these return.

use std::sync::Arc;

use serde::Serialize;
use tauri::State;

use indexer_core::application::{GroupService, ProjectService};
use indexer_core::domain::sorting::SortOptions;
use indexer_core::domain::views::{self, View, ViewCounts};
use indexer_core::error::ProjectError;

/// What the search box asked for: the ids `view` shows under `query`, and
/// whether `query` used the `name: value` syntax, so the hint can confirm it
/// without a second round trip per keystroke.
#[derive(Serialize)]
pub struct ViewMatches {
    ids: Vec<String>,
    property_query: bool,
}

/// Ids rather than projects: the frontend already holds every project, and
/// this runs on every keystroke.
#[tauri::command]
pub fn resolve_view(
    service: State<'_, Arc<ProjectService>>,
    view: View,
    query: String,
) -> Result<ViewMatches, ProjectError> {
    let live = service.list(SortOptions::default())?;
    let deleted = service.list_deleted(SortOptions::default())?;
    let ids = views::resolve_view(&view, &live, &deleted, &query)
        .into_iter()
        .map(|p| p.id.clone())
        .collect();
    Ok(ViewMatches {
        ids,
        property_query: views::is_property_query(&query),
    })
}

/// How many projects each sidebar view holds, ignoring any search.
#[tauri::command]
pub fn view_counts(
    projects: State<'_, Arc<ProjectService>>,
    groups: State<'_, Arc<GroupService>>,
) -> Result<ViewCounts, ProjectError> {
    let live = projects.list(SortOptions::default())?;
    let deleted = projects.list_deleted(SortOptions::default())?;
    Ok(views::view_counts(&live, &deleted, &groups.list()?))
}

/// Every property name in use, live or binned, for the search hint and the
/// forms' name suggestions.
#[tauri::command]
pub fn property_keys(service: State<'_, Arc<ProjectService>>) -> Result<Vec<String>, ProjectError> {
    let live = service.list(SortOptions::default())?;
    let deleted = service.list_deleted(SortOptions::default())?;
    Ok(views::property_keys(live.iter().chain(&deleted)))
}
