import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { Group, UpdateGroup } from "./types";

export async function listGroups(): Promise<Group[]> {
  try {
    return await invoke<Group[]>("list_groups");
  } catch (err) {
    throw toError(err);
  }
}

// Appends after the last group — creating one never displaces an existing
// entry, because reorderGroups is the only thing that renumbers.
export async function createGroup(name: string, color: string, icon: string): Promise<Group> {
  try {
    return await invoke<Group>("create_group", { name, color, icon });
  } catch (err) {
    throw toError(err);
  }
}

export async function updateGroup(id: string, update: UpdateGroup): Promise<Group> {
  try {
    return await invoke<Group>("update_group", { id, update });
  } catch (err) {
    throw toError(err);
  }
}

// Members become Ungrouped. No project is deleted by this.
export async function deleteGroup(id: string): Promise<void> {
  try {
    await invoke<void>("delete_group", { id });
  } catch (err) {
    throw toError(err);
  }
}

// Rewrites the whole sidebar ordering and returns what was persisted, so the
// caller renders that rather than what it hoped for.
//
// The Rust parameter is `ordered_ids`; Tauri 2 converts a camelCase key, the
// same way `deleteMetadata` reaches `delete_metadata` in projects.ts. If it
// ever arrives as null, pass `ordered_ids` instead — do not change the Rust
// signature to dodge it.
export async function reorderGroups(orderedIds: string[]): Promise<Group[]> {
  try {
    return await invoke<Group[]>("reorder_groups", { orderedIds });
  } catch (err) {
    throw toError(err);
  }
}

// ProjectError::GroupNotFound, surfaced as this exact string. Reachable
// through ordinary use: an edit form open while the group is deleted from the
// group manager produces it. The backend rejects the write and changes
// nothing, so the caller refetches the group list and clears the stale
// selection — retrying is guaranteed to fail the same way. Clearing a group
// (group_id: null) is always allowed and checks nothing.
export function isGroupNotFound(err: unknown): boolean {
  const message = err instanceof Error ? err.message : String(err);
  return /^Group with id '.*' not found$/.test(message);
}
