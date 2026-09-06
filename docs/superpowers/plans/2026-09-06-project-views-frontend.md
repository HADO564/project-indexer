# Project views, colour, icons and groups — frontend implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the frontend half of project views, colour, icons and groups — three view modes, a sidebar that replaces the flat list and absorbs the Favourites and Bin modals, and colour/icon/group pickers — against the finished backend API merged in `d6d20f2`.

**Architecture:** Two pure modules (`palette.ts`, `views.ts`) hold every decision that can be tested without mounting a component; `viewState.ts` splits its pure restore rules from its `localStorage` access for the same reason. `ProjectCard.svelte` is decomposed into behaviour (`ProjectActionsMenu`, `ProjectMark`) plus three interchangeable presentations that take their action set as a snippet prop, which is what lets one set of row layouts serve both the standard menu and Bin's. Colour never appears as a literal: a stored palette *name* resolves to a `var(--color-swatch-*)` token. Custom icons render only through `<img src="data:…">`.

**Tech Stack:** Svelte 5 (runes), SvelteKit 2 + `adapter-static` (SPA), Tailwind CSS 4 (`@theme` tokens), TypeScript 5.6, vitest 4 (node environment), Tauri 2. Rust is touched in one task only (`crates/core/src/icons/sanitize.rs`).

**Spec:** [`docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`](../specs/2026-09-05-project-views-grouping-design.md)

**Handoff:** [`docs/handoffs/2026-09-05-project-views-frontend.md`](../../handoffs/2026-09-05-project-views-frontend.md)

## Global Constraints

- **No raw colour, anywhere.** Every colour resolves through an `@theme` token. A stored colour is a palette **name** (`"gold"`), never a literal. An unknown name falls back to neutral rather than failing.
- **A custom icon renders only as `<img src="data:image/svg+xml;charset=utf-8,…">`.** Never `{@html}`. The sanitizer in `core` is layer one; the `<img>` is layer two, and it is inert regardless of what layer one missed. Two rounds of adversarial review have already found real bypasses in that sanitizer — do not make it load-bearing alone.
- **`src/lib/api/types.ts` is a hand-maintained mirror of the Rust models and nothing checks it.** Update it in the same commit as anything it mirrors.
- **`pnpm run check` baseline: 0 errors, exactly 8 `state_referenced_locally` warnings**, all in `EditProjectForm.svelte`. That is `PI-003`, a documented false positive. Do not fix them. Do not add a ninth.
- **Rust clippy baseline: exactly 2 warnings** — `module-inception` in `indexer-core`, and `unnecessary_sort_by` at `crates/core/src/platform/app_discovery.rs:116`. The pre-commit hook runs clippy **without** `-D warnings`, so it enforces no baseline at all; count them yourself with `cargo clippy --workspace --all-targets 2>&1 | grep "^warning: " | sort | uniq -c`. Read the *distinct* lines, not a raw count: cargo prints a per-crate "generated 2 warnings" summary line alongside each real warning, so a bare `grep -c` reports 4 on a clean tree.
- **The pre-commit hook runs `cargo fmt --check` against the WORKING TREE, not the index.** Run `cargo fmt` *before* `git add`, or you will commit unformatted code through a green hook.
- **Install the hook on a fresh clone:** `git config core.hooksPath .githooks`.
- **Follow the existing style.** Shared Tailwind utility strings live in `src/lib/components/styles.ts` — use `inputClass`, `labelClass`, `buttonClass`, `primaryButtonClass`, `dangerButtonClass`, `cardClass` rather than re-typing them. Chrome uses `font-display` (VT323) at 13–15px; data text uses the body mono.
- **Neither CI nor the pre-commit hook launches the app.** `PI-005` compiled, passed every gate, and still failed to show a window. This plan rewrites the whole page layout: run `pnpm tauri dev` and look at it before calling a task done.
- **Commit trailer:** every commit ends with `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>` (name whichever model did the work).
- **Scope discipline.** Do not widen the work. A real problem found outside a task gets written down in `docs/KNOWN-ISSUES.md` or `docs/checklist.md`, not fixed here. Keyboard navigation between sidebar entries is explicitly **out of scope** — it is its own roadmap item (the global-shortcut plugin is registered but bound to nothing).

## Decisions this plan settles

The handoff §6 left five questions open and named them as this half's to decide. Settled here, with reasons, so they are not reopened mid-execution:

1. **The sidebar is a fixed 13rem column and does not collapse to an icon rail.** A rail's identifying mark for a user-named group is its icon, drawn from a shared 24-glyph set — two groups can legitimately pick the same one, at which point the rail shows two indistinguishable entries. That is the "you cannot tell what is hidden" failure the sectioned design was rejected for, in a smaller costume. A fixed column also adds no new persisted state.
2. **The sidebar shows counts.** Not actually optional: the spec's own new invariant reads *"No view can hide projects without saying so: the sidebar always shows every view and its count."* Counts are computed in `views.ts` from the already-fetched arrays — no extra query, no second source of truth.
3. **Group management lives in `GroupManagerModal`**, as the spec says. Inline sidebar editing is nicer and fiddlier; the spec settled it and there is no new information.
4. **An unknown icon name renders the `folder` glyph.** It is the bundled set's most neutral member and reads as "a thing that contains a project" for both a project and a group.
5. **Keyboard navigation is not built.** Out of scope, per the handoff.

## Two deviations from the spec, and why

The standing instruction over this work is that the code is the truth and the docs are a map drawn by someone else. Two places where they disagree; in both the plan follows the code, and Task 11 amends the spec.

**A. `views.ts` does not sort.** The spec gives it the signature `(projects, groups, view, sort, query) → Project[]`. But `get_all_projects`, `get_favorite_projects` and `get_deleted_projects` **already sort**, in `core::domain::sorting`, behind Rust tests, with tie-breaking by `created_at` then the unique `id`. Re-implementing that comparator in TypeScript would create a second hand-maintained cross-language mirror with nothing checking it — the exact drift hazard `types.ts` is already flagged for. So the fetch keeps passing `SortOptions`, and `views.ts` filters order-preservingly. Sort still applies uniformly across every view, which is what the spec was protecting.

**B. Favourites is derived from the already-fetched live list, not a second fetch.** The spec's source table says `getFavoriteProjects`. Reading `ProjectService::list_favorites` (`crates/core/src/application/service.rs:121`): it filters `!is_deleted && favorite`, then sorts. `list_projects` filters `!is_deleted`, then sorts with the same options. Every comparator in `sorting.rs` ends in the unique `id`, so the sort is a total order — filtering a sorted list and sorting a filtered list therefore produce identical results. `allProjects.filter(p => p.favorite)` *is* `getFavoriteProjects(sameOptions)`. One fewer IPC round trip, and the sidebar's Favourites count and its list cannot disagree because they read the same array. The Bin is genuinely separate data (`get_all_projects` excludes soft-deleted rows) and keeps its own fetch.

## One deviation from the handoff's task order, and why

The handoff requires its task 19 (widening the sanitizer's attribute allow-list) to land before its task 20 (the bundled icon set), because sanitizing happens once on import, the store keeps only the sanitized form, and an icon imported before the widening has permanently lost `stroke-dasharray`, `stroke-dashoffset`, `stroke-miterlimit`, `fill-opacity` and `stroke-opacity`.

**This plan makes it Task 1** — earlier than the handoff's slot, never later. The binding property is *"no icon can be imported before the allow-list is widened"*, and the actual import path is the icon picker (handoff task 18), not the bundled set (task 20). Landing the widening first satisfies the handoff's constraint strictly and closes the larger hole. It is a Rust-only change with no frontend dependency, so nothing is gained by waiting.

The bundled icon set moves earlier too, into Task 2, for a plain dependency reason: `ProjectMark` (Task 4) and `SidebarEntry` (Task 5) both render bundled glyphs, and a plan cannot define a consumer before the interface it consumes exists. Every ordering constraint the handoff calls load-bearing is preserved: the no-behaviour-change component split (Task 4) lands and is verified before any new view mode; Favourites (Task 7) and Bin (Task 8) stay late and separate; the sanitizer precedes anything that can import an icon.

## File structure

**Created — pure logic (vitest, node environment, no components mounted):**

| File | Responsibility |
|---|---|
| `src/lib/palette.ts` | palette name → `var(--color-swatch-*)`, with a neutral fallback |
| `src/lib/palette.test.ts` | known name resolves, unknown falls back, mirror matches Rust |
| `src/lib/icons.ts` | the bundled glyph set (name → path data), the fallback rule, the custom-icon data-URI builder |
| `src/lib/icons.test.ts` | every name resolves, unknown falls back to `folder`, the data URI encodes correctly |
| `src/lib/views.ts` | the `View` type, view→projects resolution, search matching, sidebar counts |
| `src/lib/views.test.ts` | one test per view, query matching, counts |
| `src/lib/viewState.ts` | pure restore rules + the `localStorage` wrappers around them |
| `src/lib/viewState.test.ts` | a deleted group falls back to All; Bin is never restored |

**Created — API wrappers:**

| File | Responsibility |
|---|---|
| `src/lib/api/groups.ts` | the five group commands |
| `src/lib/api/icons.ts` | the three custom-icon commands |

**Created — components:**

| File | Responsibility |
|---|---|
| `src/lib/components/BundledIcon.svelte` | inlines one bundled glyph, tinted through `currentColor` |
| `src/lib/components/ProjectMark.svelte` | a project's icon + colour pip |
| `src/lib/components/ProjectActionsMenu.svelte` | the standard `···` menu, extracted from `ProjectCard` |
| `src/lib/components/BinActions.svelte` | Restore / Delete permanently, with the two-click confirm |
| `src/lib/components/ProjectRow.svelte` | list presentation (today's card, unchanged) |
| `src/lib/components/ProjectTile.svelte` | grid presentation |
| `src/lib/components/ProjectCompactRow.svelte` | compact presentation |
| `src/lib/components/Sidebar.svelte` | the rail: All, Favourites, groups, Ungrouped, Bin |
| `src/lib/components/SidebarEntry.svelte` | one entry — icon, colour, label, count |
| `src/lib/components/ViewControls.svelte` | mode toggle + search box |
| `src/lib/components/GroupManagerModal.svelte` | create / rename / recolour / re-icon / reorder / delete |
| `src/lib/components/SwatchPicker.svelte` | eight-swatch colour picker, with a "none" option for projects |
| `src/lib/components/IconPicker.svelte` | bundled grid + custom icons + import |

**Modified:** `src/app.css`, `src/lib/api/types.ts`, `src/lib/components/ProjectList.svelte`, `src/lib/components/CreateProjectForm.svelte`, `src/lib/components/EditProjectForm.svelte`, `src/routes/+page.svelte`, `crates/core/src/icons/sanitize.rs`, and the docs.

**Deleted:** `src/lib/components/ProjectCard.svelte` (Task 4), `src/lib/components/FavoritesModal.svelte` (Task 7), `src/lib/components/BinModal.svelte` (Task 8).

---

### Task 1: Widen the icon sanitizer's attribute allow-list

**This must land before anything in this plan can import a custom icon.** Sanitizing happens once, on import; the store keeps only the sanitized SVG and nothing re-sanitizes on render. An icon imported before this task has permanently lost the five attributes below, and widening the list afterwards does not bring them back.

**Files:**
- Modify: `crates/core/src/icons/sanitize.rs:20-46` (the `ALLOWED_ATTRS` constant) and its test module
- Modify: `docs/superpowers/specs/2026-09-05-project-views-grouping-design.md` (drop the now-stale "the implementation shipped by the backend half omits…" paragraph)

**Interfaces:**
- Consumes: nothing.
- Produces: `sanitize_svg` preserves `stroke-dasharray`, `stroke-dashoffset`, `stroke-miterlimit`, `fill-opacity`, `stroke-opacity`. No signature change.

**Why these five are safe.** All are inert presentation attributes. None takes a URL, a function reference, or a script context. They cannot carry `url(` (that is a paint-server / filter reference, and `fill` / `stroke` — which can — are already on the list and already value-scanned). The existing value scan that rejects `url(` and `javascript:` applies to every allowed attribute uniformly, so these gain no exemption from it.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)] mod tests` block in `crates/core/src/icons/sanitize.rs`:

```rust
#[test]
fn keeps_inert_presentation_attributes() {
    let input = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <path d="M1 1 L2 2" stroke-dasharray="4 2" stroke-dashoffset="1"
              stroke-miterlimit="2" fill-opacity="0.5" stroke-opacity="0.25"/>
    </svg>"#;
    let out = sanitize_svg(input).expect("well-formed icon should survive");
    for attr in [
        "stroke-dasharray",
        "stroke-dashoffset",
        "stroke-miterlimit",
        "fill-opacity",
        "stroke-opacity",
    ] {
        assert!(out.contains(attr), "sanitizer dropped {attr}: {out}");
    }
}

#[test]
fn still_rejects_url_values_in_the_newly_allowed_attributes() {
    let input = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
        <path d="M1 1 L2 2" fill-opacity="url(#evil)"/>
    </svg>"#;
    let out = sanitize_svg(input).expect("should sanitize rather than fail");
    assert!(!out.contains("url("), "url() survived in an allowed attribute: {out}");
}
```

- [ ] **Step 2: Run the tests and verify the first one fails**

Run: `cargo test -p indexer-core icons::sanitize -- --nocapture`
Expected: `keeps_inert_presentation_attributes` FAILS with "sanitizer dropped stroke-dasharray".

- [ ] **Step 3: Widen `ALLOWED_ATTRS`**

In `crates/core/src/icons/sanitize.rs`, extend the constant so it reads (keeping the existing entries and their order, adding five):

```rust
const ALLOWED_ATTRS: &[&str] = &[
    "viewBox",
    "d",
    "cx",
    "cy",
    "r",
    "rx",
    "ry",
    "x",
    "y",
    "x1",
    "y1",
    "x2",
    "y2",
    "width",
    "height",
    "points",
    "transform",
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linecap",
    "stroke-linejoin",
    "stroke-dasharray",
    "stroke-dashoffset",
    "stroke-miterlimit",
    "fill-rule",
    "fill-opacity",
    "stroke-opacity",
    "clip-rule",
    "opacity",
];
```

- [ ] **Step 4: Run the whole Rust suite**

Run: `cargo test --workspace`
Expected: all pass, including the 23 pre-existing sanitizer tests and the 2 new ones.

- [ ] **Step 5: Check formatting and the clippy baseline**

Run: `cargo fmt && cargo clippy --workspace --all-targets 2>&1 | grep "^warning: " | sort | uniq -c`
Expected: four lines — `consider using sort_by_key`, `module has the same name as its containing module`, and cargo's two "generated 2 warnings" summaries. A fifth line is one you introduced; fix it, do not proceed.

- [ ] **Step 6: Amend the spec**

In `docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`, in the "Custom icons → Sanitizer → Attributes" bullet, delete the sentence beginning *"The implementation shipped by the backend half omits the three `stroke-*` and two `*-opacity` additions above"* through the end of that paragraph, and replace it with:

```
  The implementation matches this list exactly as of the frontend half's
  Task 1; the gap it had on merge is closed.
```

- [ ] **Step 7: Commit**

```bash
cargo fmt
git add crates/core/src/icons/sanitize.rs docs/superpowers/specs/2026-09-05-project-views-grouping-design.md
git commit -m "$(cat <<'MSG'
fix(icons): allow five inert presentation attributes through the sanitizer

