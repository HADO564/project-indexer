import { describe, expect, it } from "vitest";
import type { Group, Project } from "./api/types";
import {
  isPropertyQuery,
  matchesQuery,
  propertyKeys,
  parseViewKey,
  resolveView,
  viewCounts,
  viewKey,
  viewLabel,
  type View,
} from "./views";

const project = (over: Partial<Project> = {}): Project => ({
  id: "id-1",
  is_deleted: false,
  name: "Friction Engine",
  description: "",
  directory: "D:\\Games\\friction-engine",
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  last_opened_at: null,
  tags: [],
  favorite: false,
  open_with: null,
  notes: null,
  properties: {},
  trackers: [],
  group_id: null,
  color: null,
  icon: null,
  ...over,
});

const group = (over: Partial<Group> = {}): Group => ({
  id: "g-work",
  name: "Work",
  color: "cyan",
  icon: "briefcase",
  position: 0,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
  ...over,
});

const work = group();
const personal = group({ id: "g-personal", name: "Personal", position: 1 });

const a = project({ id: "a", name: "Alpha", group_id: "g-work", favorite: true });
const b = project({ id: "b", name: "Bravo", group_id: "g-work" });
const c = project({ id: "c", name: "Charlie", group_id: null, favorite: true });
const d = project({ id: "d", name: "Delta", group_id: "g-personal" });
const gone = project({ id: "z", name: "Zulu", is_deleted: true });

const live = [a, b, c, d];
const deleted = [gone];

describe("viewKey / parseViewKey", () => {
  it("round-trips every view", () => {
    const views: View[] = [
      { kind: "all" },
      { kind: "favorites" },
      { kind: "ungrouped" },
      { kind: "bin" },
      { kind: "group", id: "g-work" },
    ];
    for (const view of views) {
      expect(parseViewKey(viewKey(view))).toEqual(view);
    }
  });
  it("round-trips a group id containing a colon", () => {
    const view: View = { kind: "group", id: "a:b:c" };
    expect(parseViewKey(viewKey(view))).toEqual(view);
  });
  it("returns null for junk rather than throwing", () => {
    expect(parseViewKey("nonsense")).toBeNull();
    expect(parseViewKey("group:")).toBeNull();
    expect(parseViewKey("")).toBeNull();
    expect(parseViewKey(null)).toBeNull();
    expect(parseViewKey(undefined)).toBeNull();
  });
});

describe("resolveView", () => {
  it("all is every live project", () => {
    expect(resolveView({ kind: "all" }, live, deleted, "").map((p) => p.id)).toEqual([
      "a",
      "b",
      "c",
      "d",
    ]);
  });
  it("favorites is the favourited live projects", () => {
    expect(resolveView({ kind: "favorites" }, live, deleted, "").map((p) => p.id)).toEqual([
      "a",
      "c",
    ]);
  });
  it("a group is the projects carrying its id", () => {
    expect(
      resolveView({ kind: "group", id: "g-work" }, live, deleted, "").map((p) => p.id),
    ).toEqual(["a", "b"]);
  });
  it("ungrouped catches group_id == null", () => {
    expect(resolveView({ kind: "ungrouped" }, live, deleted, "").map((p) => p.id)).toEqual(["c"]);
  });
  it("bin is the deleted projects, which are not in the live list", () => {
    expect(resolveView({ kind: "bin" }, live, deleted, "").map((p) => p.id)).toEqual(["z"]);
  });
  it("preserves the order it was given, because the backend already sorted", () => {
    const reversed = [...live].reverse();
    expect(resolveView({ kind: "all" }, reversed, deleted, "").map((p) => p.id)).toEqual([
      "d",
      "c",
      "b",
      "a",
    ]);
  });
  it("applies the query within the view rather than across everything", () => {
    expect(
      resolveView({ kind: "group", id: "g-work" }, live, deleted, "a").map((p) => p.id),
    ).toEqual(["a", "b"]);
    expect(
      resolveView({ kind: "group", id: "g-work" }, live, deleted, "charlie").map((p) => p.id),
    ).toEqual([]);
  });
  it("returns an empty list for a group with no members", () => {
    expect(resolveView({ kind: "group", id: "g-nobody" }, live, deleted, "")).toEqual([]);
  });
});

describe("matchesQuery", () => {
  const p = project({
    name: "Friction Engine",
    directory: "D:\\Games\\fe",
    tags: ["rust", "game"],
  });
  it("matches the name, case-insensitively", () => {
    expect(matchesQuery(p, "FRICTION")).toBe(true);
  });
  it("matches the path", () => {
    expect(matchesQuery(p, "games")).toBe(true);
  });
  it("matches a tag", () => {
    expect(matchesQuery(p, "rust")).toBe(true);
  });
  it("does not match something absent", () => {
    expect(matchesQuery(p, "unreal")).toBe(false);
  });
  it("treats an empty or whitespace query as matching everything", () => {
    expect(matchesQuery(p, "")).toBe(true);
    expect(matchesQuery(p, "   ")).toBe(true);
  });
  it("matches a property value in free text too", () => {
    const withProps = project({ properties: { client: "Acme Corp" } });
    expect(matchesQuery(withProps, "acme")).toBe(true);
  });
});

