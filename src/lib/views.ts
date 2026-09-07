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

// A "name: value" query, e.g. `client: acme`. The name must be non-empty and
// hold no whitespace — which is what keeps `D:\Games` and `build at 12:30`
// as ordinary text searches rather than queries for a property called "D" or
// "build at 12". A single-character name is likewise rejected, since a bare
// drive letter is far more common than a one-letter property.
const PROPERTY_QUERY = /^\s*([^\s:]{2,})\s*:\s*([\s\S]*)$/;

export function isPropertyQuery(query: string): boolean {
  return PROPERTY_QUERY.test(query);
}

// Every distinct property name across the given projects, folded to lower case
// and sorted. Backs the search hint — the syntax is only discoverable if the
// app says which names are actually in use.
export function propertyKeys(projects: Project[]): string[] {
  const seen = new Set<string>();
  for (const p of projects) {
    for (const key of Object.keys(p.properties ?? {})) seen.add(key.toLowerCase());
  }
  return [...seen].sort();
}

function matchesProperty(project: Project, name: string, value: string): boolean {
  const wanted = name.trim().toLowerCase();
  const needle = value.trim().toLowerCase();
  for (const [key, stored] of Object.entries(project.properties ?? {})) {
    if (key.trim().toLowerCase() !== wanted) continue;
    // An empty value asks "does this project have the property at all?",
    // which is the natural reading of typing `client:` and pausing.
    return needle.length === 0 || stored.toLowerCase().includes(needle);
  }
  return false;
}

// Two modes. `name: value` asks about one property and matches nothing else —
// a project without that property never matches, however its text reads.
// Anything else is free text over name, path, tags and property values.
//
// An empty query matches everything so the caller does not special-case it.
export function matchesQuery(project: Project, query: string): boolean {
  const property = PROPERTY_QUERY.exec(query);
  if (property) return matchesProperty(project, property[1], property[2]);

  const q = query.trim().toLowerCase();
  if (q.length === 0) return true;
  if (project.name.toLowerCase().includes(q)) return true;
  if (project.directory.toLowerCase().includes(q)) return true;
  if (project.tags.some((tag) => tag.toLowerCase().includes(q))) return true;
  return Object.values(project.properties ?? {}).some((v) => v.toLowerCase().includes(q));
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