stroke-dasharray, stroke-dashoffset, stroke-miterlimit, fill-opacity and
stroke-opacity are in the spec's allow-list but were missing from
ALLOWED_ATTRS. Real icon sets use all five routinely.

This lands first because sanitizing is lossy and happens once, on import:
the store keeps only the sanitized SVG and nothing re-sanitizes on render,
so an icon imported before this change loses those attributes permanently.
Nothing in the app can import an icon yet — that arrives with the icon
picker later in this plan — which is what makes now the last safe moment.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 2: The swatch palette and the bundled icon set

Two pure token/data modules. Everything downstream resolves colour and glyphs through them, and both are testable without mounting a component.

**Files:**
- Modify: `src/app.css` (the `@theme` block)
- Create: `src/lib/palette.ts`, `src/lib/palette.test.ts`
- Create: `src/lib/icons.ts`, `src/lib/icons.test.ts`
- Create: `src/lib/components/BundledIcon.svelte`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `SWATCHES: readonly ["cyan","gold","amber","rust","violet","green","blue","pink"]`
  - `type Swatch = (typeof SWATCHES)[number]`
  - `isSwatch(name: unknown): name is Swatch`
  - `swatchVar(name: string | null | undefined): string` — a CSS value, always safe to put in a `style` attribute
  - `ICON_NAMES: readonly string[]` (24 entries), `FALLBACK_ICON = "folder"`
  - `iconPaths(name: string | null | undefined): readonly string[]` — SVG `d` strings, never empty
  - `isCustomIcon(name: string | null | undefined): boolean` — true for `custom:*`
  - `customIconName(name: string): string` — strips the `custom:` prefix
  - `customIconSrc(svg: string): string` — the `data:` URI
  - `<BundledIcon name={...} class={...} />`

- [ ] **Step 1: Write the failing palette test**

Create `src/lib/palette.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import { SWATCHES, isSwatch, swatchVar } from "./palette";

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
```

- [ ] **Step 2: Run it and verify it fails**

Run: `pnpm test -- palette`
Expected: FAIL — cannot resolve `./palette`.

- [ ] **Step 3: Write `src/lib/palette.ts`**

```ts
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
// interpolate into a `style` attribute — the caller can never smuggle
// arbitrary CSS through a stored colour name.
export function swatchVar(name: string | null | undefined): string {
  return isSwatch(name) ? `var(--color-swatch-${name})` : NEUTRAL;
}
```

- [ ] **Step 4: Run the palette test**

Run: `pnpm test -- palette`
Expected: PASS (13 assertions across 8 tests).

- [ ] **Step 5: Add the swatch tokens to `src/app.css`**

Inside the existing `@theme { … }` block, after the `--color-rust` line and before `--font-display`, insert:

```css
  /* Project and group swatches. A stored colour is one of these eight *names*
     (see src/lib/palette.ts and crates/core/src/domain/palette.rs), resolved
     to the token at render — never a stored literal — so a theme override
     recolours every project and group coherently. All eight are tuned to the
     same light L / moderate S so they read on the dark ground the way
     trackerColor() already guarantees for tracker hues. */
  --color-swatch-cyan: #56c8c4;
  --color-swatch-gold: #e7b64e;
  --color-swatch-amber: #e2903a;
  --color-swatch-rust: #e2605a;
  --color-swatch-violet: #a98cf0;
  --color-swatch-green: #7fc98a;
  --color-swatch-blue: #6fa8e6;
  --color-swatch-pink: #e58bbd;
```

- [ ] **Step 6: Write the failing icon test**

Create `src/lib/icons.test.ts`:

```ts
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
  });
  it("strips the prefix", () => {
    expect(customIconName("custom:my-logo")).toBe("my-logo");
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
```

- [ ] **Step 7: Run it and verify it fails**

Run: `pnpm test -- icons`
Expected: FAIL — cannot resolve `./icons`.

- [ ] **Step 8: Write `src/lib/icons.ts`**

Each glyph is a list of SVG `d` strings drawn on a `0 0 24 24` viewBox for a 2px `currentColor` stroke with round caps and joins — the same construction as the inline SVGs already in `+page.svelte`, so a bundled icon tints to whatever colour its container sets. Paths only, no `<circle>`: it keeps the renderer a single `{#each}` and means no component ever needs `{@html}`.

```ts
// The bundled icon set: 24 line glyphs, each a list of SVG path `d` strings on
// a 0 0 24 24 viewBox, stroked in currentColor. Bundled icons are inline and
// therefore tintable, which is why a group's sidebar entry can take its
// group's colour. A *custom* icon is never inlined — see customIconSrc below.
//
// An unknown name resolves to FALLBACK_ICON rather than failing. `core`
// deliberately does not own this list (it validates only that a group's icon
// name is non-empty), so a record naming an icon this build has never heard of
// is a normal state, not an error.

const GLYPHS: Record<string, readonly string[]> = {
  folder: ["M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"],
  briefcase: [
    "M3 9a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    "M9 7V5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2",
    "M3 13h18",
  ],
  code: ["M8 6l-5 6 5 6", "M16 6l5 6-5 6", "M14 4l-4 16"],
  terminal: ["M4 6l6 6-6 6", "M13 18h7"],
  gamepad: [
    "M7 8h10a5 5 0 0 1 5 5v1a4 4 0 0 1-7 2.6l-1-1H8l-1 1A4 4 0 0 1 0 14v-1a5 5 0 0 1 5-5z",
    "M6 12h3",
    "M7.5 10.5v3",
    "M16 11.5h.01",
    "M18 13.5h.01",
  ],
  palette: [
    "M12 3a9 9 0 1 0 0 18 2 2 0 0 0 1.6-3.2 2 2 0 0 1 1.6-3.2H18a3 3 0 0 0 3-3A9 9 0 0 0 12 3z",
    "M7.5 11.5h.01",
    "M10 8h.01",
    "M14.5 8h.01",
  ],
  book: ["M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2z", "M6 17h13"],
  music: ["M9 18V6l10-2v12", "M6 18a3 3 0 1 0 3 3 3 3 0 0 0-3-3z", "M16 16a3 3 0 1 0 3 3 3 3 0 0 0-3-3z"],
  camera: [
    "M3 9a2 2 0 0 1 2-2h2l1.5-2h7L17 7h2a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z",
    "M12 10a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7z",
  ],
  film: ["M3 4h18v16H3z", "M7 4v16", "M17 4v16", "M3 12h18", "M3 8h4", "M17 8h4", "M3 16h4", "M17 16h4"],
  flask: ["M9 3v6L4 19a2 2 0 0 0 1.8 2h12.4A2 2 0 0 0 20 19l-5-10V3", "M8 3h8", "M7 14h10"],
  rocket: [
    "M12 2c3 2.5 5 6.5 5 11l-2.5 3h-5L7 13c0-4.5 2-8.5 5-11z",
    "M9.5 19c-1 1.5-1 3 0 3s2-1 2-3",
    "M14.5 19c1 1.5 1 3 0 3s-2-1-2-3",
    "M12 10h.01",
  ],
  globe: ["M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z", "M3 12h18", "M12 3c2.5 2.6 3.8 5.7 3.8 9S14.5 18.4 12 21c-2.5-2.6-3.8-5.7-3.8-9S9.5 5.6 12 3z"],
  database: ["M4 6c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3z", "M4 6v12c0 1.7 3.6 3 8 3s8-1.3 8-3V6", "M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3"],
  server: ["M3 4h18v6H3z", "M3 14h18v6H3z", "M7 7h.01", "M7 17h.01"],
  cpu: ["M7 7h10v10H7z", "M4 9h3", "M4 15h3", "M17 9h3", "M17 15h3", "M9 4v3", "M15 4v3", "M9 17v3", "M15 17v3"],
  box: ["M12 3l8 4.5v9L12 21l-8-4.5v-9z", "M4 7.5l8 4.5 8-4.5", "M12 12v9"],
  layers: ["M12 3l9 5-9 5-9-5z", "M3 13l9 5 9-5", "M3 17l9 5 9-5"],
  pen: ["M4 20l4-1 11-11a2.1 2.1 0 0 0-3-3L5 16z", "M14 6l4 4"],
  wrench: ["M20 6a5 5 0 0 1-6.6 6.6L6 20a2.1 2.1 0 0 1-3-3l7.4-7.4A5 5 0 0 1 17 4z"],
  heart: ["M12 20S3.5 14.5 3.5 9A4.5 4.5 0 0 1 12 6.8 4.5 4.5 0 0 1 20.5 9c0 5.5-8.5 11-8.5 11z"],
  star: ["M12 3.5l2.6 5.3 5.8.8-4.2 4.1 1 5.8-5.2-2.7-5.2 2.7 1-5.8L3.6 9.6l5.8-.8z"],
  flag: ["M5 21V4", "M5 4h11l-2 3.5L16 11H5"],
  home: ["M4 11l8-7 8 7", "M6 9.5V20h12V9.5", "M10 20v-5h4v5"],
};

export const ICON_NAMES: readonly string[] = Object.keys(GLYPHS);

export const FALLBACK_ICON = "folder";

const CUSTOM_PREFIX = "custom:";

// A custom icon is stored as "custom:<slug>". It is never drawn inline, so
// iconPaths falls back for it — the caller checks isCustomIcon first and
// renders an <img> instead.
export function isCustomIcon(name: string | null | undefined): boolean {
  return typeof name === "string" && name.startsWith(CUSTOM_PREFIX);
}

export function customIconName(name: string): string {
  return name.startsWith(CUSTOM_PREFIX) ? name.slice(CUSTOM_PREFIX.length) : name;
}

export function iconPaths(name: string | null | undefined): readonly string[] {
  if (typeof name === "string" && !isCustomIcon(name)) {
    const paths = GLYPHS[name];
    if (paths) return paths;
  }
  return GLYPHS[FALLBACK_ICON];
}

// The one way a custom icon reaches the DOM. `core` returns sanitized SVG
// source rather than a data URI so it needs no base64 dependency for
// something the browser assembles in one expression.
//
// This is deliberately a data: URI for an <img>, never inlined markup. The
// sanitizer is the first security layer; an <img> is the second, and it
// cannot execute script or fetch anything regardless of what the first one
// missed. `img-src 'self' data:` is already in both content security
// policies. Reaching for {@html} here collapses two independent layers into
// one — do not.
export function customIconSrc(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}
```

- [ ] **Step 9: Run the icon test**

Run: `pnpm test -- icons`
Expected: PASS. If "has 24 unique names" fails, count the keys in `GLYPHS` — there must be exactly 24.

- [ ] **Step 10: Write `src/lib/components/BundledIcon.svelte`**

```svelte
<script lang="ts">
  import { iconPaths } from "$lib/icons";

  // Renders one glyph from the bundled set, stroked in currentColor so the
  // container decides the colour. An unknown name draws the fallback glyph
  // rather than nothing — a record from a newer build must still render.
  let {
    name,
    class: klass = "h-4 w-4",
  }: {
    name: string | null | undefined;
    class?: string;
  } = $props();

  const paths = $derived(iconPaths(name));
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  class={klass}
  aria-hidden="true"
>
  {#each paths as d}
    <path {d} />
  {/each}
</svg>
```

- [ ] **Step 11: Run the full gates**

Run: `pnpm test && pnpm run check`
Expected: all tests pass; check reports **0 errors, 8 warnings**.

- [ ] **Step 12: Commit**

