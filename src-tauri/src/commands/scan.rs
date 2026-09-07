use std::sync::Arc;

use tauri::State;

use indexer_core::application::{ImportReport, ImportSelection, ScanService};
use indexer_core::domain::scan::{ScanReport, ScanRequest};
use indexer_core::error::ProjectError;

/// Walks a folder looking for projects the ticked detectors recognize, and
/// returns what it found **without registering anything**. The review step is
/// the point: registering a project is a durable act, and a bulk one should be
/// a deliberate one.
///
/// `async` so the walk runs on Tauri's pool and the window stays responsive.
/// It is blocking from the frontend's point of view — there are no progress
/// events and no cancellation, which is acceptable only because the walk is
/// bounded by `MAX_DIRECTORIES`; `stopped_early` on the report says when that
/// bound was hit.
#[tauri::command]
pub async fn scan_folder(
    scan: State<'_, Arc<ScanService>>,
    request: ScanRequest,
) -> Result<ScanReport, ProjectError> {
    scan.scan(&request)
}

/// Registers the rows the user kept, using the names they left in the review
/// list. Best-effort per row: a directory that fails is reported in
/// `failures` and the rest still land.
#[tauri::command]
pub async fn import_scanned(
    scan: State<'_, Arc<ScanService>>,
    selections: Vec<ImportSelection>,
) -> Result<ImportReport, ProjectError> {
    scan.import(&selections)
}

/// The detector kinds this build registers, so the scan form's tick-list is
/// built from what the binary actually has rather than a hardcoded array —
/// invariant 1, "a new detector is implement + register, zero frontend code".
#[tauri::command]
pub fn list_detector_kinds(scan: State<'_, Arc<ScanService>>) -> Vec<String> {
    scan.detector_kinds()
}
