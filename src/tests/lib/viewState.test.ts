import { describe, expect, it } from "vitest";
import type { Group } from "$lib/api/types";
import { restoreView, restoreViewMode } from "$lib/viewState";

const group = (id: string, position: number): Group => ({
  id,
  name: id,
  color: "cyan",
  icon: "briefcase",
  position,
  created_at: "2026-01-01T00:00:00Z",
  updated_at: "2026-01-01T00:00:00Z",
});

const groups = [group("g-work", 0), group("g-personal", 1)];

describe("restoreViewMode", () => {
  it("restores a known mode", () => {
    expect(restoreViewMode("grid")).toBe("grid");
    expect(restoreViewMode("list")).toBe("list");
  });
  it("defaults to list for junk or nothing stored", () => {
    expect(restoreViewMode(null)).toBe("list");
    expect(restoreViewMode("")).toBe("list");
    expect(restoreViewMode("mosaic")).toBe("list");
  });
  it("falls back to list for the retired compact mode", () => {
    // Anyone who left the app in compact lands on list rather than a blank
    // toggle, without needing a stored-state migration.
    expect(restoreViewMode("compact")).toBe("list");
  });
});

describe("restoreView", () => {
  it("restores All, Favourites and Ungrouped", () => {
    expect(restoreView("all", groups)).toEqual({ kind: "all" });
    expect(restoreView("favorites", groups)).toEqual({ kind: "favorites" });
    expect(restoreView("ungrouped", groups)).toEqual({ kind: "ungrouped" });
  });
  it("restores a group that still exists", () => {
    expect(restoreView("group:g-work", groups)).toEqual({ kind: "group", id: "g-work" });
  });
  it("falls back to All for a group that has since been deleted", () => {
    // Rendering an empty list with no explanation is the failure this avoids.
    expect(restoreView("group:g-gone", groups)).toEqual({ kind: "all" });
  });
  it("never restores Bin as the landing view", () => {
    // The bin is a destination you go to, not a place to start.
    expect(restoreView("bin", groups)).toEqual({ kind: "all" });
  });
  it("falls back to All for junk or nothing stored", () => {
    expect(restoreView(null, groups)).toEqual({ kind: "all" });
    expect(restoreView("", groups)).toEqual({ kind: "all" });
    expect(restoreView("group:", groups)).toEqual({ kind: "all" });
    expect(restoreView("nonsense", groups)).toEqual({ kind: "all" });
  });
  it("falls back to All when there are no groups at all", () => {
    expect(restoreView("group:g-work", [])).toEqual({ kind: "all" });
  });
});
