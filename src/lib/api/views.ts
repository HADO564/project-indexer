import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { View, ViewCounts, ViewMatches } from "./types";

// Which projects `view` shows under `query`, decided by core so the GUI and
// the CLI search the same way. Runs on every keystroke, so it returns ids
// only — filter the list already on screen by them.
export async function resolveView(view: View, query: string): Promise<ViewMatches> {
  try {
    return await invoke<ViewMatches>("resolve_view", { view, query });
  } catch (err) {
    throw toError(err);
  }
}

// How many projects each sidebar view holds, ignoring any search.
export async function viewCounts(): Promise<ViewCounts> {
  try {
    return await invoke<ViewCounts>("view_counts");
  } catch (err) {
    throw toError(err);
  }
}

// Every property name in use across live and binned projects, folded to lower
// case and sorted.
export async function propertyKeys(): Promise<string[]> {
  try {
    return await invoke<string[]>("property_keys");
  } catch (err) {
    throw toError(err);
  }
}
