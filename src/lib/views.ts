import type { Group, Project } from "./api/types";

// The sidebar selects a View, which is a data source plus an action set.
// Groups are navigation, not sections: selecting one changes what the main
// list shows, and the current selection stays visible. There is deliberately
// no collapsible-band state anywhere in this design — a collapsed band hides
// projects with nothing on screen saying they exist, which is why the
// sectioned design was rejected.
export type View =
  | { kind: "all" }
  | { kind: "favorites" }
  | { kind: "group"; id: string }
  | { kind: "ungrouped" }
  | { kind: "bin" };

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

// Name, path and tags — the three things a row shows that identify a project.
// An empty query matches everything so the caller does not special-case it.
export function matchesQuery(project: Project, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return true;
  if (project.name.toLowerCase().includes(q)) return true;
  if (project.directory.toLowerCase().includes(q)) return true;
  return project.tags.some((tag) => tag.toLowerCase().includes(q));
}

// `live` is get_all_projects (non-deleted, already sorted by the backend) and
// `deleted` is get_deleted_projects. Filtering is order-preserving, so
// whatever sort the backend applied still holds inside every view — which is
// why this module does not sort. Re-implementing core::domain::sorting in
// TypeScript would be a second unchecked cross-language mirror, the drift
// hazard api/types.ts is already flagged for.
//
// Favourites is derived from `live` rather than fetched: list_favorites
// filters !is_deleted && favorite and then sorts, and every comparator in
// sorting.rs ends in the unique id, so filtering a sorted list and sorting a
// filtered list are the same list. Deriving it also means the sidebar count
// and the list can never disagree.
export function resolveView(
  view: View,
  live: Project[],
  deleted: Project[],
  query: string,
): Project[] {
  const source =
    view.kind === "bin"
      ? deleted
      : view.kind === "favorites"
        ? live.filter((p) => p.favorite)
        : view.kind === "ungrouped"
          ? live.filter((p) => p.group_id === null)
          : view.kind === "group"
            ? live.filter((p) => p.group_id === view.id)
            : live;

  return source.filter((p) => matchesQuery(p, query));
}

export interface ViewCounts {
  all: number;
  favorites: number;
  ungrouped: number;
  bin: number;
  groups: Record<string, number>;
}

// Counts ignore the search query: the sidebar reports what each view holds,
// not what a transient filter leaves of it. The spec's invariant is that no
// view can hide projects without saying so, which means an empty group has to
// show a 0 rather than vanish — hence seeding the map from every group first.
export function viewCounts(live: Project[], deleted: Project[], groups: Group[]): ViewCounts {
  const byGroup: Record<string, number> = {};
  for (const g of groups) byGroup[g.id] = 0;

  let ungrouped = 0;
  let favorites = 0;
  for (const p of live) {
    if (p.favorite) favorites++;
    if (p.group_id === null) {
      ungrouped++;
    } else if (p.group_id in byGroup) {
      byGroup[p.group_id]++;
    }
    // A project naming a group that no longer exists counts towards neither.
    // It is a transient state — delete_group clears membership in one
    // transaction — and the next refetch resolves it.
  }

  return { all: live.length, favorites, ungrouped, bin: deleted.length, groups: byGroup };
}
