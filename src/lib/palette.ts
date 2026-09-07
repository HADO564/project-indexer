// Hand-maintained mirror of crates/core/src/domain/palette.rs. That file is
// the source of truth for which names are storable; change one, change the
// other. Nothing checks this automatically.
//
// A stored colour is a bare name ("gold"), never a literal, so a future theme
// plugin can recolour every project and group coherently by overriding the
// @theme tokens in app.css. Resolving an unknown name to neutral rather than
// throwing is the same ignore-what-you-do-not-know rule the --json contract
// uses: a database written by a newer build must still render.
export const SWATCHES = [
  "cyan",
  "gold",
  "amber",
  "rust",
  "violet",
  "green",
  "blue",
  "pink",
] as const;

export type Swatch = (typeof SWATCHES)[number];

const NEUTRAL = "var(--color-phos-faint)";

export function isSwatch(name: unknown): name is Swatch {
  return typeof name === "string" && (SWATCHES as readonly string[]).includes(name);
}

// Exactly "#" plus six hex digits — nothing else. Mirrors is_hex_literal in
// crates/core/src/domain/palette.rs.
//
// The strictness is the security boundary, not pedantry: this value is
// interpolated straight into a `style` attribute and `style-src` still carries
// `unsafe-inline`, so the shape of what is allowed through is the only thing
// stopping a stored colour carrying arbitrary CSS. Shorthand, named colours
// and rgb() are refused rather than normalised.
const HEX = /^#[0-9a-f]{6}$/i;

export function isHexColor(name: unknown): boolean {
  return typeof name === "string" && HEX.test(name);
}

// Returns a CSS value. Either one of a fixed set of `var(--color-*)` strings,
// or a hex literal that has passed the exact-six-digit test above — so a
// stored colour still cannot smuggle arbitrary CSS into a `style` attribute,
// which is what makes this safe to interpolate.
//
// A project may store either; a *group* is palette-only, matching
// is_known_swatch / is_valid_project_color in core.
export function swatchVar(name: string | null | undefined): string {
  if (isSwatch(name)) return `var(--color-swatch-${name})`;
  if (isHexColor(name)) return name as string;
  return NEUTRAL;
}

// The colour a project's mark takes: its own, else its group's, else neutral.
//
// Inheriting the group's colour is what makes a project in a coloured group
// read as belonging to it at a glance, rather than sitting grey next to a
// coloured sidebar entry. The *pip* deliberately does not inherit — it is the
// secondary level, and if it did, "this project has its own colour" would be
// indistinguishable from "it inherits its group's".
export function markVar(
  projectColor: string | null | undefined,
  groupColor: string | null | undefined,
): string {
  return swatchVar(projectColor ?? groupColor);
}
