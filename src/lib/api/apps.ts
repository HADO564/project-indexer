import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { InstalledApp } from "./types";

export async function listInstalledApps(): Promise<InstalledApp[]> {
  try {
    return await invoke<InstalledApp[]>("list_installed_apps");
  } catch (err) {
    throw toError(err);
  }
}
