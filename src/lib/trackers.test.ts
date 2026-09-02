import { describe, expect, it } from "vitest";
import { trackerColor, trackerFields, trackerKind } from "./trackers";
import type { Tracker } from "./api/types";

const gitTracker = (over: Record<string, unknown> = {}): Tracker =>
  ({
    Git: {
      repo_root: "D:\\Games\\friction-engine",
      dirty: true,
      detached_head: false,
      repo_url: "git@github.com:acme/friction-engine.git",
      web_url: "https://github.com/acme/friction-engine",
      contributors: [],
      curr_branch: "main",
      branches: ["main", "develop"],
      commit_hash: "a1b2c3d4",
      ...over,
    },
  }) as unknown as Tracker;

describe("trackerKind", () => {
  it("reads the variant key", () => {
    expect(trackerKind(gitTracker())).toBe("Git");
  });
  it("reads a bare string variant", () => {
    expect(trackerKind("Unity" as unknown as Tracker)).toBe("Unity");
  });
});

describe("trackerColor", () => {
  it("is case-insensitive and stable for a known kind", () => {
    expect(trackerColor("git")).toBe(trackerColor("Git"));
    expect(trackerColor("git")).toBe("hsl(2 65% 70%)");
  });
  it("gives known kinds distinct hues", () => {
    const kinds = ["git", "unreal", "unity", "blender"];
    expect(new Set(kinds.map(trackerColor)).size).toBe(kinds.length);
  });
  it("derives a stable hue for an unknown kind", () => {
    expect(trackerColor("godot-mono")).toBe(trackerColor("godot-mono"));
    expect(trackerColor("godot-mono")).toMatch(/^hsl\(\d{1,3} 65% 70%\)$/);
  });
  it("always fixes L/S so any kind reads on the dark ground", () => {
    for (const k of ["git", "unreal", "xyz", "a-brand-new-tracker"]) {
      expect(trackerColor(k)).toMatch(/ 65% 70%\)$/);
    }
  });
});

describe("trackerFields typing", () => {
  const byLabel = (t: Tracker) =>
    Object.fromEntries(trackerFields(t).map((f) => [f.label, f]));

  it("types an http url as a link", () => {
    expect(byLabel(gitTracker())["Web url"]).toMatchObject({
      type: "link",
      text: "https://github.com/acme/friction-engine",
    });
  });

  it("types an ssh remote as code, not a broken link", () => {
    expect(byLabel(gitTracker())["Repo url"].type).toBe("code");
  });

  it("types *_root / *_path keys as path", () => {
    expect(byLabel(gitTracker())["Repo root"].type).toBe("path");
  });

  it("types commit-hash keys as code", () => {
    expect(byLabel(gitTracker())["Commit hash"].type).toBe("code");
  });

  it("types arrays as chips and drops empty ones", () => {
    const f = byLabel(gitTracker());
    expect(f["Branches"]).toMatchObject({ type: "chips", items: ["main", "develop"] });
    expect(f["Contributors"]).toBeUndefined();
  });

  it("shows a true bool as a flag and hides a false one", () => {
    const f = byLabel(gitTracker());
    expect(f["Dirty"].type).toBe("flag");
    expect(f["Detached head"]).toBeUndefined();
  });

  it("omits null / empty-string values", () => {
    expect(byLabel(gitTracker({ commit_hash: null }))["Commit hash"]).toBeUndefined();
  });

  it("falls back to text", () => {
    expect(byLabel(gitTracker())["Curr branch"].type).toBe("text");
  });
});
