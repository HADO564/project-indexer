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

// Returns a CSS value, not a colour literal. The return is always one of a
// fixed set of `var(--color-*)` strings, which is what makes it safe to
// interpolate into a `style` attribute — a stored colour name can never
// smuggle arbitrary CSS through it.
export function swatchVar(name: string | null | undefined): string {
  return isSwatch(name) ? `var(--color-swatch-${name})` : NEUTRAL;
}
