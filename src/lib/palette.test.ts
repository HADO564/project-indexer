import { describe, expect, it } from "vitest";
import { SWATCHES, isSwatch, markVar, swatchVar } from "./palette";

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
  it("never returns a literal colour, whatever it is given", () => {
    for (const input of ["#fff", "red", "url(x)", "gold; background: red"]) {
      expect(swatchVar(input)).toMatch(/^var\(--color-[a-z-]+\)$/);
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
