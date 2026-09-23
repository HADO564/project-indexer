import { describe, expect, it } from "vitest";
import type { Group, View } from "$lib/api/types";
import { parseViewKey, viewKey, viewLabel } from "$lib/views";

// Which projects a view holds, its counts and the search moved to core; their
// tests moved with them, to crates/core/src/tests/domain/views.rs. What is
// left here is the browser's part: the storage key and the on-screen label.

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
