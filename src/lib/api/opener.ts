import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { Project } from "./types";

export async function openProjectDirectory(id: string): Promise<Project> {
  try {
    return await invoke<Project>("open_project", { id });
  } catch (err) {
    throw toError(err);
  }
}
