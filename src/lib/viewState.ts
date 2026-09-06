import type { Group } from "./api/types";
import { parseViewKey, viewKey, type View } from "./views";

// The selected view and the view mode persist; sort does not, which is how it
// already behaves. They live in localStorage rather than projects.db because
// the database is a cross-app contract devmon attaches read-only, and UI
// preferences are none of its business.
//
// Persisting the selection is only safe because the sidebar is legible: the
// current selection and every other view are always on screen, so restoring
// one cannot surprise anyone. That is precisely why collapse state could not
// be persisted in the rejected sectioned design.
export type ViewMode = "list" | "grid" | "compact";

const MODES: readonly ViewMode[] = ["list", "grid", "compact"];

const MODE_KEY = "pi.viewMode";
const VIEW_KEY = "pi.view";

// Pure. `raw` is whatever was in storage — possibly written by an older build,
// possibly hand-edited, possibly absent.
export function restoreViewMode(raw: string | null): ViewMode {
  return MODES.includes(raw as ViewMode) ? (raw as ViewMode) : "list";
}

// Pure. Two rules the spec names explicitly:
//  - a view whose group has since been deleted falls back to All, rather than
//    rendering an empty list with nothing saying why;
//  - Bin is never restored as the landing view. It is a destination you go to,
//    not a place to start.
export function restoreView(raw: string | null, groups: Group[]): View {
  const view = parseViewKey(raw);
  if (!view) return { kind: "all" };
  if (view.kind === "bin") return { kind: "all" };
  if (view.kind === "group" && !groups.some((g) => g.id === view.id)) return { kind: "all" };
  return view;
}

// Storage access, split from the rules above so the rules are testable in
// vitest's node environment, where there is no localStorage. Guarded because a
// browser with site data blocked throws on the accessor itself.
function read(key: string): string | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(key, value);
  } catch {
    // A preference that cannot be saved is not worth an error banner.
  }
}

export function loadViewMode(): ViewMode {
  return restoreViewMode(read(MODE_KEY));
}

export function saveViewMode(mode: ViewMode): void {
  write(MODE_KEY, mode);
}

export function loadView(groups: Group[]): View {
  return restoreView(read(VIEW_KEY), groups);
}

export function saveView(view: View): void {
  write(VIEW_KEY, viewKey(view));
}
