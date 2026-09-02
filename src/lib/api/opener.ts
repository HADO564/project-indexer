import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { Project } from "./types";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";

// Distinctive prefix of ProjectError::OpenWithAppMissing's message on the
// Rust side. Errors are plain strings across the Tauri boundary, so this is
// how the UI tells "the configured app is gone" apart from any other open
// failure, to offer a fallback instead of just showing a red banner.
const OPEN_WITH_APP_MISSING_PREFIX =
  "The app associated with this project has been removed or cannot be found";

export function isOpenWithAppMissing(err: unknown): boolean {
  return err instanceof Error && err.message.startsWith(OPEN_WITH_APP_MISSING_PREFIX);
}

export async function openProjectDirectory(id: string): Promise<Project> {
  try {
    return await invoke<Project>("open_project", { id });
  } catch (err) {
    throw toError(err);
  }
}

// Opens a project's directory with the system file explorer, ignoring
// whatever `open_with` app is configured for it.
export async function openProjectInExplorer(id: string): Promise<Project> {
  try {
    return await invoke<Project>("open_project_in_explorer", { id });
  } catch (err) {
    throw toError(err);
  }
}

// Opens a URL in the system browser.
export async function openExternalUrl(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch (err) {
    throw toError(err);
  }
}

// Reveals a file or directory in the system file explorer.
export async function revealPath(path: string): Promise<void> {
  try {
    await revealItemInDir(path);
  } catch (err) {
    throw toError(err);
  }
}
