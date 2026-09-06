import { describe, expect, it } from "vitest";
import {
  FALLBACK_ICON,
  ICON_NAMES,
  customIconName,
  customIconSrc,
  iconPaths,
  isCustomIcon,
} from "./icons";

describe("ICON_NAMES", () => {
  it("has 24 unique names", () => {
    expect(ICON_NAMES.length).toBe(24);
    expect(new Set(ICON_NAMES).size).toBe(24);
  });
  it("includes the fallback", () => {
    expect(ICON_NAMES).toContain(FALLBACK_ICON);
  });
  it("gives every name at least one drawable path", () => {
    for (const name of ICON_NAMES) {
      expect(iconPaths(name).length).toBeGreaterThan(0);
    }
  });
});

describe("iconPaths", () => {
  it("falls back to the folder glyph for an unknown name rather than failing", () => {
    expect(iconPaths("no-such-icon")).toEqual(iconPaths(FALLBACK_ICON));
  });
  it("falls back for a project or group with no icon set", () => {
    expect(iconPaths(null)).toEqual(iconPaths(FALLBACK_ICON));
    expect(iconPaths(undefined)).toEqual(iconPaths(FALLBACK_ICON));
  });
  it("falls back for a custom name, which is not drawn inline", () => {
    expect(iconPaths("custom:my-logo")).toEqual(iconPaths(FALLBACK_ICON));
  });
});

describe("isCustomIcon / customIconName", () => {
  it("recognises the custom prefix", () => {
    expect(isCustomIcon("custom:my-logo")).toBe(true);
    expect(isCustomIcon("gamepad")).toBe(false);
    expect(isCustomIcon(null)).toBe(false);
    expect(isCustomIcon(undefined)).toBe(false);
  });
  it("strips the prefix", () => {
    expect(customIconName("custom:my-logo")).toBe("my-logo");
  });
  it("leaves a bundled name alone", () => {
    expect(customIconName("gamepad")).toBe("gamepad");
  });
});

describe("customIconSrc", () => {
  it("builds a data URI the img tag can load", () => {
    const src = customIconSrc('<svg viewBox="0 0 24 24"><path d="M1 1"/></svg>');
    expect(src.startsWith("data:image/svg+xml;charset=utf-8,")).toBe(true);
  });
  it("percent-encodes the characters that would break the attribute", () => {
    const src = customIconSrc('<svg a="b#c"></svg>');
    expect(src).toContain("%3Csvg");
    expect(src).toContain("%23");
    expect(src).not.toContain("<");
    expect(src).not.toContain("#");
  });
});