```bash
git add src/app.css src/lib/palette.ts src/lib/palette.test.ts src/lib/icons.ts src/lib/icons.test.ts src/lib/components/BundledIcon.svelte
git commit -m "$(cat <<'MSG'
feat(ui): add the swatch palette and the bundled icon set

Eight --color-swatch-* tokens in the @theme block, palette.ts resolving a
stored palette *name* to one of them, and 24 line glyphs keyed by name with
a folder fallback for anything unrecognised.

Both are pure modules so they test without mounting a component, and both
follow the ignore-what-you-do-not-know rule: an unknown colour resolves to
neutral and an unknown icon draws the fallback, because core does not own
either list and a record from a newer build must still render.

customIconSrc builds the data: URI for a custom icon. Custom icons render
only through <img> — the sanitizer is one security layer and the img is the
second, and inlining would collapse them into one.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 3: `views.ts` — the pure view resolution

**Files:**
- Create: `src/lib/views.ts`, `src/lib/views.test.ts`
- Modify: `src/lib/api/types.ts` (add `Group`, `UpdateGroup`, `StoredIcon`; fix the stale `src-tauri/src/models/*` path comments)
- Create: `src/lib/api/groups.ts`, `src/lib/api/icons.ts`

**Interfaces:**
- Consumes: `Project` from `types.ts`.
- Produces:
  - `type View = { kind: "all" } | { kind: "favorites" } | { kind: "group"; id: string } | { kind: "ungrouped" } | { kind: "bin" }`
  - `viewKey(view: View): string` / `parseViewKey(key: string | null): View | null`
  - `viewLabel(view: View, groups: Group[]): string`
  - `matchesQuery(project: Project, query: string): boolean`
  - `resolveView(view: View, live: Project[], deleted: Project[], query: string): Project[]`
  - `interface ViewCounts { all: number; favorites: number; ungrouped: number; bin: number; groups: Record<string, number> }`
  - `viewCounts(live: Project[], deleted: Project[], groups: Group[]): ViewCounts`
  - `listGroups/createGroup/updateGroup/deleteGroup/reorderGroups` in `api/groups.ts`
  - `listCustomIcons/importCustomIcon/deleteCustomIcon` in `api/icons.ts`

- [ ] **Step 1: Add the missing types to `src/lib/api/types.ts`**

Replace the file's first two comment lines with:

```ts
// Mirrors the Rust models in crates/core/src/domain/ (project.rs,
// update_project.rs, group.rs, update_group.rs, tracker.rs, git.rs,
// unreal.rs, sorting.rs, installed_app.rs) and crates/core/src/infra/
// icon_store.rs. Hand-maintained — nothing checks it, so update it in the
// same commit as anything it mirrors.
// Dates stay as ISO strings (chrono::DateTime<Utc> serializes to RFC3339).
```

Then append at the end of the file:

```ts
// Mirrors crates/core/src/domain/group.rs. `color` is a palette name (see
// palette.ts); `icon` is a bundled icon name (see icons.ts) — core validates
// only that it is non-empty, because it does not own the bundled set.
export interface Group {
  id: string;
  name: string;
  color: string;
  icon: string;
  position: number;
  created_at: string;
  updated_at: string;
}

// Mirrors crates/core/src/domain/update_group.rs. Unlike UpdateProject, no
// field here is nullable — a group always has a name, a colour and an icon —
// so an omitted key means unchanged and there is no "clear" case.
// `position` is deliberately absent: reordering goes through reorderGroups,
// which rewrites the whole ordering.
export interface UpdateGroup {
  name?: string;
  color?: string;
  icon?: string;
}

// Mirrors crates/core/src/infra/icon_store.rs. `svg` is sanitized SVG
// *source*, not a data URI — core returns source so it needs no base64
// dependency; the browser assembles the URI in one expression. See
// customIconSrc in src/lib/icons.ts, which is the only place that happens.
export interface StoredIcon {
  name: string;
  svg: string;
}
```

- [ ] **Step 2: Write `src/lib/api/groups.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { Group, UpdateGroup } from "./types";

export async function listGroups(): Promise<Group[]> {
  try {
    return await invoke<Group[]>("list_groups");
  } catch (err) {
    throw toError(err);
  }
}

export async function createGroup(name: string, color: string, icon: string): Promise<Group> {
  try {
    return await invoke<Group>("create_group", { name, color, icon });
  } catch (err) {
    throw toError(err);
  }
}

export async function updateGroup(id: string, update: UpdateGroup): Promise<Group> {
  try {
    return await invoke<Group>("update_group", { id, update });
  } catch (err) {
    throw toError(err);
  }
}

// Members become Ungrouped. No project is deleted by this.
export async function deleteGroup(id: string): Promise<void> {
  try {
    await invoke<void>("delete_group", { id });
  } catch (err) {
    throw toError(err);
  }
}

// Rewrites the whole sidebar ordering and returns what was persisted, so the
// caller renders that rather than what it hoped for.
//
// The Rust parameter is `ordered_ids`; Tauri 2 converts a camelCase key, the
// same way `deleteMetadata` reaches `delete_metadata` in projects.ts. If it
// ever arrives as null, pass `ordered_ids` instead — do not change the Rust
// signature to dodge it.
export async function reorderGroups(orderedIds: string[]): Promise<Group[]> {
  try {
    return await invoke<Group[]>("reorder_groups", { orderedIds });
  } catch (err) {
    throw toError(err);
  }
}

// ProjectError::GroupNotFound, surfaced as this exact string. Reachable
// through ordinary use: an edit form open while the group is deleted from the
// group manager produces it. The backend rejects the write and changes
// nothing, so the caller refetches the group list and clears the stale
// selection — retrying is guaranteed to fail the same way. Clearing a group
// (group_id: null) is always allowed and checks nothing.
export function isGroupNotFound(err: unknown): boolean {
  const message = err instanceof Error ? err.message : String(err);
  return /^Group with id '.*' not found$/.test(message);
}
```

- [ ] **Step 3: Write `src/lib/api/icons.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";
import { toError } from "./errors";
import type { StoredIcon } from "./types";

export async function listCustomIcons(): Promise<StoredIcon[]> {
  try {
    return await invoke<StoredIcon[]>("list_custom_icons");
  } catch (err) {
    throw toError(err);
  }
}

// Reads an SVG from `path`, sanitizes it, and stores the sanitized form — the
// original is never copied.
//
// This fails for *content* reasons as often as IO ones: an undefined entity,
// a bare `&`, `&nbsp;` (an HTML entity XML does not define, common in
// HTML-flavoured exports), nothing drawable left after sanitizing, over
// 256 KB, or a backslash in an attribute value. Every one of those names its
// reason in the message and every one is something the user can fix, so
// callers must surface `err.message` rather than a generic failure.
export async function importCustomIcon(path: string): Promise<StoredIcon> {
  try {
    return await invoke<StoredIcon>("import_custom_icon", { path });
  } catch (err) {
    throw toError(err);
  }
}

export async function deleteCustomIcon(name: string): Promise<void> {
  try {
    await invoke<void>("delete_custom_icon", { name });
  } catch (err) {
    throw toError(err);
  }
}
```

- [ ] **Step 4: Write the failing `views.ts` test**

Create `src/lib/views.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { Group, Project } from "./api/types";
import {
  matchesQuery,
  parseViewKey,
  resolveView,
  viewCounts,
  viewKey,
  viewLabel,
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
  client: null,
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
    for (const view of [
      { kind: "all" },
      { kind: "favorites" },
      { kind: "ungrouped" },
      { kind: "bin" },
      { kind: "group", id: "g-work" },
    ] as const) {
      expect(parseViewKey(viewKey(view))).toEqual(view);
    }
  });
  it("round-trips a group id containing a colon", () => {
    const view = { kind: "group", id: "a:b:c" } as const;
    expect(parseViewKey(viewKey(view))).toEqual(view);
  });
  it("returns null for junk rather than throwing", () => {
    expect(parseViewKey("nonsense")).toBeNull();
    expect(parseViewKey("group:")).toBeNull();
    expect(parseViewKey("")).toBeNull();
    expect(parseViewKey(null)).toBeNull();
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
  const p = project({ name: "Friction Engine", directory: "D:\\Games\\fe", tags: ["rust", "game"] });
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
```

- [ ] **Step 5: Run it and verify it fails**

Run: `pnpm test -- views`
Expected: FAIL — cannot resolve `./views`.

- [ ] **Step 6: Write `src/lib/views.ts`**

```ts
import type { Group, Project } from "./api/types";

// The sidebar selects a View, which is a data source plus an action set.
// Groups are navigation, not sections: selecting one changes what the main
// list shows, and the current selection stays visible. There is deliberately
// no collapsible-band state anywhere in this design — a collapsed band hides
// projects with nothing on screen saying they exist.
export type View =
  | { kind: "all" }
  | { kind: "favorites" }
  | { kind: "group"; id: string }
  | { kind: "ungrouped" }
  | { kind: "bin" };

const GROUP_PREFIX = "group:";

// A stable string form, for localStorage and for {#each} keys.
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

// Name, path and tags — the three things a row shows that identify a project.
// An empty query matches everything so the caller does not special-case it.
export function matchesQuery(project: Project, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return true;
  if (project.name.toLowerCase().includes(q)) return true;
  if (project.directory.toLowerCase().includes(q)) return true;
  return project.tags.some((tag) => tag.toLowerCase().includes(q));
}

// `live` is get_all_projects (non-deleted, already sorted by the backend) and
// `deleted` is get_deleted_projects. Filtering is order-preserving, so
// whatever sort the backend applied still holds inside every view — which is
// why this module does not sort. Re-implementing core::domain::sorting in
// TypeScript would be a second cross-language mirror with nothing checking it.
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
// show a 0 rather than vanish — hence the seed loop over every group.
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
```

- [ ] **Step 7: Run the tests**

Run: `pnpm test`
Expected: PASS — the `trackers`, `palette`, `icons` and `views` suites all green.

- [ ] **Step 8: Run check and commit**

Run: `pnpm run check`
Expected: 0 errors, 8 warnings.

```bash
git add src/lib/views.ts src/lib/views.test.ts src/lib/api/types.ts src/lib/api/groups.ts src/lib/api/icons.ts
git commit -m "$(cat <<'MSG'
feat(views): add the pure view-resolution module and the group/icon APIs

views.ts resolves a sidebar View to a project list and computes the per-view
counts, with no component mounted and no fetch of its own.

It deliberately does not sort. get_all_projects and get_deleted_projects
already sort in core::domain::sorting, tie-broken by the unique id, and
filtering an already-sorted list preserves that order — so a TypeScript
comparator would be a second cross-language mirror with nothing checking it,
which is the drift hazard types.ts is already flagged for.

Favourites derives from the live list rather than fetching: list_favorites
filters !is_deleted && favorite then sorts, and because the sort is a total
order that is the same list get_favorite_projects returns. It also makes the
sidebar count and the list read the same array.

types.ts gains Group, UpdateGroup and StoredIcon, and its stale
src-tauri/src/models/* path comments now point at crates/core/src/domain/.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 4: Split `ProjectCard` — no behaviour change

**This is a pure refactor and must be verified as such before any new view mode exists.** List mode has to look and behave exactly as it does today. Building the grid tile first is the tempting mistake; this split is what makes the tile cheap.

**Files:**
- Create: `src/lib/components/ProjectActionsMenu.svelte`, `src/lib/components/ProjectMark.svelte`, `src/lib/components/ProjectRow.svelte`, `src/lib/components/ProjectTile.svelte`, `src/lib/components/ProjectCompactRow.svelte`
- Modify: `src/lib/components/ProjectList.svelte`
- Modify: `src/routes/+page.svelte` (pass `customIcons` through)
- Delete: `src/lib/components/ProjectCard.svelte`

**Interfaces:**
- Consumes: `swatchVar` (Task 2), `BundledIcon` (Task 2), `isCustomIcon`/`customIconName`/`customIconSrc` (Task 2).
- Produces:
  - `<ProjectActionsMenu {project} {onEdit} {onRequestDelete} {onOpened} {onTrackersRefreshed} {onOpenWithAppMissing} {onerror} />`
  - `<ProjectMark {project} {customIcons} size?="sm"|"lg" />`
  - `<ProjectRow {project} {directoryMissing} {customIcons} {groupColor} {actions} />` where `actions: Snippet`
  - `<ProjectTile …/>`, `<ProjectCompactRow …/>` with the same prop list
  - `ProjectList` gains `customIcons: Map<string, string>` and `groups: Group[]`

**Note on `customIcons`.** It is a `Map<name → data URI>`, built once in `+page.svelte` from `listCustomIcons()` and drilled down. There is no store precedent in this codebase — `architecture.md` lists `lib/stores/*` as deliberately deferred ("watch it, don't pre-split") — so drilling one more prop follows the existing pattern rather than importing a new one.

**Note on the `style` attribute.** `swatchVar()` returns one of nine fixed `var(--color-*)` strings and can return nothing else, so setting `style="--mark: {swatchVar(...)}"` cannot interpolate an arbitrary string into CSS. `style-src` already carries `unsafe-inline` for `trackerColor(kind)`; this adds no new exposure, and using a custom property is the shape that makes tightening it possible later.

- [ ] **Step 1: Record the "before" so the no-change claim is checkable**

Run `pnpm tauri dev`, and with at least two projects registered (one favourited, one with tags, one with a missing directory if you can arrange it) capture the list view — a screenshot or a careful note of: row order, the `> name` line with its accent chevron, the star, the missing-directory bin glyph, the path line, description, tag chips, tracker badges, and the `···` menu's five items (Open, Details, Detect type, Edit, Delete) with Delete's rust hover.

- [ ] **Step 2: Create `src/lib/components/ProjectActionsMenu.svelte`**

Move the menu out of `ProjectCard.svelte` verbatim — same markup, same `menuItem` string, same handlers, same `Escape` binding:

```svelte
<script lang="ts">
  import { isOpenWithAppMissing, openProjectDirectory } from "$lib/api/opener";
  import { refreshProjectTrackers } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass } from "./styles";

  let {
    project,
    onEdit,
    onRequestDelete,
    onOpened,
    onTrackersRefreshed,
    onOpenWithAppMissing,
    onerror,
  }: {
    project: Project;
    onEdit: (project: Project) => void;
    onRequestDelete: (project: Project) => void;
    onOpened: () => void | Promise<void>;
    onTrackersRefreshed: () => void | Promise<void>;
    onOpenWithAppMissing: (project: Project) => void;
    onerror?: (message: string) => void;
  } = $props();

  let refreshing = $state(false);
  let menuOpen = $state(false);

  const menuItem =
    "px-3 py-1.5 text-left font-display text-[14px] text-phos-dim hover:bg-panel-2 hover:text-phos disabled:cursor-default disabled:text-phos-faint disabled:hover:bg-transparent disabled:hover:text-phos-faint";

  async function handleOpen() {
    try {
      await openProjectDirectory(project.id);
      await onOpened();
    } catch (err) {
      if (isOpenWithAppMissing(err)) {
        onOpenWithAppMissing(project);
      } else {
        onerror?.((err as Error).message);
      }
    }
  }

  async function handleRefreshTrackers() {
    refreshing = true;
    try {
      await refreshProjectTrackers(project.id);
      await onTrackersRefreshed();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      refreshing = false;
    }
  }
</script>

<div class="relative shrink-0">
  <button
    type="button"
    onclick={() => (menuOpen = !menuOpen)}
    class={buttonClass}
    aria-haspopup="menu"
    aria-expanded={menuOpen}
    aria-label="Project actions"
  >
    ···
  </button>

  {#if menuOpen}
    <button
      type="button"
      class="fixed inset-0 z-10 cursor-default"
      tabindex="-1"
      aria-label="Close menu"
      onclick={() => (menuOpen = false)}
    ></button>
    <div
      role="menu"
      class="absolute right-0 z-20 mt-1 flex min-w-40 flex-col rounded-sm border border-line bg-panel py-1"
    >
      <button
        role="menuitem"
        type="button"
        class={menuItem}
        onclick={() => {
          menuOpen = false;
          handleOpen();
        }}
      >
        Open
      </button>
      <a role="menuitem" href={`/project/${project.id}`} class={menuItem} onclick={() => (menuOpen = false)}>
        Details
      </a>
      <button
        role="menuitem"
        type="button"
        class={menuItem}
        disabled={refreshing}
        onclick={() => {
          menuOpen = false;
          handleRefreshTrackers();
        }}
      >
        {refreshing ? "Detecting…" : "Detect type"}
      </button>
      <button
        role="menuitem"
        type="button"
        class={menuItem}
        onclick={() => {
          menuOpen = false;
          onEdit(project);
        }}
      >
        Edit
      </button>
      <button
        role="menuitem"
        type="button"
        class={`${menuItem} hover:bg-rust! hover:text-void!`}
        onclick={() => {
          menuOpen = false;
          onRequestDelete(project);
        }}
      >
        Delete
      </button>
    </div>
  {/if}
</div>

<svelte:window
  onkeydown={(e) => {
    if (e.key === "Escape") menuOpen = false;
  }}
/>
```

- [ ] **Step 3: Create `src/lib/components/ProjectMark.svelte`**

```svelte
<script lang="ts">
  import { customIconSrc, customIconName, isCustomIcon } from "$lib/icons";
  import { swatchVar } from "$lib/palette";
  import type { Project } from "$lib/api/types";
  import BundledIcon from "./BundledIcon.svelte";

  // A project's identity mark: its icon, plus a colour pip.
  //
  // The *pip* carries project colour, not the icon, because a custom icon is
  // an <img> and cannot be tinted — this is what makes colour read identically
  // for bundled and custom icons. A bundled icon additionally tints to the
  // same colour, since it is inline and can.
  let {
    project,
    customIcons,
    size = "sm",
  }: {
    project: Project;
    customIcons: Map<string, string>;
    size?: "sm" | "lg";
  } = $props();

  const color = $derived(swatchVar(project.color));
  const box = $derived(size === "lg" ? "h-7 w-7" : "h-5 w-5");
  const glyph = $derived(size === "lg" ? "h-5 w-5" : "h-4 w-4");
  const pip = $derived(size === "lg" ? "h-2 w-2" : "h-1.5 w-1.5");
  const custom = $derived(
    isCustomIcon(project.icon) ? customIcons.get(customIconName(project.icon!)) : undefined,
  );
</script>

<span class={`relative inline-flex shrink-0 items-center justify-center ${box}`}>
  {#if custom}
    <!-- A custom icon renders only through an <img> with a data: URI. It is
         inert regardless of what the sanitizer missed, which is the second of
         two independent security layers. Never {@html}. -->
    <img src={custom} alt="" class={glyph} />
  {:else}
    <span style={`color: ${color}`} class="inline-flex">
      <BundledIcon name={project.icon} class={glyph} />
    </span>
  {/if}
  {#if project.color}
    <span
      class={`absolute -bottom-0.5 -right-0.5 rounded-full ${pip}`}
      style={`background: ${color}`}
      title={project.color}
    ></span>
  {/if}
</span>
```

- [ ] **Step 4: Create `src/lib/components/ProjectRow.svelte`**

This is `ProjectCard`'s markup with the menu replaced by the `actions` snippet, plus the mark and the group's left edge. **The mark and the edge only render when the project actually has an icon/colour or a group**, so a project with none looks exactly as it does today.

```svelte
<script lang="ts">
  import type { Project } from "$lib/api/types";
  import type { Snippet } from "svelte";
  import ProjectMark from "./ProjectMark.svelte";
  import TrackerBadges from "./TrackerBadges.svelte";

  let {
    project,
    directoryMissing = false,
    customIcons,
    groupColor = null,
    actions,
  }: {
    project: Project;
    directoryMissing?: boolean;
    customIcons: Map<string, string>;
    // The group's palette name, or null in a single-group view and for an
    // ungrouped project. The left edge earns its place in the mixed views —
    // All and Favourites — where projects from different groups sit together
    // and the edge is what tells you which is which.
    groupColor?: string | null;
    actions: Snippet;
  } = $props();
</script>

<div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
  <div class="min-w-0 flex-1">
    <div class="flex min-w-0 items-center gap-2">
      {#if project.icon || project.color}
        <ProjectMark {project} {customIcons} />
      {/if}
      <strong class="min-w-0 truncate font-display text-[15px] text-phos">
        <span class="text-accent">&gt;</span>&nbsp;{project.name}
      </strong>
      {#if project.favorite}<span class="shrink-0 text-gold" title="Favorite">★</span>{/if}
      {#if directoryMissing}
        <span class="shrink-0 text-amber" title="Directory deleted or moved">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="h-4 w-4"
          >
            <path d="M3 6h18" />
            <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
            <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
            <path d="M10 11v6" />
            <path d="M14 11v6" />
          </svg>
        </span>
      {/if}
    </div>
    <div
      class={`truncate text-[12px] ${directoryMissing ? "text-phos-faint line-through" : "text-phos-dim"}`}
    >
      {project.directory}
    </div>
    {#if project.description}
      <p class="mt-1 text-sm text-phos-dim">{project.description}</p>
    {/if}
    {#if project.tags.length > 0}
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#each project.tags as tag}
          <span
            class="rounded-sm border border-line px-1.5 py-0.5 text-[10px] uppercase tracking-wide text-phos-dim"
          >
            {tag}
          </span>
        {/each}
      </div>
    {/if}
    <TrackerBadges trackers={project.trackers} />
  </div>
  {@render actions()}
</div>
```

Note: `groupColor` is declared here but the left edge is drawn by `ProjectList` on the `<li>`, so the border sits outside the row's padding. Keep the prop for symmetry with the tile, which draws its own.

- [ ] **Step 5: Create `src/lib/components/ProjectTile.svelte`**

```svelte
<script lang="ts">
  import type { Project } from "$lib/api/types";
  import type { Snippet } from "svelte";
  import { swatchVar } from "$lib/palette";
  import ProjectMark from "./ProjectMark.svelte";
  import TrackerBadges from "./TrackerBadges.svelte";

  let {
    project,
    directoryMissing = false,
    customIcons,
    groupColor = null,
    actions,
  }: {
    project: Project;
    directoryMissing?: boolean;
    customIcons: Map<string, string>;
    groupColor?: string | null;
    actions: Snippet;
  } = $props();
</script>

<div
  class="flex h-full flex-col gap-2 border-l-2 pl-3"
  style={`border-color: ${groupColor ? swatchVar(groupColor) : "transparent"}`}
>
  <div class="flex items-start justify-between gap-2">
    <ProjectMark {project} {customIcons} size="lg" />
    {@render actions()}
  </div>
  <div class="flex min-w-0 items-center gap-1.5">
    <strong class="min-w-0 truncate font-display text-[15px] text-phos">{project.name}</strong>
    {#if project.favorite}<span class="shrink-0 text-gold" title="Favorite">★</span>{/if}
    {#if directoryMissing}
      <span class="shrink-0 text-amber" title="Directory deleted or moved">!</span>
    {/if}
  </div>
  <div
    class={`truncate text-[11px] ${directoryMissing ? "text-phos-faint line-through" : "text-phos-dim"}`}
    title={project.directory}
  >
    {project.directory}
  </div>
  <div class="mt-auto">
    <TrackerBadges trackers={project.trackers} />
  </div>
</div>
```

- [ ] **Step 6: Create `src/lib/components/ProjectCompactRow.svelte`**

```svelte
<script lang="ts">
  import type { Project } from "$lib/api/types";
  import type { Snippet } from "svelte";
  import ProjectMark from "./ProjectMark.svelte";

  let {
    project,
    directoryMissing = false,
    customIcons,
    groupColor = null,
    actions,
  }: {
    project: Project;
    directoryMissing?: boolean;
    customIcons: Map<string, string>;
    groupColor?: string | null;
    actions: Snippet;
  } = $props();
</script>

<div class="flex min-w-0 items-center gap-2">
  <ProjectMark {project} {customIcons} />
  <strong class="shrink-0 truncate font-display text-[14px] text-phos">{project.name}</strong>
  {#if project.favorite}<span class="shrink-0 text-gold" title="Favorite">★</span>{/if}
  <span
    class={`min-w-0 flex-1 truncate text-[11px] ${directoryMissing ? "text-phos-faint line-through" : "text-phos-dim"}`}
    title={project.directory}
  >
    {project.directory}
  </span>
  {@render actions()}
</div>
```

- [ ] **Step 7: Rewrite `src/lib/components/ProjectList.svelte`**

Still list-only at this task — the mode prop arrives in Task 6. The `<li>` keeps today's `rounded-sm border border-line p-3`, and gains only a left edge when the project has a group colour.

```svelte
<script lang="ts">
  import type { Group, Project } from "$lib/api/types";
  import { swatchVar } from "$lib/palette";
  import EditProjectForm from "./EditProjectForm.svelte";
  import ProjectActionsMenu from "./ProjectActionsMenu.svelte";
  import ProjectRow from "./ProjectRow.svelte";
  import { cardClass } from "./styles";

  let {
    projects,
    groups,
    customIcons,
    loading,
    editingId,
    missingDirs,
    onEdit,
    onCancelEdit,
    onSaved,
    onRequestDelete,
    onOpened,
    onTrackersRefreshed,
    onOpenWithAppMissing,
    onerror,
  }: {
    projects: Project[];
    groups: Group[];
    customIcons: Map<string, string>;
    loading: boolean;
    editingId: string | null;
    missingDirs: Set<string>;
    onEdit: (project: Project) => void;
    onCancelEdit: () => void;
    onSaved: () => void | Promise<void>;
    onRequestDelete: (project: Project) => void;
    onOpened: () => void | Promise<void>;
    onTrackersRefreshed: () => void | Promise<void>;
    onOpenWithAppMissing: (project: Project) => void;
    onerror: (message: string) => void;
  } = $props();

  // The group's palette *name*, resolved per project. Null when the project
  // has no group or its group has since been deleted.
  function groupColorOf(project: Project): string | null {
    if (!project.group_id) return null;
    return groups.find((g) => g.id === project.group_id)?.color ?? null;
  }
</script>

<section class={cardClass}>
  <h2 class="mb-3 font-display text-[14px] uppercase tracking-wide text-phos-dim"><span class="text-gold">//</span> projects</h2>
  {#if loading && projects.length === 0}
    <!-- Only on a cold start (empty + loading). A re-sort or a create/delete
         refetches too, but it's a local store read — instant — so keep the
         current list on screen rather than flashing this. -->
    <p class="text-sm text-phos-dim">Loading…</p>
  {:else if projects.length === 0}
    <p class="text-sm text-phos-dim">No projects yet.</p>
  {:else}
    <ul class="flex flex-col gap-3">
      {#each projects as project (project.id)}
        {@const groupColor = groupColorOf(project)}
        <li
          class="rounded-sm border border-line p-3"
          style={groupColor ? `border-left: 2px solid ${swatchVar(groupColor)}` : undefined}
        >
          {#if editingId === project.id}
            <EditProjectForm {project} {onSaved} onCancel={onCancelEdit} {onerror} />
          {:else}
            <ProjectRow
              {project}
              directoryMissing={missingDirs.has(project.id)}
              {customIcons}
              {groupColor}
            >
              {#snippet actions()}
                <ProjectActionsMenu
                  {project}
                  {onEdit}
                  {onRequestDelete}
                  {onOpened}
                  {onTrackersRefreshed}
                  {onOpenWithAppMissing}
                  {onerror}
                />
              {/snippet}
            </ProjectRow>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>
```

- [ ] **Step 8: Wire the new props in `src/routes/+page.svelte`**

Add to the script block:

```ts
  import { listGroups } from "$lib/api/groups";
  import { listCustomIcons } from "$lib/api/icons";
  import { customIconSrc } from "$lib/icons";
  import type { Group } from "$lib/api/types";

  let groups = $state<Group[]>([]);
  let customIcons = $state<Map<string, string>>(new Map());

  // Best-effort, both of them: a failure to load groups or icons must not
  // stop the project list rendering. A missing group just means no left edge;
  // a missing custom icon falls back to the bundled glyph.
  async function loadGroups() {
    try {
      groups = await listGroups();
    } catch (err) {
      error = (err as Error).message;
    }
  }

  async function loadCustomIcons() {
    try {
      const icons = await listCustomIcons();
      customIcons = new Map(icons.map((i) => [i.name, customIconSrc(i.svg)]));
    } catch {
      customIcons = new Map();
    }
  }
```

Call both from the existing `$effect`:

```ts
  $effect(() => {
    loadProjects();
    loadGroups();
    loadCustomIcons();
  });
```

and pass them to `ProjectList`:

```svelte
  <ProjectList
    {projects}
    {groups}
    {customIcons}
    {loading}
    …
```

- [ ] **Step 9: Delete `ProjectCard.svelte`**

```bash
git rm src/lib/components/ProjectCard.svelte
```

Then confirm nothing still imports it:

Run: `grep -rn "ProjectCard" src/`
Expected: no output.

- [ ] **Step 10: Verify — this is the step the task exists for**

Run: `pnpm run check`
Expected: **0 errors, 8 warnings.** A 9th warning or any error means stop.

Run: `pnpm tauri dev`
Compare against Step 1's record. Every one of these must be true:
- Row order, spacing and border identical.
- `> Name` with the accent chevron, the gold star on a favourite, the amber bin glyph on a missing directory.
- Path line, description, tag chips, tracker badges all as before.
- The `···` menu opens, closes on Escape, closes on outside click, and has Open / Details / Detect type / Edit / Delete with Delete going rust on hover.
- Edit-in-place still swaps the row for `EditProjectForm`.
- A project with no colour and no icon shows **no** mark and **no** left edge — visually identical to today.

- [ ] **Step 11: Commit**

```bash
git add -A src/lib/components src/routes/+page.svelte
git commit -m "$(cat <<'MSG'
refactor(ui): split ProjectCard into behaviour and three presentations

ProjectActionsMenu and ProjectMark come out of ProjectCard, then ProjectRow,
ProjectTile and ProjectCompactRow render the same project three ways. Each
presentation takes its action set as a snippet prop, which is what will let
one set of layouts serve both the standard menu and Bin's Restore/Purge pair
without a second copy of the row markup.

No behaviour change: list mode is byte-for-byte the same markup it was, and
was checked against a running app before and after. The mark and the group's
left edge render only when a project actually has an icon, a colour or a
group, so nothing that exists today looks different.

ProjectCard.svelte is deleted; ProjectTile and ProjectCompactRow are not
rendered yet — the mode toggle that reaches them arrives next.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 5: `Sidebar` + `SidebarEntry`, and the page layout

All / groups / Ungrouped only. Favourites and Bin stay as header buttons opening their modals until Tasks 7 and 8 retire them — that is what keeps each retirement independently verifiable.

**Files:**
- Create: `src/lib/components/Sidebar.svelte`, `src/lib/components/SidebarEntry.svelte`
- Modify: `src/routes/+page.svelte`

**Interfaces:**
- Consumes: `View`, `viewKey`, `viewCounts`, `resolveView` (Task 3); `swatchVar` (Task 2); `BundledIcon` (Task 2).
- Produces:
  - `<SidebarEntry {icon} {label} {count} {color} {selected} {onSelect} />`
  - `<Sidebar {groups} {counts} {selected} {onSelect} {showFavorites} {showBin} {onManageGroups} />` — `showFavorites` / `showBin` default to `false` here and flip to `true` in Tasks 7 and 8; `onManageGroups` is optional and unwired until Task 9.

- [ ] **Step 1: Create `src/lib/components/SidebarEntry.svelte`**

```svelte
<script lang="ts">
  import { swatchVar } from "$lib/palette";
  import BundledIcon from "./BundledIcon.svelte";

  // One sidebar entry: icon, colour, label, count. The count is not optional
  // by design — the spec's invariant is that no view can hide projects
  // without saying so, which means an empty group shows a 0 rather than
  // disappearing.
  let {
    icon,
    label,
    count,
    color = null,
    selected = false,
    onSelect,
  }: {
    icon: string;
    label: string;
    count: number;
    color?: string | null;
    selected?: boolean;
    onSelect: () => void;
  } = $props();
</script>

<button
  type="button"
  onclick={onSelect}
  aria-current={selected ? "page" : undefined}
  class={`flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left font-display text-[14px] ${
    selected ? "bg-panel-2 text-phos" : "text-phos-dim hover:bg-panel-2 hover:text-phos"
  }`}
>
  <span class="shrink-0" style={color ? `color: ${swatchVar(color)}` : undefined}>
    <BundledIcon name={icon} class="h-4 w-4" />
  </span>
  <span class="min-w-0 flex-1 truncate">{label}</span>
  <span class="shrink-0 text-[12px] text-phos-faint tabular-nums">{count}</span>
</button>
```

- [ ] **Step 2: Create `src/lib/components/Sidebar.svelte`**

```svelte
<script lang="ts">
  import type { Group } from "$lib/api/types";
  import type { View, ViewCounts } from "$lib/views";
  import { viewKey } from "$lib/views";
  import SidebarEntry from "./SidebarEntry.svelte";

  // The rail. Groups are navigation, not sections: the current selection is
  // always visible and so is the fact that other groups exist, which is what
  // the rejected collapsible-band design could not do. It is a fixed column
  // and does not collapse to an icon rail — two groups may legitimately pick
  // the same glyph from the bundled set, and an icon-only rail would then
  // show two entries you cannot tell apart.
  let {
    groups,
    counts,
    selected,
    onSelect,
    showFavorites = false,
    showBin = false,
    onManageGroups,
  }: {
    groups: Group[];
    counts: ViewCounts;
    selected: View;
    onSelect: (view: View) => void;
    showFavorites?: boolean;
    showBin?: boolean;
    onManageGroups?: () => void;
  } = $props();

  const key = $derived(viewKey(selected));
</script>

<nav class="flex w-52 shrink-0 flex-col gap-1" aria-label="Views">
  <SidebarEntry
    icon="layers"
    label="All"
    count={counts.all}
    selected={key === "all"}
    onSelect={() => onSelect({ kind: "all" })}
  />
  {#if showFavorites}
    <SidebarEntry
      icon="star"
      label="Favourites"
      count={counts.favorites}
      color="gold"
      selected={key === "favorites"}
      onSelect={() => onSelect({ kind: "favorites" })}
    />
  {/if}

  <div class="mt-3 flex items-center justify-between px-2">
    <span class="font-display text-[12px] uppercase tracking-wide text-phos-faint">
      <span class="text-gold">//</span> groups
    </span>
    {#if onManageGroups}
      <button
        type="button"
        onclick={onManageGroups}
        class="rounded-sm px-1 font-display text-[14px] leading-none text-phos-faint hover:text-phos"
        title="Manage groups"
        aria-label="Manage groups"
      >
        +
      </button>
    {/if}
  </div>

  {#if groups.length === 0}
    <p class="px-2 text-[12px] text-phos-faint">No groups yet.</p>
  {:else}
    {#each groups as group (group.id)}
      <SidebarEntry
        icon={group.icon}
        label={group.name}
        count={counts.groups[group.id] ?? 0}
        color={group.color}
        selected={key === viewKey({ kind: "group", id: group.id })}
        onSelect={() => onSelect({ kind: "group", id: group.id })}
      />
    {/each}
  {/if}

  <SidebarEntry
    icon="box"
    label="Ungrouped"
    count={counts.ungrouped}
    selected={key === "ungrouped"}
    onSelect={() => onSelect({ kind: "ungrouped" })}
  />

  {#if showBin}
    <div class="mt-3">
      <SidebarEntry
        icon="folder"
        label="Bin"
        count={counts.bin}
        selected={key === "bin"}
        onSelect={() => onSelect({ kind: "bin" })}
      />
    </div>
  {/if}
</nav>
```

- [ ] **Step 3: Rewrite the layout in `src/routes/+page.svelte`**

Add to the script:

```ts
  import Sidebar from "$lib/components/Sidebar.svelte";
  import { resolveView, viewCounts, type View } from "$lib/views";

  let selectedView = $state<View>({ kind: "all" });
  let deletedProjects = $state<Project[]>([]);
  let query = $state("");

  const counts = $derived(viewCounts(projects, deletedProjects, groups));
  const visibleProjects = $derived(
    resolveView(selectedView, projects, deletedProjects, query),
  );

  function handleSelectView(view: View) {
    selectedView = view;
    error = "";
  }
```

`deletedProjects` stays empty until Task 8; the Bin count reads 0 and the Bin entry is hidden, so nothing depends on it yet.

Replace the `<main>` wrapper with sidebar + content, keeping the header bar full width:

```svelte
<div class="mx-auto max-w-6xl px-4 py-8">
  <div class="mb-6 flex items-center justify-between gap-2 border-b border-line pb-3">
    <!-- header contents unchanged: title, and the Favourites/Bin buttons,
         which Tasks 7 and 8 remove as the sidebar absorbs them -->
  </div>

  <ErrorBanner message={error} />

  <div class="mt-4 flex gap-6">
    <Sidebar
      {groups}
      {counts}
      selected={selectedView}
      onSelect={handleSelectView}
    />
    <main class="min-w-0 flex-1">
      <CreateProjectForm onCreated={handleCreated} onerror={handleError} />

      <div class="mb-3 flex justify-end">
        <SortControls bind:by={sortBy} bind:direction={sortDirection} />
      </div>

      <ProjectList
        projects={visibleProjects}
        {groups}
        {customIcons}
        {loading}
        {editingId}
        {missingDirs}
        onEdit={handleEdit}
        onCancelEdit={handleCancelEdit}
        onSaved={handleSaved}
        onRequestDelete={handleRequestDelete}
        onOpened={handleOpened}
        onTrackersRefreshed={handleTrackersRefreshed}
        onOpenWithAppMissing={handleOpenWithAppMissing}
        onerror={handleError}
      />
    </main>
  </div>
</div>
```

The `max-w-3xl` becomes `max-w-6xl` because the page is now two columns.

- [ ] **Step 4: Handle a selected group that gets deleted while selected**

Add to `+page.svelte`, so a deleted group cannot leave the list rendering nothing:

```ts
  // A group can disappear underneath the selection — from the group manager,
  // or from another window. Falling back to All beats rendering an empty
  // list with no explanation.
  $effect(() => {
    if (selectedView.kind === "group" && !groups.some((g) => g.id === selectedView.id)) {
      selectedView = { kind: "all" };
    }
  });
```

- [ ] **Step 5: Verify in the app**

Run: `pnpm run check` → 0 errors, 8 warnings. Then `pnpm tauri dev`.
- The sidebar shows All with the right count, a "no groups yet" line, and Ungrouped.
- With no groups, All and Ungrouped hold the same projects and both counts match.
- Selecting Ungrouped and then All swaps the list correctly.
- The list rows still render exactly as after Task 4.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/Sidebar.svelte src/lib/components/SidebarEntry.svelte src/routes/+page.svelte
git commit -m "$(cat <<'MSG'
feat(ui): add the sidebar and switch the page to a two-column layout

All, the user's groups, and Ungrouped. Favourites and Bin are still header
buttons opening their modals — they move in as their own tasks so each
retirement is verified against the modal it replaces.

Every entry shows its count, because the spec's invariant is that no view
can hide projects without saying so: an empty group has to show a 0 rather
than disappear. Counts come from views.ts over the already-fetched arrays,
so there is no extra query and no second source of truth.

The sidebar is a fixed column and does not collapse to an icon rail. Two
groups may legitimately choose the same glyph from the 24-icon bundled set,
and an icon-only rail would then show two entries you cannot tell apart —
the same "you cannot see what is hidden" failure the collapsible-band design
was rejected for.

A group deleted while selected falls back to All rather than rendering an
empty list.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 6: `ViewControls` and `viewState.ts`

**Files:**
- Create: `src/lib/components/ViewControls.svelte`
- Create: `src/lib/viewState.ts`, `src/lib/viewState.test.ts`
- Modify: `src/lib/components/ProjectList.svelte` (accept `mode`)
- Modify: `src/routes/+page.svelte`

**Interfaces:**
- Consumes: `View`, `viewKey`, `parseViewKey` (Task 3); the three presentations (Task 4).
- Produces:
  - `type ViewMode = "list" | "grid" | "compact"`
  - `restoreViewMode(raw: string | null): ViewMode` and `restoreView(raw: string | null, groups: Group[]): View` — pure, tested
  - `loadViewMode()` / `saveViewMode(mode)` / `loadView(groups)` / `saveView(view)` — the `localStorage` wrappers
  - `<ViewControls bind:mode bind:query />`
  - `ProjectList` gains `mode: ViewMode`

**Why the pure/impure split.** vitest runs in the `node` environment (see `vite.config.ts`), where there is no `localStorage`. Splitting the restore *rules* from the storage *access* is what makes the two rules the spec actually cares about testable, and it matches the existing precedent that pure logic lives in a module and is tested without mounting anything.

**Why `localStorage` and not `projects.db`.** The database is a cross-app contract that devmon attaches read-only. UI preferences are none of its business.

- [ ] **Step 1: Write the failing `viewState` test**

Create `src/lib/viewState.test.ts`:

```ts
import { describe, expect, it } from "vitest";
import type { Group } from "./api/types";
import { restoreView, restoreViewMode } from "./viewState";

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
    expect(restoreViewMode("compact")).toBe("compact");
    expect(restoreViewMode("list")).toBe("list");
  });
  it("defaults to list for junk or nothing stored", () => {
    expect(restoreViewMode(null)).toBe("list");
    expect(restoreViewMode("")).toBe("list");
    expect(restoreViewMode("mosaic")).toBe("list");
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
```

- [ ] **Step 2: Run it and verify it fails**

Run: `pnpm test -- viewState`
Expected: FAIL — cannot resolve `./viewState`.

- [ ] **Step 3: Write `src/lib/viewState.ts`**

```ts
import type { Group } from "./api/types";
import { parseViewKey, viewKey, type View } from "./views";

// The selected view and the view mode persist; sort does not, which is how it
// already behaves. They live in localStorage rather than projects.db because
// the database is a cross-app contract devmon attaches read-only, and UI
// preferences are none of its business.
//
// Persisting the selection is only safe because the sidebar is legible: the
// current selection and every other view are always on screen, so restoring
// one cannot surprise anyone. That is precisely why collapse state could not
// be persisted in the rejected sectioned design.
export type ViewMode = "list" | "grid" | "compact";

const MODES: readonly ViewMode[] = ["list", "grid", "compact"];

const MODE_KEY = "pi.viewMode";
const VIEW_KEY = "pi.view";

// Pure. `raw` is whatever was in storage — possibly written by an older build,
// possibly hand-edited, possibly absent.
export function restoreViewMode(raw: string | null): ViewMode {
  return MODES.includes(raw as ViewMode) ? (raw as ViewMode) : "list";
}

// Pure. Two rules the spec names explicitly:
//  - a view whose group has since been deleted falls back to All, rather than
//    rendering an empty list with nothing saying why;
//  - Bin is never restored as the landing view. It is a destination you go to,
//    not a place to start.
export function restoreView(raw: string | null, groups: Group[]): View {
  const view = parseViewKey(raw);
  if (!view) return { kind: "all" };
  if (view.kind === "bin") return { kind: "all" };
  if (view.kind === "group" && !groups.some((g) => g.id === view.id)) return { kind: "all" };
  return view;
}

// Storage access. Guarded because SvelteKit prerenders this app to static
// HTML at build time, where there is no window — and because a browser with
// site data blocked throws on the accessor itself.
function read(key: string): string | null {
  try {
    return typeof localStorage === "undefined" ? null : localStorage.getItem(key);
  } catch {
    return null;
  }
}

function write(key: string, value: string): void {
  try {
    if (typeof localStorage !== "undefined") localStorage.setItem(key, value);
  } catch {
    // A preference that cannot be saved is not worth an error banner.
  }
}

export function loadViewMode(): ViewMode {
  return restoreViewMode(read(MODE_KEY));
}

export function saveViewMode(mode: ViewMode): void {
  write(MODE_KEY, mode);
}

export function loadView(groups: Group[]): View {
  return restoreView(read(VIEW_KEY), groups);
}

export function saveView(view: View): void {
  write(VIEW_KEY, viewKey(view));
}
```

- [ ] **Step 4: Run the test**

Run: `pnpm test -- viewState`
Expected: PASS (13 tests).

- [ ] **Step 5: Create `src/lib/components/ViewControls.svelte`**

```svelte
<script lang="ts">
  import type { ViewMode } from "$lib/viewState";
  import BundledIcon from "./BundledIcon.svelte";
  import { inputClass } from "./styles";

  let {
    mode = $bindable<ViewMode>("list"),
    query = $bindable(""),
  }: {
    mode?: ViewMode;
    query?: string;
  } = $props();

  const modes: { value: ViewMode; icon: string; label: string }[] = [
    { value: "list", icon: "layers", label: "List" },
    { value: "grid", icon: "box", label: "Grid" },
    { value: "compact", icon: "flag", label: "Compact" },
  ];
</script>

<div class="flex items-center gap-2">
  <input
    bind:value={query}
    type="search"
    placeholder="Search name, path or tag"
    aria-label="Search projects"
    class={`h-8 min-w-0 flex-1 ${inputClass}`}
  />
  <div class="flex shrink-0 items-center gap-0.5 rounded-sm border border-line bg-panel-2 p-0.5">
    {#each modes as m}
      <button
        type="button"
        onclick={() => (mode = m.value)}
        aria-pressed={mode === m.value}
        title={m.label}
        aria-label={`${m.label} view`}
        class={`inline-flex h-7 w-7 items-center justify-center rounded-sm ${
          mode === m.value ? "bg-panel text-accent" : "text-phos-dim hover:text-phos"
        }`}
      >
        <BundledIcon name={m.icon} class="h-4 w-4" />
      </button>
    {/each}
  </div>
</div>
```

- [ ] **Step 6: Teach `ProjectList` the three modes**

In `src/lib/components/ProjectList.svelte`, add `mode` to the props and imports:

```ts
  import ProjectCompactRow from "./ProjectCompactRow.svelte";
  import ProjectTile from "./ProjectTile.svelte";
  import type { ViewMode } from "$lib/viewState";
```

Add `mode: ViewMode;` to the prop type and `mode,` to the destructure. Then replace the `<ul>` block with one that switches container and presentation. The `actions` snippet is defined once and passed to whichever presentation renders:

```svelte
  {#snippet standardActions(project: Project)}
    <ProjectActionsMenu
      {project}
      {onEdit}
      {onRequestDelete}
      {onOpened}
      {onTrackersRefreshed}
      {onOpenWithAppMissing}
      {onerror}
    />
  {/snippet}

  <ul
    class={mode === "grid"
      ? "grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-3"
      : mode === "compact"
        ? "flex flex-col"
        : "flex flex-col gap-3"}
  >
    {#each projects as project (project.id)}
      {@const groupColor = groupColorOf(project)}
      {@const missing = missingDirs.has(project.id)}
      <li
        class={mode === "compact"
          ? "border-b border-line px-1 py-1.5 last:border-b-0"
          : "rounded-sm border border-line p-3"}
        style={mode === "list" && groupColor
          ? `border-left: 2px solid ${swatchVar(groupColor)}`
          : undefined}
      >
        {#if editingId === project.id}
          <EditProjectForm {project} {onSaved} onCancel={onCancelEdit} {onerror} />
        {:else if mode === "grid"}
          <ProjectTile {project} directoryMissing={missing} {customIcons} {groupColor}>
            {#snippet actions()}{@render standardActions(project)}{/snippet}
          </ProjectTile>
        {:else if mode === "compact"}
          <ProjectCompactRow {project} directoryMissing={missing} {customIcons} {groupColor}>
            {#snippet actions()}{@render standardActions(project)}{/snippet}
          </ProjectCompactRow>
        {:else}
          <ProjectRow {project} directoryMissing={missing} {customIcons} {groupColor}>
            {#snippet actions()}{@render standardActions(project)}{/snippet}
          </ProjectRow>
        {/if}
      </li>
    {/each}
  </ul>
```

- [ ] **Step 7: Wire it into `+page.svelte`**

Replace the `SortControls` row so search, mode and sort share one line, and persist both pieces of state:

```ts
  import ViewControls from "$lib/components/ViewControls.svelte";
  import { loadView, loadViewMode, saveView, saveViewMode, type ViewMode } from "$lib/viewState";

  // Initialised directly rather than in an $effect. `src/routes/+layout.ts`
  // sets `ssr = false`, so component init only ever runs in the browser and
  // localStorage is there — and initialising here means the first render is
  // already the restored mode, with no save-effect racing the load.
  let viewMode = $state<ViewMode>(loadViewMode());

  // The selected view cannot be restored the same way: "group:<id>" falls
  // back to All when that group no longer exists, and answering that needs
  // the group list, which has not been fetched at init.
  let groupsLoaded = $state(false);
  let viewRestored = $state(false);
```

In `loadGroups`, mark the attempt finished either way — a failed group fetch cannot validate a stored group id, and falling back to All is the right answer in that case too:

```ts
  async function loadGroups() {
    try {
      groups = await listGroups();
    } catch (err) {
      error = (err as Error).message;
    } finally {
      groupsLoaded = true;
    }
  }
```

Then restore the view exactly once, on the first completed group load. The `viewRestored` guard is what stops a later refetch — after a group is created or deleted — from yanking the user back to the stored view they have since navigated away from:

```ts
  $effect(() => {
    if (groupsLoaded && !viewRestored) {
      selectedView = loadView(groups);
      viewRestored = true;
    }
  });

  $effect(() => {
    saveViewMode(viewMode);
  });
```

and in `handleSelectView`, persist the choice:

```ts
  function handleSelectView(view: View) {
    selectedView = view;
    saveView(view);
    error = "";
  }
```

Markup:

```svelte
      <div class="mb-3 flex items-center gap-2">
        <div class="min-w-0 flex-1">
          <ViewControls bind:mode={viewMode} bind:query />
        </div>
        <SortControls bind:by={sortBy} bind:direction={sortDirection} />
      </div>

      <ProjectList
        projects={visibleProjects}
        mode={viewMode}
        …
```

- [ ] **Step 8: Verify**

Run: `pnpm test && pnpm run check`
Expected: all green; 0 errors, 8 warnings.

Run: `pnpm tauri dev` and check:
- All three modes render and the toggle shows which is active.
- Typing in search filters live, and clearing it restores the full list.
- Search applies *within* the selected view (select Ungrouped, search for a grouped project's name — no result).
- Sort still works in every mode.
- Restart the app: the view mode comes back, and the selected view comes back.

- [ ] **Step 9: Commit**

```bash
git add src/lib/components/ViewControls.svelte src/lib/components/ProjectList.svelte src/lib/viewState.ts src/lib/viewState.test.ts src/routes/+page.svelte
git commit -m "$(cat <<'MSG'
feat(ui): add the view-mode toggle, search, and persisted view state

Three modes — list, grid, compact — over one ProjectList, each presentation
receiving the same actions snippet. Search matches name, path and tags, and
applies within the selected view rather than across everything.

viewState.ts splits the pure restore rules from localStorage access so the
two rules the spec names are testable in vitest's node environment, where
there is no localStorage: a view whose group has since been deleted falls
back to All rather than rendering an unexplained empty list, and Bin is
never restored as the landing view — it is a destination, not a home.

Preferences live in localStorage, not projects.db: the database is a
cross-app contract devmon attaches read-only, and UI preferences are none of
its business. Sort stays unpersisted, as it already was.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 7: Fold Favourites into the sidebar

Retires a working feature. Verify against the modal before deleting it.

**Files:**
- Modify: `src/routes/+page.svelte`
- Modify: `src/lib/components/Sidebar.svelte` (nothing — `showFavorites` already exists; just pass `true`)
- Delete: `src/lib/components/FavoritesModal.svelte`

**Interfaces:**
- Consumes: everything from Tasks 3, 5, 6.
- Produces: no new exports. `Sidebar` is called with `showFavorites={true}`.

- [ ] **Step 1: Record what the modal does, so the replacement can be checked against it**

Run `pnpm tauri dev`, open Favourites from the header, and note: the list is favourited projects only; each row offers **Open** and a **★** that un-favourites and removes the row; sort controls; "No favorites yet." when empty; Escape and the Close button dismiss it.

The standard `···` menu covers Open. Un-favouriting is *not* in the standard menu today — it was only reachable from the modal's ★ and from the Edit form's Favorite checkbox. Keep it reachable: the Favourites view rows keep their star, and clicking it un-favourites.

- [ ] **Step 2: Make the row star actionable**

In `src/lib/components/ProjectRow.svelte`, `ProjectTile.svelte` and `ProjectCompactRow.svelte`, replace the static star with a button, adding an `onToggleFavorite` optional prop to each:

```ts
    onToggleFavorite,
  }: {
    …
    // Optional: when absent the star is a static marker, as it is in every
    // other view. The Favourites view supplies it so un-favouriting stays
    // reachable — it was only ever offered by FavoritesModal's star and the
    // edit form's checkbox, and this view replaces the first of those.
    onToggleFavorite?: (project: Project) => void;
  } = $props();
```

and the markup, in all three:

```svelte
      {#if project.favorite}
        {#if onToggleFavorite}
          <button
            type="button"
            onclick={() => onToggleFavorite(project)}
            class="shrink-0 text-gold hover:text-phos"
            title="Remove from favourites"
            aria-label="Remove from favourites"
          >
            ★
          </button>
        {:else}
          <span class="shrink-0 text-gold" title="Favorite">★</span>
        {/if}
      {/if}
```

- [ ] **Step 3: Thread it through `ProjectList`**

Add `onToggleFavorite?: (project: Project) => void;` to the props and pass it to all three presentations.

- [ ] **Step 4: Wire it in `+page.svelte`**

```ts
  import { updateProject } from "$lib/api/projects";

  // Only offered in the Favourites view, mirroring the star FavoritesModal
  // had. A failure refetches so the list matches what the backend actually
  // holds rather than an optimistic guess.
  async function handleToggleFavorite(project: Project) {
    try {
      await updateProject(project.id, { favorite: !project.favorite });
      await loadProjects();
    } catch (err) {
      error = (err as Error).message;
      await loadProjects();
    }
  }
```

Pass to `ProjectList`:

```svelte
        onToggleFavorite={selectedView.kind === "favorites" ? handleToggleFavorite : undefined}
```

- [ ] **Step 5: Turn the sidebar entry on and remove the header button**

In the `<Sidebar …>` call add `showFavorites={true}`.

Delete from the header markup the entire Favourites `<button …aria-label="Open favorites">…</button>` block, and from the script: `favoritesOpen`, `handleOpenFavorites`, `handleCloseFavorites`, `handleFavoritesChanged`, the `FavoritesModal` import, and the `{#if favoritesOpen}…{/if}` block.

- [ ] **Step 6: Verify against the modal, then delete it**

Run: `pnpm tauri dev` and confirm, before deleting anything:
- The sidebar Favourites entry shows the same count as the modal's list length.
- Selecting it lists exactly the favourited projects.
- **Open** works from the `···` menu (including the "app is missing" path — the `OpenWithMissingModal` still opens).
- Clicking a row's ★ un-favourites it and the row leaves the view.
- With no favourites the list shows the empty state.
- Sort and search work in this view like any other.

Then:

```bash
git rm src/lib/components/FavoritesModal.svelte
grep -rn "FavoritesModal\|favoritesOpen" src/
```
Expected: no output from the grep.

- [ ] **Step 7: Run the gates and commit**

Run: `pnpm test && pnpm run check` → all green; 0 errors, 8 warnings.

```bash
git add -A src/
git commit -m "$(cat <<'MSG'
feat(ui): fold Favourites into the sidebar and delete FavoritesModal

Favourites becomes a sidebar view with the standard actions menu, verified
against the modal before it was removed: same membership, same count, Open
including the missing-app path, and the same empty state.

Un-favouriting stays reachable. It was only ever offered by the modal's star
and the edit form's checkbox, so the row star becomes a button in the
Favourites view — passed as an optional callback, which leaves it a static
marker in every other view exactly as it is today.

The modal's own SortControls goes with it: every view shares one list
surface now, so that duplication has nowhere left to live.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 8: Fold the Bin into the sidebar, with `BinActions`

Retires a working feature. **The two-click purge confirm is a preserved invariant, not a detail.**

**Files:**
- Create: `src/lib/components/BinActions.svelte`
- Modify: `src/routes/+page.svelte`, `src/lib/components/ProjectList.svelte`
- Delete: `src/lib/components/BinModal.svelte`

**Interfaces:**
- Consumes: `restoreProject`, `deleteProject` from `api/projects.ts`; the snippet-prop shape from Task 4.
- Produces: `<BinActions {project} {onRestored} {onPurged} {onerror} />`; `ProjectList` gains `binMode: boolean`.

**Why a separate action component rather than a flag on the menu.** Bin rows must not offer Open, Edit or Detect type — the directory is gone. Taking the action set as a snippet is what lets the same three row layouts serve both sets without a second copy of the row markup.

- [ ] **Step 1: Record what the modal does**

Run `pnpm tauri dev`, open the Bin, and note: **Restore** puts the project back in the main list; **Delete permanently** turns into **Confirm?** on the first click and only purges on the second; both buttons disable while a call is in flight; "The bin is empty." when empty.

- [ ] **Step 2: Create `src/lib/components/BinActions.svelte`**

```svelte
<script lang="ts">
  import { deleteProject, restoreProject } from "$lib/api/projects";
  import type { Project } from "$lib/api/types";
  import { buttonClass, dangerButtonClass } from "./styles";

  let {
    project,
    onRestored,
    onPurged,
    onerror,
  }: {
    project: Project;
    onRestored: () => void | Promise<void>;
    onPurged: () => void | Promise<void>;
    onerror?: (message: string) => void;
  } = $props();

  let pending = $state(false);
  // Purging is permanent, so the danger button asks for a second click before
  // it acts rather than stacking a confirmation dialog on top of the view.
  // This is a preserved invariant, carried over verbatim from BinModal.
  let confirming = $state(false);

  async function handleRestore() {
    pending = true;
    try {
      await restoreProject(project.id);
      await onRestored();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      pending = false;
    }
  }

  async function handlePurge() {
    if (!confirming) {
      confirming = true;
      return;
    }
    pending = true;
    try {
      await deleteProject(project.id);
      await onPurged();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      pending = false;
      confirming = false;
    }
  }
</script>

<div class="flex shrink-0 gap-2">
  <button type="button" onclick={handleRestore} disabled={pending} class={buttonClass}>
    Restore
  </button>
  <button type="button" onclick={handlePurge} disabled={pending} class={dangerButtonClass}>
    {confirming ? "Confirm?" : "Delete permanently"}
  </button>
</div>
```

- [ ] **Step 3: Teach `ProjectList` the Bin action set**

Add the props:

```ts
    binMode = false,
    onBinChanged,
  }: {
    …
    // Bin rows never offer Open, Edit or Detect type — the directory is gone.
    binMode?: boolean;
    onBinChanged?: () => void | Promise<void>;
  } = $props();
```

Add the second snippet next to `standardActions`:

```svelte
  {#snippet binActions(project: Project)}
    <BinActions
      {project}
      onRestored={() => onBinChanged?.()}
      onPurged={() => onBinChanged?.()}
      {onerror}
    />
  {/snippet}
```

and use it in each presentation's `actions`:

```svelte
          {#snippet actions()}{@render (binMode ? binActions : standardActions)(project)}{/snippet}
```

Also guard editing — a Bin row must never swap to `EditProjectForm`:

```svelte
        {#if editingId === project.id && !binMode}
```

and the empty state:

```svelte
  {:else if projects.length === 0}
    <p class="text-sm text-phos-dim">{binMode ? "The bin is empty." : "No projects yet."}</p>
```

- [ ] **Step 4: Fetch the deleted projects in `+page.svelte`**

```ts
  import { getDeletedProjects } from "$lib/api/projects";
```

Extend `loadProjects` so both lists refresh together and the Bin count is always current:

```ts
    try {
      projects = await getAllProjects({ by: sortBy, direction: sortDirection });
      deletedProjects = await getDeletedProjects({ by: sortBy, direction: sortDirection });
    } catch (err) {
      error = (err as Error).message;
    } finally {
      loading = false;
    }
```

and add:

```ts
  async function handleBinChanged() {
    error = "";
    await loadProjects();
  }
```

- [ ] **Step 5: Turn the sidebar entry on and remove the header button**

In the `<Sidebar …>` call add `showBin={true}`.

Pass to `ProjectList`:

```svelte
        binMode={selectedView.kind === "bin"}
        onBinChanged={handleBinChanged}
```

Delete from the header markup the Bin `<button …aria-label="Open bin">…</button>` block, and from the script: `binOpen`, `handleOpenBin`, `handleCloseBin`, `handleRestored`, the `BinModal` import, and the `{#if binOpen}…{/if}` block.

- [ ] **Step 6: Verify against the modal, then delete it**

Run: `pnpm tauri dev`. With at least one project in the bin (delete a project's directory, keeping metadata), confirm **before deleting anything**:
- The Bin count in the sidebar matches the number of rows in the view.
- **Restore** returns the project to All, and both counts update.
- **Delete permanently** shows **Confirm?** on the first click and only purges on the second. *This is the invariant — check it explicitly.*
- Clicking a different row's purge button while another is mid-confirm does not purge anything.
- Bin rows offer **only** Restore and Delete permanently — no `···` menu, no Open, no Edit, no Detect type.
- Clicking a Bin row does not open the edit form.
- With the bin empty, "The bin is empty." shows.
- All three view modes render Bin rows correctly.

Then:

```bash
git rm src/lib/components/BinModal.svelte
grep -rn "BinModal\|binOpen" src/
```
Expected: no output from the grep.

- [ ] **Step 7: Run the gates and commit**

Run: `pnpm test && pnpm run check` → all green; 0 errors, 8 warnings.

```bash
git add -A src/
git commit -m "$(cat <<'MSG'
feat(ui): fold the Bin into the sidebar with BinActions, delete BinModal

The Bin becomes a sidebar view over the same three row layouts, with its own
action set supplied as a snippet — which is what lets one set of layouts
serve both Bin and every other view without a second copy of the markup.

The two-click purge confirm is carried over verbatim and was checked against
the modal before it was deleted: "Delete permanently" becomes "Confirm?" on
the first click and only purges on the second. It is a preserved invariant,
not a detail — purging is permanent and it deliberately avoids stacking a
confirmation dialog on top of the view.

Bin rows offer no Open, Edit or Detect type, and cannot open the edit form:
the directory is gone.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 9: `GroupManagerModal`

**Files:**
- Create: `src/lib/components/SwatchPicker.svelte`, `src/lib/components/GroupManagerModal.svelte`
- Modify: `src/routes/+page.svelte`

**Interfaces:**
- Consumes: `api/groups.ts` (Task 3); `SWATCHES`, `swatchVar` (Task 2); `ICON_NAMES` (Task 2).
- Produces:
  - `<SwatchPicker bind:value {allowNone} />` — `value: string | null`
  - `<GroupManagerModal {groups} {onChanged} {onClose} {onerror} />`

**Note.** The icon picker with custom-icon import arrives in Task 10; `GroupManagerModal` uses a plain bundled-icon `<select>` here, and Task 10 swaps it for `IconPicker`. Group icons come from the bundled set only — a custom SVG cannot be tinted and a sidebar entry must take its group's colour.

- [ ] **Step 1: Create `src/lib/components/SwatchPicker.svelte`**

```svelte
<script lang="ts">
  import { SWATCHES, swatchVar } from "$lib/palette";
  import { labelClass } from "./styles";

  // Stores a palette *name*, never a literal. swatchVar can only ever return
  // one of nine fixed var(--color-*) strings, so nothing arbitrary reaches
  // the style attribute.
  let {
    value = $bindable<string | null>(null),
    label = "Colour",
    // Projects may have no colour; a group must have one.
    allowNone = false,
  }: {
    value?: string | null;
    label?: string;
    allowNone?: boolean;
  } = $props();
</script>

<div class={labelClass}>
  {label}
  <div class="flex flex-wrap items-center gap-1.5">
    {#if allowNone}
      <button
        type="button"
        onclick={() => (value = null)}
        title="No colour"
        aria-label="No colour"
        aria-pressed={value === null}
        class={`h-6 w-6 rounded-full border ${value === null ? "border-phos" : "border-line"} text-[11px] text-phos-faint`}
      >
        —
      </button>
    {/if}
    {#each SWATCHES as swatch}
      <button
        type="button"
        onclick={() => (value = swatch)}
        title={swatch}
        aria-label={swatch}
        aria-pressed={value === swatch}
        class={`h-6 w-6 rounded-full border-2 ${value === swatch ? "border-phos" : "border-transparent"}`}
        style={`background: ${swatchVar(swatch)}`}
      ></button>
    {/each}
  </div>
</div>
```

- [ ] **Step 2: Create `src/lib/components/GroupManagerModal.svelte`**

```svelte
<script lang="ts">
  import { createGroup, deleteGroup, reorderGroups, updateGroup } from "$lib/api/groups";
  import type { Group } from "$lib/api/types";
  import { ICON_NAMES } from "$lib/icons";
  import BundledIcon from "./BundledIcon.svelte";
  import SwatchPicker from "./SwatchPicker.svelte";
  import { buttonClass, dangerButtonClass, inputClass, labelClass, primaryButtonClass } from "./styles";

  let {
    groups,
    onChanged,
    onClose,
    onerror,
  }: {
    groups: Group[];
    onChanged: () => void | Promise<void>;
    onClose: () => void;
    onerror?: (message: string) => void;
  } = $props();

  let newName = $state("");
  let newColor = $state<string | null>("cyan");
  let newIcon = $state("briefcase");
  let busy = $state(false);
  // Deleting a group never deletes a project — its members become Ungrouped —
  // but it is still irreversible, so it takes a second click rather than a
  // dialog on top of a dialog, matching the Bin's purge.
  let confirmDeleteId = $state<string | null>(null);

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") onClose();
  }

  async function run(action: () => Promise<unknown>) {
    busy = true;
    try {
      await action();
      await onChanged();
    } catch (err) {
      onerror?.((err as Error).message);
    } finally {
      busy = false;
    }
  }

  async function handleCreate(event: Event) {
    event.preventDefault();
    const name = newName.trim();
    if (name.length === 0) return;
    await run(async () => {
      // A group must have a colour; the picker cannot produce null here
      // because allowNone is not set, but the type permits it.
      await createGroup(name, newColor ?? "cyan", newIcon);
      newName = "";
    });
  }

  // Renaming, recolouring and re-iconing all go through the same partial
  // update: an omitted key means unchanged.
  async function handleRename(group: Group, name: string) {
    const trimmed = name.trim();
    if (trimmed.length === 0 || trimmed === group.name) return;
    await run(() => updateGroup(group.id, { name: trimmed }));
  }

  async function handleRecolour(group: Group, color: string | null) {
    if (!color || color === group.color) return;
    await run(() => updateGroup(group.id, { color }));
  }

  async function handleReicon(group: Group, icon: string) {
    if (icon === group.icon) return;
    await run(() => updateGroup(group.id, { icon }));
  }

  // Reorder rewrites the whole ordering and returns what was persisted, so the
  // caller refetches rather than trusting its own optimistic order.
  async function handleMove(index: number, delta: number) {
    const next = index + delta;
    if (next < 0 || next >= groups.length) return;
    const ids = groups.map((g) => g.id);
    [ids[index], ids[next]] = [ids[next], ids[index]];
    await run(() => reorderGroups(ids));
  }

  async function handleDelete(group: Group) {
    if (confirmDeleteId !== group.id) {
      confirmDeleteId = group.id;
      return;
    }
    await run(async () => {
      await deleteGroup(group.id);
      confirmDeleteId = null;
    });
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class="fixed inset-0 z-[100] flex items-center justify-center bg-void/85"
  role="presentation"
  onclick={onClose}
  onkeydown={handleKeydown}
>
  <div
    class="max-h-[85vh] w-11/12 max-w-xl overflow-y-auto rounded-sm border border-line bg-panel p-6"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="group-manager-title"
    onclick={(e) => e.stopPropagation()}
    onkeydown={(e) => e.stopPropagation()}
  >
    <div class="flex items-center justify-between">
      <h2 id="group-manager-title" class="mt-0 text-lg font-semibold text-phos">Groups</h2>
      <button type="button" onclick={onClose} class={buttonClass}>Close</button>
    </div>
    <p class="mt-2 text-sm text-phos-dim">
      A project belongs to one group at a time. Deleting a group never deletes a project — its
      members become Ungrouped.
    </p>

    <form onsubmit={handleCreate} class="mt-4 flex flex-col gap-3 rounded-sm bg-panel-2 p-3">
      <label class={labelClass}>
        New group
        <input bind:value={newName} placeholder="Client work" required class={inputClass} />
      </label>
      <SwatchPicker bind:value={newColor} />
      <label class={labelClass}>
        Icon
        <select bind:value={newIcon} class={inputClass}>
          {#each ICON_NAMES as name}
            <option value={name}>{name}</option>
          {/each}
        </select>
      </label>
      <button type="submit" disabled={busy} class={`self-start ${primaryButtonClass}`}>
        {busy ? "Working…" : "Add group"}
      </button>
    </form>

    {#if groups.length === 0}
      <p class="mt-4 text-sm text-phos-dim">No groups yet.</p>
    {:else}
      <ul class="mt-4 flex flex-col gap-2">
        {#each groups as group, index (group.id)}
          <li class="rounded-sm border border-line p-3">
            <div class="flex flex-wrap items-center gap-2">
              <span class="shrink-0" style={`color: ${swatchVar(group.color)}`}>
                <BundledIcon name={group.icon} class="h-4 w-4" />
              </span>
              <input
                value={group.name}
                disabled={busy}
                onchange={(e) => handleRename(group, e.currentTarget.value)}
                class={`min-w-0 flex-1 ${inputClass}`}
                aria-label={`Rename ${group.name}`}
              />
              <select
                value={group.icon}
                disabled={busy}
                onchange={(e) => handleReicon(group, e.currentTarget.value)}
                class={inputClass}
                aria-label={`Icon for ${group.name}`}
              >
                {#each ICON_NAMES as name}
                  <option value={name}>{name}</option>
                {/each}
              </select>
              <button
                type="button"
                disabled={busy || index === 0}
                onclick={() => handleMove(index, -1)}
                class={buttonClass}
                aria-label={`Move ${group.name} up`}
              >
                ↑
              </button>
              <button
                type="button"
                disabled={busy || index === groups.length - 1}
                onclick={() => handleMove(index, 1)}
                class={buttonClass}
                aria-label={`Move ${group.name} down`}
              >
                ↓
              </button>
              <button
                type="button"
                disabled={busy}
                onclick={() => handleDelete(group)}
                class={dangerButtonClass}
              >
                {confirmDeleteId === group.id ? "Confirm?" : "Delete"}
              </button>
            </div>
            <!-- SwatchPicker binds a value; recolouring an existing group
                 needs a call instead, so this row commits directly. -->
            <div class="mt-2 flex flex-wrap items-center gap-1.5">
              {#each SWATCHES as swatch}
                <button
                  type="button"
                  disabled={busy}
                  onclick={() => handleRecolour(group, swatch)}
                  title={swatch}
                  aria-label={`${swatch} for ${group.name}`}
                  aria-pressed={group.color === swatch}
                  class={`h-5 w-5 rounded-full border-2 ${group.color === swatch ? "border-phos" : "border-transparent"}`}
                  style={`background: ${swatchVar(swatch)}`}
                ></button>
              {/each}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
```

Two details in the above that are easy to get wrong:

- The script needs `import { SWATCHES, swatchVar } from "$lib/palette";` **as well as** the `SwatchPicker` import — the per-group row commits rather than binds, so it needs the raw list and the resolver.
- Both colour sites go through `swatchVar(...)`, never an interpolated token name like `var(--color-swatch-${group.color})`. A group whose stored colour this build does not recognise must fall back to neutral, not produce an undefined custom property that renders as no colour at all.

- [ ] **Step 3: Wire it into `+page.svelte`**

```ts
  import GroupManagerModal from "$lib/components/GroupManagerModal.svelte";

  let groupManagerOpen = $state(false);

  async function handleGroupsChanged() {
    error = "";
    await loadGroups();
    // A project's group_id may have been cleared by a group deletion, so the
    // project list is stale too.
    await loadProjects();
  }
```

On the `<Sidebar …>` call add `onManageGroups={() => (groupManagerOpen = true)}`, and at the bottom of the markup:

```svelte
{#if groupManagerOpen}
  <GroupManagerModal
    {groups}
    onChanged={handleGroupsChanged}
    onClose={() => (groupManagerOpen = false)}
    onerror={handleError}
  />
{/if}
```

- [ ] **Step 4: Verify**

Run: `pnpm test && pnpm run check` → all green; 0 errors, 8 warnings.

Run: `pnpm tauri dev` and confirm:
- The `+` in the sidebar's groups header opens the manager.
- Creating a group appends it **last** in the sidebar, not first.
- A duplicate name (differing only in case) is rejected with the backend's message in the error banner, and no group is created.
- An empty name is rejected.
- Renaming, recolouring and re-iconing each update the sidebar entry immediately.
- ↑ / ↓ reorder, and the order survives a restart (it is persisted, not local).
- Delete asks **Confirm?** first. After deleting a group, its projects appear under **Ungrouped** and **none of them is gone**. If that group was the selected view, the selection falls back to All.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/GroupManagerModal.svelte src/lib/components/SwatchPicker.svelte src/routes/+page.svelte
git commit -m "$(cat <<'MSG'
feat(groups): add the group manager — create, rename, recolour, re-icon, reorder, delete

Reached from the sidebar's groups header. Reorder sends the whole ordering
through reorder_groups and re-reads what was persisted rather than trusting
its own optimistic order, since set_group_positions is the only thing that
renumbers.

Delete takes a second click, matching the Bin's purge. It never deletes a
project: members become Ungrouped, and if the deleted group was the selected
view the sidebar falls back to All.

Group icons come from the bundled set only — a custom SVG cannot be tinted,
and a sidebar entry has to take its group's colour.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 10: Colour, icon and group pickers in the create/edit forms

The last feature task, and the one that first lets a user import a custom icon — which is why Task 1 had to land first.

**Files:**
- Create: `src/lib/components/IconPicker.svelte`
- Modify: `src/lib/components/CreateProjectForm.svelte`, `src/lib/components/EditProjectForm.svelte`, `src/lib/components/GroupManagerModal.svelte`, `src/routes/+page.svelte`

**Interfaces:**
- Consumes: `api/icons.ts`, `isGroupNotFound` (Task 3); `SwatchPicker` (Task 9); `ICON_NAMES`, `customIconSrc` (Task 2).
- Produces: `<IconPicker bind:value {customIcons} {onIconsChanged} {bundledOnly} {onerror} />` — `value: string | null`.

**Note on create.** `create_project` takes only name/directory/description/tags. So the create form collects colour, icon and group and applies them with an `updateProject` immediately after the create — one extra call, on a local database, and it keeps `CreateProjectInput` unchanged.

- [ ] **Step 1: Create `src/lib/components/IconPicker.svelte`**

```svelte
<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { deleteCustomIcon, importCustomIcon, listCustomIcons } from "$lib/api/icons";
  import { ICON_NAMES } from "$lib/icons";
  import BundledIcon from "./BundledIcon.svelte";
  import { buttonClass, labelClass } from "./styles";

  let {
    value = $bindable<string | null>(null),
    customIcons,
    onIconsChanged,
    // Group icons come from the bundled set only: a custom SVG renders as an
    // <img> and cannot be tinted, and a sidebar entry must take its group's
    // colour.
    bundledOnly = false,
    onerror,
  }: {
    value?: string | null;
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    bundledOnly?: boolean;
    onerror?: (message: string) => void;
  } = $props();

  let importing = $state(false);

  const cell =
    "inline-flex h-8 w-8 items-center justify-center rounded-sm border-2 text-phos-dim hover:text-phos";

  async function handleImport() {
    importing = true;
    try {
      const path = await open({
        multiple: false,
        filters: [{ name: "SVG", extensions: ["svg"] }],
      });
      if (typeof path !== "string") return;
      const icon = await importCustomIcon(path);
      await onIconsChanged?.();
      value = `custom:${icon.name}`;
    } catch (err) {
      // Import fails for content reasons as often as IO ones — an undefined
      // entity, a bare &, &nbsp; (an HTML entity XML does not define, common
      // in HTML-flavoured exports), nothing drawable left after sanitizing,
      // over 256 KB, a backslash in an attribute value. The message names the
      // reason and every one of them is something the user can fix, so
      // surface it rather than a generic failure.
      onerror?.((err as Error).message);
    } finally {
      importing = false;
    }
  }

  async function handleDeleteCustom(name: string) {
    try {
      await deleteCustomIcon(name);
      if (value === `custom:${name}`) value = null;
      await onIconsChanged?.();
    } catch (err) {
      onerror?.((err as Error).message);
    }
  }
</script>

<div class={labelClass}>
  Icon
  <div class="flex flex-wrap items-center gap-1">
    <button
      type="button"
      onclick={() => (value = null)}
      title="No icon"
      aria-label="No icon"
      aria-pressed={value === null}
      class={`${cell} ${value === null ? "border-phos" : "border-line"}`}
    >
      —
    </button>
    {#each ICON_NAMES as name}
      <button
        type="button"
        onclick={() => (value = name)}
        title={name}
        aria-label={name}
        aria-pressed={value === name}
        class={`${cell} ${value === name ? "border-phos text-phos" : "border-transparent"}`}
      >
        <BundledIcon {name} class="h-4 w-4" />
      </button>
    {/each}
  </div>

  {#if !bundledOnly}
    <div class="mt-2 flex flex-wrap items-center gap-1">
      {#each [...customIcons] as [name, src] (name)}
        <span class="relative inline-flex">
          <button
            type="button"
            onclick={() => (value = `custom:${name}`)}
            title={name}
            aria-label={name}
            aria-pressed={value === `custom:${name}`}
            class={`${cell} ${value === `custom:${name}` ? "border-phos" : "border-transparent"}`}
          >
            <!-- A custom icon renders only through <img> with a data: URI —
                 inert regardless of what the sanitizer missed. Never {@html}. -->
            <img {src} alt="" class="h-4 w-4" />
          </button>
          <button
            type="button"
            onclick={() => handleDeleteCustom(name)}
            title={`Delete ${name}`}
            aria-label={`Delete icon ${name}`}
            class="absolute -right-1 -top-1 rounded-full bg-panel px-1 text-[10px] leading-none text-phos-faint hover:text-rust"
          >
            ×
          </button>
        </span>
      {/each}
      <button type="button" onclick={handleImport} disabled={importing} class={buttonClass}>
        {importing ? "Importing…" : "Import SVG…"}
      </button>
    </div>
  {/if}
</div>
```

- [ ] **Step 2: Add the three fields to `EditProjectForm.svelte`**

Add props and state — **without adding a 9th `state_referenced_locally` warning**. The existing 8 come from plain `$state(project.x)` initialisers; three more of that shape would make 11, and the standing instruction is that the baseline stays at 8.

So initialise these three through `untrack`, which says explicitly what the plain form only implies — capture the value once, deliberately — and produces no warning:

```ts
  import { untrack } from "svelte";

  let color = $state<string | null>(untrack(() => project.color));
  let icon = $state<string | null>(untrack(() => project.icon));
  let groupId = $state<string | null>(untrack(() => project.group_id));
```

Do **not** convert the existing 8 to this form. They are `PI-003`, a documented false positive, and rewriting them is outside this task's scope. `pnpm run check` must still report **0 errors, 8 warnings** after this step — verify that before moving on.

```ts
  import { isGroupNotFound } from "$lib/api/groups";
  import type { Group } from "$lib/api/types";
  import IconPicker from "./IconPicker.svelte";
  import SwatchPicker from "./SwatchPicker.svelte";

  let {
    project,
    groups,
    customIcons,
    onIconsChanged,
    onGroupsStale,
    onSaved,
    onCancel,
    onerror,
  }: {
    project: Project;
    groups: Group[];
    customIcons: Map<string, string>;
    onIconsChanged?: () => void | Promise<void>;
    // Called when the backend rejects the write because the selected group no
    // longer exists. The caller refetches the group list.
    onGroupsStale?: () => void | Promise<void>;
    onSaved: () => void | Promise<void>;
    onCancel: () => void;
    onerror?: (message: string) => void;
  } = $props();
```

In `handleSubmit`, add the three fields and handle `GroupNotFound`:

```ts
      await updateProject(project.id, {
        name,
        directory,
        description,
        tags: parseTags(tags),
        favorite,
        notes: notes || null,
        client: client || null,
        open_with: openWith || null,
        group_id: groupId,
        color,
        icon,
      });
      await onSaved();
    } catch (err) {
      // The group can be deleted from the group manager while this form is
      // open. The backend rejects the write and changes nothing, so refetch
      // the group list and clear the stale selection — retrying is guaranteed
      // to fail the same way. Clearing a group (group_id: null) is always
      // allowed and checks nothing.
      if (isGroupNotFound(err)) {
        groupId = null;
        await onGroupsStale?.();
        onerror?.("That group no longer exists — the selection was cleared. Save again.");
      } else {
        onerror?.((err as Error).message);
      }
    } finally {
```

Markup, after the Notes field:

```svelte
  <label class={labelClass}>
    Group
    <select bind:value={groupId} class={inputClass}>
      <option value={null}>Ungrouped</option>
      {#each groups as group (group.id)}
        <option value={group.id}>{group.name}</option>
      {/each}
    </select>
  </label>
  <SwatchPicker bind:value={color} allowNone />
  <IconPicker bind:value={icon} {customIcons} {onIconsChanged} onerror={(m) => onerror?.(m)} />
```

- [ ] **Step 3: Add the same three to `CreateProjectForm.svelte`**

Same imports and the same three `$state` values (initialised to `null` — no `state_referenced_locally` risk here, since there is no prop to reference). Add the same three markup blocks, and apply them after the create:

```ts
      const created = await createProject(input);
      // create_project takes only name/directory/description/tags, so the
      // three new fields are applied straight after. One extra call against a
      // local database, and CreateProjectInput stays unchanged.
      if (color || icon || groupId) {
        await updateProject(created.id, { color, icon, group_id: groupId });
      }
      name = "";
      directory = "";
      description = "";
      tags = "";
      color = null;
      icon = null;
      groupId = null;
      await onCreated();
```

with the same `isGroupNotFound` handling in the catch.

- [ ] **Step 4: Swap the group manager's icon `<select>` for `IconPicker`**

In `GroupManagerModal.svelte`, add `customIcons` to its props (drilled from `+page.svelte`), and replace both icon `<select>` elements with:

```svelte
        <IconPicker bind:value={newIcon} {customIcons} bundledOnly onerror={(m) => onerror?.(m)} />
```

That is the **new group** form only. For an existing group, `IconPicker` is the wrong shape for the same reason `SwatchPicker` was: it binds a value where re-iconing needs a call. Replace that group's icon `<select>` with a glyph button row that calls `handleReicon` directly, matching the swatch row beside it:

```svelte
            <div class="mt-2 flex flex-wrap items-center gap-1">
              {#each ICON_NAMES as name}
                <button
                  type="button"
                  disabled={busy}
                  onclick={() => handleReicon(group, name)}
                  title={name}
                  aria-label={`${name} icon for ${group.name}`}
                  aria-pressed={group.icon === name}
                  class={`inline-flex h-7 w-7 items-center justify-center rounded-sm border-2 ${group.icon === name ? "border-phos text-phos" : "border-transparent text-phos-dim hover:text-phos"}`}
                >
                  <BundledIcon {name} class="h-4 w-4" />
                </button>
              {/each}
            </div>
```

So in the group manager `IconPicker` appears exactly once, in the new-group form, always with `bundledOnly`. Group icons never come from the custom set: a custom SVG is an `<img>` and cannot be tinted, and a sidebar entry has to take its group's colour.

- [ ] **Step 5: Thread the new props through `ProjectList` and `+page.svelte`**

`ProjectList` passes `groups`, `customIcons`, `onIconsChanged` and `onGroupsStale` to `EditProjectForm`. `+page.svelte` supplies:

```ts
  async function handleIconsChanged() {
    await loadCustomIcons();
  }
```

and `onGroupsStale={loadGroups}`. `CreateProjectForm` gets `{groups} {customIcons} onIconsChanged={handleIconsChanged} onGroupsStale={loadGroups}`.

- [ ] **Step 6: Verify — including every icon-import failure mode**

Run: `pnpm test && pnpm run check` → 0 errors; note the warning count and reconcile it with Step 2's decision.

Run: `pnpm tauri dev`:
- Setting a colour on a project shows the pip in all three view modes, and the same colour in each.
- Setting a bundled icon shows it, tinted to the project colour.
- Assigning a group moves the project under that group's sidebar entry, and the left edge in All takes the group's colour.
- Clearing a group (choosing Ungrouped) always works, even for a group that has just been deleted.
- **Open a project's edit form, delete its group from the group manager in the same session, then save.** Expect the message "That group no longer exists — the selection was cleared. Save again.", the group select back at Ungrouped, and the project unchanged in the backend. Save again — it succeeds.
- **Import a valid SVG icon.** It appears in the picker and renders on the project.
- **Import each failure case and confirm the reason reaches the error banner, not a generic message:** an SVG containing `&nbsp;`, one with a bare `&` in text, one with `&xxe;`, a file over 256 KB, and an SVG containing only a `<script>` (nothing drawable). Each should name its own reason.
- **Import an SVG using `stroke-dasharray`** and confirm the dashes survive — this is what Task 1 bought.
- Deleting a custom icon removes it from the picker; a project still naming it falls back to the bundled `folder` glyph rather than breaking.

- [ ] **Step 7: Commit**

```bash
git add -A src/
git commit -m "$(cat <<'MSG'
feat(ui): add colour, icon and group pickers to the create and edit forms

The colour picker stores a palette name and the icon picker stores either a
bundled name or "custom:<slug>". Custom icons import through the dialog
plugin and render only as <img src="data:...">.

Import failures surface the backend's own message. It fails for content
reasons as often as IO ones — an undefined entity, a bare &, &nbsp; (an HTML
entity XML does not define, common in HTML-flavoured exports), nothing
drawable left, over 256 KB, a backslash in an attribute value — and each of
those is something the user can act on, which a generic "could not import"
would throw away.

Assigning a project to a group that has since been deleted returns
GroupNotFound; the form clears the stale selection and refetches the group
list rather than retrying, since the backend rejected the write and changed
nothing. Clearing a group is always allowed.

Group icons stay bundled-only: a custom SVG cannot be tinted and a sidebar
entry must take its group's colour.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

### Task 11: Documentation

`architecture.md`, `ROADMAP.md` and the backend half of `checklist.md` were **already brought up to date in the backend branch** — the schema section describes `groups` and `projects.group_id`, the module map lists `icons/` and `IconStore`, the dependency list includes `quick-xml`, invariant 10 records the `IconStore` exception, and the migration-fixture item moved to done in both. Read them; do not rewrite them. You are adding the frontend's story.

**Files:**
- Modify: `docs/checklist.md`, `CHANGELOG.md`, `docs/architecture.md`, `docs/handoffs/2026-09-05-project-views-frontend.md`, `docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`

- [ ] **Step 1: `docs/checklist.md`**

Move the **Project views, colour, icons and groups** bullet out of "Open (features)" into a completed section, replacing "*in progress*" with a description of what shipped. Add a frontend test-count line beside the Rust one: state the vitest total (`pnpm test` prints it) and name the four suites — `trackers`, `palette`, `icons`, `views`, `viewState`.

Add to the appropriate list:

```markdown
- [x] Three view modes (list / grid / compact) over one `ProjectList`, each presentation taking its action set as a snippet prop — which is what lets the same three layouts serve both the standard `···` menu and the Bin's Restore/Purge pair
- [x] Sidebar navigation — All / Favourites / groups / Ungrouped / Bin, with counts. `FavoritesModal` and `BinModal` are deleted; there is one navigation system rather than two, and the two modals' duplicated `SortControls` went with them
- [x] Palette colours resolved through `@theme` tokens (`--color-swatch-*`), never stored as literals; an unknown name falls back to neutral
- [x] 24 bundled line glyphs plus user-imported SVGs; custom icons render only through `<img src="data:…">`, and an unknown icon name draws the `folder` fallback
- [x] `viewState.ts` persists the selected view and the view mode in `localStorage` — never `projects.db`, which is the devmon cross-app contract. A view whose group has since been deleted falls back to All; Bin is never restored as the landing view
```

Also record, under the open items, the one thing this half deliberately did not build:

```markdown
- [ ] Keyboard navigation between sidebar entries — the obvious first binding for the registered-but-unbound global shortcut. Deliberately not absorbed into the views work.
```

- [ ] **Step 2: `CHANGELOG.md`**

Add an `## [Unreleased]` section (or extend the existing one) in Keep a Changelog form:

```markdown
### Added
- Three project view modes: list, grid and compact.
- A sidebar replacing the flat project list: All, Favourites, your groups, Ungrouped and Bin, each with a count.
- Groups — create, rename, recolour, re-icon, reorder and delete. A project belongs to one group at a time; deleting a group never deletes a project, its members become Ungrouped.
- A palette colour and an icon per project, and per group. Icons come from a bundled set of 24, or from your own SVGs.
- Search across project name, path and tags, applied within the selected view.

### Changed
- The Favourites and Bin modals are now sidebar views. Restore and the two-click permanent-delete confirm behave exactly as they did.

### Fixed
- The icon sanitizer no longer strips `stroke-dasharray`, `stroke-dashoffset`, `stroke-miterlimit`, `fill-opacity` or `stroke-opacity` — five inert presentation attributes real icon sets use routinely.
```

- [ ] **Step 3: `docs/architecture.md`**

Add one bullet under **Invariants worth protecting**, numbered 12, and one line to the frontend-page-state item in the Deferred list:

```markdown
12. **A stored colour is a palette name and a custom icon is never inlined.**
    `Project.color` and `Group.color` hold one of the eight names in
    `core::domain::palette` (mirrored in `src/lib/palette.ts`), resolved to a
    `var(--color-swatch-*)` token at render — never a literal — so a theme
    override recolours every project and group coherently. An unknown name
    falls back to neutral rather than failing. A **custom** icon renders only
    as `<img src="data:image/svg+xml;…">`: the sanitizer in `core::icons` is
    the first security layer and the `<img>` is the second, inert regardless
    of what the first one missed. `{@html}` on a stored SVG collapses two
    independent layers into one and must never appear.
```

Update the deferred **Frontend page-state extraction** entry, since `+page.svelte` grew:

```markdown
- **Frontend page-state extraction** (`lib/stores/*`). `+page.svelte` is now
  ~350 lines and holds the projects, the deleted projects, the groups, the
  custom-icon map, the selected view, the view mode and the query. The pure
  logic is already out (`views.ts`, `viewState.ts`, `palette.ts`, `icons.ts`);
  what remains is the fetch-and-refetch orchestration and the prop drilling of
  `groups` / `customIcons` down to `ProjectMark`. Still watch it rather than
  pre-splitting — the trigger is a second route needing the same state.
```

- [ ] **Step 4: Close out the handoff**

At the top of `docs/handoffs/2026-09-05-project-views-frontend.md`, change the status line to:

```markdown
**Status:** **complete** — implemented by
[`docs/superpowers/plans/2026-09-06-project-views-frontend.md`](../superpowers/plans/2026-09-06-project-views-frontend.md).
The five open questions in §6 are answered in that plan's "Decisions this plan
settles"; the two deviations from §3 and from the spec are argued in its
"Two deviations from the spec". Kept for the reasoning, not as live work.
```

- [ ] **Step 5: Reconcile the spec with what shipped**

In `docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`:

- In **Frontend structure**, change the `views.ts` bullet to record that sorting stayed on the backend:

```markdown
- `src/lib/views.ts` — `(view, live, deleted, query) → Project[]`, plus the
  per-view sidebar counts. It deliberately does **not** sort: the three list
  commands already sort in `core::domain::sorting`, tie-broken by the unique
  `id`, so filtering an already-sorted list preserves that order and a
  TypeScript comparator would be a second unchecked cross-language mirror.
```

- In the **Views** table, change Favourites' source from `getFavoriteProjects` to:

```markdown
| Favourites | `getAllProjects`, filtered to `favorite` — identical to what `get_favorite_projects` returns, because every comparator ends in the unique `id` so the sort is a total order |
```

- Mark the **Tasks** list items 10-20 as done.

- [ ] **Step 6: Verify the docs describe the code**

Re-read each edited paragraph against the file it describes. Specifically confirm: the swatch token names in `architecture.md` match `app.css`; the `views.ts` signature in the spec matches the exported function; the checklist's test counts match `pnpm test` output.

- [ ] **Step 7: Commit**

```bash
git add docs/ CHANGELOG.md
git commit -m "$(cat <<'MSG'
docs: record the frontend half of project views, colour, icons and groups

checklist.md moves the feature out of Open with the frontend's story;
CHANGELOG.md gets an Unreleased entry; architecture.md gains invariant 12
(a stored colour is a palette name, a custom icon is never inlined) and an
updated note on +page.svelte's size.

The spec is reconciled with what shipped on two points, both argued in the
plan: views.ts does not sort, because core::domain::sorting already does and
a TypeScript comparator would be an unchecked cross-language mirror; and
Favourites derives from the live list, which is provably the same list
get_favorite_projects returns.

The handoff is marked complete and kept for its reasoning.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
MSG
)"
```

---

## Final verification, before calling the whole plan done

Not a task — the gate over all of them. Run every item and record the actual output; a green suite is evidence about the tree it ran on and nothing else.

- [ ] `cargo test --workspace` — all pass.
- [ ] `cargo fmt --check` — clean. (Run `cargo fmt` *before* `git add`, not after.)
- [ ] `cargo clippy --workspace --all-targets 2>&1 | grep "^warning: " | sort | uniq -c` — exactly the two pre-existing warnings (`consider using sort_by_key`, `module has the same name as its containing module`) plus cargo's two summary lines. Four lines total on a clean tree; anything else is yours.
- [ ] `pnpm test` — all suites pass; note the total.
- [ ] `pnpm run check` — **0 errors**, and a warning count you have reconciled with Task 10 Step 2 and with `docs/KNOWN-ISSUES.md` PI-003.
- [ ] `pnpm run build` — succeeds.
- [ ] `pnpm tauri dev` — **the app launches and shows a window.** Then walk: every sidebar entry; all three view modes; search; sort; create with colour/icon/group; edit; delete; restore from the Bin; the two-click purge; the group manager's full CRUD and reorder; a custom icon import and a failed one; a restart to confirm the persisted view and mode come back, and that a view whose group was deleted lands on All.
- [ ] `git status` — clean, and nothing untracked that should have been committed.
