import { describe, expect, it } from "vitest";
import { SWATCHES, isHexColor, isSwatch, markVar, swatchVar } from "./palette";

describe("SWATCHES", () => {
  it("mirrors crates/core/src/domain/palette.rs exactly, in order", () => {
    expect([...SWATCHES]).toEqual([
      "cyan",
      "gold",
      "amber",
      "rust",
      "violet",
      "green",
      "blue",
      "pink",
    ]);
  });
  it("has eight unique names", () => {
    expect(new Set(SWATCHES).size).toBe(8);
  });
});

describe("isSwatch", () => {
  it("accepts a known name", () => {
    expect(isSwatch("gold")).toBe(true);
  });
  it("rejects anything else", () => {
    expect(isSwatch("chartreuse")).toBe(false);
    expect(isSwatch("#e7b64e")).toBe(false);
    expect(isSwatch("")).toBe(false);
    expect(isSwatch(null)).toBe(false);
    expect(isSwatch(undefined)).toBe(false);
  });
});

describe("swatchVar", () => {
  it("resolves a known name to its theme token", () => {
    expect(swatchVar("cyan")).toBe("var(--color-swatch-cyan)");
    expect(swatchVar("pink")).toBe("var(--color-swatch-pink)");
  });
  it("falls back to neutral for an unknown name rather than failing", () => {
    expect(swatchVar("chartreuse")).toBe("var(--color-phos-faint)");
  });
  it("falls back to neutral for a project with no colour set", () => {
    expect(swatchVar(null)).toBe("var(--color-phos-faint)");
    expect(swatchVar(undefined)).toBe("var(--color-phos-faint)");
  });
  it("passes through a valid hex literal from the colour picker", () => {
    expect(swatchVar("#e7b64e")).toBe("#e7b64e");
  });
  it("never emits anything but a token or an exact hex literal", () => {
    // This is the security boundary: the result goes straight into a `style`
    // attribute, and style-src still carries unsafe-inline.
    for (const input of [
      "#fff",
      "red",
      "url(x)",
      "gold; background: red",
      "#e7b64e; background: url(x)",
      "rgb(1,2,3)",
      "#gggggg",
      "#e7b64",
    ]) {
      expect(swatchVar(input)).toMatch(/^(var\(--color-[a-z-]+\)|#[0-9a-f]{6})$/i);
    }
  });
});

describe("isHexColor", () => {
  it("accepts exactly six hex digits after a hash", () => {
    expect(isHexColor("#e7b64e")).toBe(true);
    expect(isHexColor("#FFFFFF")).toBe(true);
  });
  it("refuses every other shape rather than normalising it", () => {
    for (const input of ["#abc", "#e7b64", "#e7b64ee", "e7b64e", "#gggggg", "red", "", null]) {
      expect(isHexColor(input)).toBe(false);
    }
  });
});

describe("markVar", () => {
  it("prefers the project's own colour", () => {
    expect(markVar("gold", "cyan")).toBe("var(--color-swatch-gold)");
  });
  it("falls back to the group's colour, so a project in a coloured group is not grey", () => {
    expect(markVar(null, "cyan")).toBe("var(--color-swatch-cyan)");
    expect(markVar(undefined, "cyan")).toBe("var(--color-swatch-cyan)");
  });
  it("falls back to neutral when neither has one", () => {
    expect(markVar(null, null)).toBe("var(--color-phos-faint)");
  });
  it("falls back to neutral for an unknown name at either level", () => {
    expect(markVar("chartreuse", null)).toBe("var(--color-phos-faint)");
    expect(markVar(null, "chartreuse")).toBe("var(--color-phos-faint)");
  });
});
