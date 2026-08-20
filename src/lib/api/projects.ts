import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { CreateProjectInput, Project, UpdateProject } from "./types";

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

export async function updateProject(id: string, update: UpdateProject): Promise<Project> {
  try {
    return await invoke<Project>("update_project", { id, update });
  } catch (err) {
    throw toError(err);
  }
}

export async function deleteProject(id: string): Promise<void> {
  try {
    await invoke<void>("delete_project", { id });
  } catch (err) {
    throw toError(err);
  }
}
