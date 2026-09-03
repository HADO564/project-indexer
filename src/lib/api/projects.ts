import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { CreateProjectInput, Project, ProjectInspection, SortOptions, UpdateProject } from "./types";

export async function createProject(input: CreateProjectInput): Promise<Project> {
  try {
    return await invoke<Project>("create_project", {
      name: input.name,
      directory: input.directory,
      description: input.description ?? null,
      tags: input.tags ?? null,
    });
  } catch (err) {
    throw toError(err);
  }
}

export async function getProject(id: string): Promise<Project> {
  try {
    return await invoke<Project>("get_project", { id });
  } catch (err) {
    throw toError(err);
  }
}

export async function getAllProjects(options?: SortOptions): Promise<Project[]> {
  try {
    return await invoke<Project[]>("get_all_projects", { options: options ?? null });
  } catch (err) {
    throw toError(err);
  }
}

// IDs of live projects whose directory is gone from disk (deleted or moved).
// Inaccessible directories (offline drive, permissions) are not included.
export async function listMissingDirectories(): Promise<string[]> {
  try {
    return await invoke<string[]>("list_missing_directories");
  } catch (err) {
    throw toError(err);
  }
}

export async function getDeletedProjects(options?: SortOptions): Promise<Project[]> {
  try {
    return await invoke<Project[]>("get_deleted_projects", { options: options ?? null });
  } catch (err) {
    throw toError(err);
  }
}

export async function getFavoriteProjects(options?: SortOptions): Promise<Project[]> {
  try {
    return await invoke<Project[]>("get_favorite_projects", { options: options ?? null });
  } catch (err) {
    throw toError(err);
  }
}

export async function updateProject(id: string, update: UpdateProject): Promise<Project> {
  try {
    return await invoke<Project>("update_project", { id, update });
  } catch (err) {
    throw toError(err);
  }
}

// Permanently purges a project's metadata. Only meant to be used on an
// already soft-deleted project (from the bin) — see `deleteProjectDirectory`.
export async function deleteProject(id: string): Promise<void> {
  try {
    await invoke<void>("delete_project", { id });
  } catch (err) {
    throw toError(err);
  }
}

// Deletes a project's directory from disk. When `deleteMetadata` is false
// the project's tracked metadata is kept (soft-deleted, shows up in the
// bin); when true it's purged immediately.
export async function deleteProjectDirectory(
  id: string,
  deleteMetadata: boolean,
): Promise<void> {
  try {
    await invoke<void>("delete_project_directory", { id, deleteMetadata });
  } catch (err) {
    throw toError(err);
  }
}

// Removes a project's tracked metadata without touching its directory on
// disk — "stop indexing this," not "delete it." Works on any project, not
// just ones already in the bin; re-adding it later is just pointing
// createProject at the same directory again.
export async function untrackProject(id: string): Promise<void> {
  try {
    await invoke<void>("untrack_project", { id });
  } catch (err) {
    throw toError(err);
  }
}

export async function restoreProject(id: string): Promise<Project> {
  try {
    return await invoke<Project>("restore_project", { id });
  } catch (err) {
    throw toError(err);
  }
}

// Re-runs project-type detection (git, and whatever else is registered)
// against the project's directory and persists the result. Unlike creation
// (where detection is best-effort and failures are swallowed), a failure
// here is a real error — this is an explicit, user-triggered retry.
export async function refreshProjectTrackers(id: string): Promise<Project> {
  try {
    return await invoke<Project>("refresh_project_trackers", { id });
  } catch (err) {
    throw toError(err);
  }
}

// Read-only: loads a project and runs a live detection pass without
// persisting. `only` re-runs a single detector by kind. Backs /project/[id].
export async function inspectProject(
  id: string,
  opts?: { only?: string },
): Promise<ProjectInspection> {
  try {
    return await invoke<ProjectInspection>("inspect_project", {
      id,
      only: opts?.only ?? null,
    });
  } catch (err) {
    throw toError(err);
  }
}

// Backend-computed project-name suggestion for a picked directory: the git
// remote's repo name if the dir is a repo with a remote, else the folder
// name; null when neither. Best-effort — only throws on IPC failure.
export async function suggestProjectName(directory: string): Promise<string | null> {
  try {
    return await invoke<string | null>("suggest_project_name", { directory });
  } catch (err) {
    throw toError(err);
  }
}
