import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { StoredIcon } from "./types";

export async function listCustomIcons(): Promise<StoredIcon[]> {
  try {
    return await invoke<StoredIcon[]>("list_custom_icons");
  } catch (err) {
    throw toError(err);
  }
}

// Reads an SVG from `path`, sanitizes it, and stores the sanitized form — the
// original is never copied.
//
// This fails for *content* reasons as often as IO ones: an undefined entity,
// a bare `&`, `&nbsp;` (an HTML entity XML does not define, common in
// HTML-flavoured exports), nothing drawable left after sanitizing, over
// 256 KB, or a backslash in an attribute value. Every one of those names its
// reason in the message and every one is something the user can fix, so
// callers must surface `err.message` rather than a generic failure —
// "this icon could not be imported" tells the user nothing they can act on.
export async function importCustomIcon(path: string): Promise<StoredIcon> {
  try {
    return await invoke<StoredIcon>("import_custom_icon", { path });
  } catch (err) {
    throw toError(err);
  }
}

export async function deleteCustomIcon(name: string): Promise<void> {
  try {
    await invoke<void>("delete_custom_icon", { name });
  } catch (err) {
    throw toError(err);
  }
}
