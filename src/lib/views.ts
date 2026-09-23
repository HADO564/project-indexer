import type { Group, View } from "./api/types";

// How a view is written down and what it is called on screen — the two parts
// of views that belong to the browser. Which projects a view holds, its
// counts and the search now live in core (crates/core/src/domain/views.rs),
// reached through $lib/api/views, so the GUI and the CLI cannot drift apart
// on what "Favourites" or `client: acme` means.

const GROUP_PREFIX = "group:";

// A stable string form, for localStorage and for comparing selections.
export function viewKey(view: View): string {
  return view.kind === "group" ? `${GROUP_PREFIX}${view.id}` : view.kind;
}

// Returns null rather than throwing for anything unrecognised — the input is
// whatever was in localStorage, which a previous build or a hand-edit may
// have written. The caller decides the fallback.
export function parseViewKey(key: string | null | undefined): View | null {
  if (typeof key !== "string") return null;
  if (key === "all" || key === "favorites" || key === "ungrouped" || key === "bin") {
    return { kind: key };
  }
  if (key.startsWith(GROUP_PREFIX)) {
    // slice, not split, so a group id containing a colon survives.
    const id = key.slice(GROUP_PREFIX.length);
    return id.length > 0 ? { kind: "group", id } : null;
  }
  return null;
}

export function viewLabel(view: View, groups: Group[]): string {
  switch (view.kind) {
    case "all":
      return "All";
    case "favorites":
      return "Favourites";
    case "ungrouped":
      return "Ungrouped";
    case "bin":
      return "Bin";
    case "group":
      return groups.find((g) => g.id === view.id)?.name ?? "Unknown group";
  }
}
