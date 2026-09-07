import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { ImportReport, ImportSelection, ScanReport, ScanRequest } from "./types";

// Walks a folder and reports what it found. Registers nothing — the review
// step between this and importScanned is the whole point of the feature.
export async function scanFolder(request: ScanRequest): Promise<ScanReport> {
  try {
    return await invoke<ScanReport>("scan_folder", { request });
  } catch (err) {
    throw toError(err);
  }
}

// Registers the reviewed rows. Best-effort per row: check `failures` on the
// result rather than expecting this to throw for one bad directory.
export async function importScanned(selections: ImportSelection[]): Promise<ImportReport> {
  try {
    return await invoke<ImportReport>("import_scanned", { selections });
  } catch (err) {
    throw toError(err);
  }
}

// The detector kinds this build registers. The scan form's tick-list is built
// from this, never a hardcoded array — a new detector must cost zero frontend
// code.
export async function listDetectorKinds(): Promise<string[]> {
  try {
    return await invoke<string[]>("list_detector_kinds");
  } catch (err) {
    throw toError(err);
  }
}
