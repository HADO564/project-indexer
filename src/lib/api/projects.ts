import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { CreateProjectInput, Project, SortOptions, UpdateProject } from "./types";

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

export async function getAllProjects(): Promise<Project[]> {
  try {
    return await invoke<Project[]>("get_all_projects");
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

export async function restoreProject(id: string): Promise<Project> {
  try {
    return await invoke<Project>("restore_project", { id });
  } catch (err) {
    throw toError(err);
  }
}