describe("matchesQuery — the name: value syntax", () => {
  const acme = project({ id: "acme", name: "Alpha", properties: { client: "Acme Corp" } });
  const globex = project({ id: "globex", name: "Bravo", properties: { client: "Globex" } });
  const none = project({ id: "none", name: "Acme lookalike" });

  it("matches on the named property only", () => {
    expect(matchesQuery(acme, "client: Acme")).toBe(true);
    expect(matchesQuery(globex, "client: Acme")).toBe(false);
  });
  it("does not match a project lacking the property, even if the text appears elsewhere", () => {
    // The whole point of the syntax: "client: acme" is a question about the
    // client property, not a free-text search that happens to spell acme.
    expect(matchesQuery(none, "client: acme")).toBe(false);
    expect(matchesQuery(none, "acme")).toBe(true);
  });
  it("is case-insensitive in both the key and the value", () => {
    expect(matchesQuery(acme, "CLIENT: acme")).toBe(true);
    expect(matchesQuery(acme, "client: ACME")).toBe(true);
  });
  it("tolerates space on either side of the colon, or none", () => {
    for (const q of ["client:Acme", "client : Acme", "  client:  Acme  "]) {
      expect(matchesQuery(acme, q)).toBe(true);
    }
  });
  it("matches every project that has the property when no value is given", () => {
    expect(matchesQuery(acme, "client:")).toBe(true);
    expect(matchesQuery(globex, "client:")).toBe(true);
    expect(matchesQuery(none, "client:")).toBe(false);
  });
  it("matches a substring of the value, like free text does", () => {
    expect(matchesQuery(acme, "client: corp")).toBe(true);
  });
  it("falls back to free text when nothing precedes the colon", () => {
    // ":30" is not a property query — there is no name — so it searches text.
    const timed = project({ name: "build at 12:30" });
    expect(matchesQuery(timed, ":30")).toBe(true);
  });
  it("treats a Windows drive letter as free text, not a property", () => {
    // "D:" would otherwise be read as the property "D", and every path search
    // on Windows starts this way.
    const onDrive = project({ directory: "D:\Games\fe" });
    expect(matchesQuery(onDrive, "D:\Games")).toBe(true);
  });
});

describe("isPropertyQuery", () => {
  it("recognises a name: value query", () => {
    expect(isPropertyQuery("client: acme")).toBe(true);
    expect(isPropertyQuery("client:")).toBe(true);
  });
  it("rejects free text, a drive letter, and a bare colon", () => {
    expect(isPropertyQuery("acme")).toBe(false);
    expect(isPropertyQuery("D:\Games")).toBe(false);
    expect(isPropertyQuery(":30")).toBe(false);
    expect(isPropertyQuery("")).toBe(false);
  });
});

describe("propertyKeys", () => {
  it("lists the distinct keys across the given projects, sorted", () => {
    const one = project({ id: "1", properties: { client: "Acme", engine: "Unreal" } });
    const two = project({ id: "2", properties: { client: "Globex", priority: "high" } });
    expect(propertyKeys([one, two])).toEqual(["client", "engine", "priority"]);
  });
  it("is empty when no project has any", () => {
    expect(propertyKeys([project()])).toEqual([]);
  });
  it("folds keys differing only in case, so the hint does not list both", () => {
    const one = project({ id: "1", properties: { Client: "Acme" } });
    const two = project({ id: "2", properties: { client: "Globex" } });
    expect(propertyKeys([one, two])).toEqual(["client"]);
  });
});

describe("viewCounts", () => {
  it("counts every view, including a group with no members", () => {
    const empty = group({ id: "g-empty", name: "Empty", position: 2 });
    expect(viewCounts(live, deleted, [work, personal, empty])).toEqual({
      all: 4,
      favorites: 2,
      ungrouped: 1,
      bin: 1,
      groups: { "g-work": 2, "g-personal": 1, "g-empty": 0 },
    });
  });
  it("does not count a project whose group was deleted as a member of anything", () => {
    const orphan = project({ id: "o", group_id: "g-gone" });
    const counts = viewCounts([...live, orphan], deleted, [work, personal]);
    expect(counts.all).toBe(5);
    expect(counts.ungrouped).toBe(1);
    expect(counts.groups).toEqual({ "g-work": 2, "g-personal": 1 });
  });
  it("ignores the search query — the sidebar reports what a view holds", () => {
    // Counts are deliberately not query-filtered: the invariant is that no
    // view can hide projects without saying so.
    expect(viewCounts([], [], [work]).groups["g-work"]).toBe(0);
  });
});

describe("viewLabel", () => {
  it("names the built-in views", () => {
    expect(viewLabel({ kind: "all" }, [])).toBe("All");
    expect(viewLabel({ kind: "favorites" }, [])).toBe("Favourites");
    expect(viewLabel({ kind: "ungrouped" }, [])).toBe("Ungrouped");
    expect(viewLabel({ kind: "bin" }, [])).toBe("Bin");
  });
  it("names a group by its group", () => {
    expect(viewLabel({ kind: "group", id: "g-work" }, [work])).toBe("Work");
  });
  it("falls back for a group that no longer exists", () => {
    expect(viewLabel({ kind: "group", id: "g-gone" }, [work])).toBe("Unknown group");
  });
});
