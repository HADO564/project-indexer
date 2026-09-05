# Handoff — project views, colour, icons and groups (frontend half)

**Date:** 2026-09-05
**Status:** blocked on the backend plan, then ready. Nothing here needs a design
decision first — the shape is settled and written down.
**Prerequisite:** [`docs/superpowers/plans/2026-09-05-groups-backend.md`](../superpowers/plans/2026-09-05-groups-backend.md)
complete and merged. Check it: if `list_groups` does not answer from the
devtools console, the backend is not done.

This is the second half of one feature. The first half builds the entire
backend — the `Group` entity, the first schema migration, an SVG sanitizer and
an icon store — so that this half can be written against a finished API rather
than alongside a moving one. **The spec governs both halves and is the document
to read first:**
[`docs/superpowers/specs/2026-09-05-project-views-grouping-design.md`](../superpowers/specs/2026-09-05-project-views-grouping-design.md).

---

## 1. What you are building, in one paragraph

The project list becomes something you scan rather than read. It gains **three
view modes** (list, grid, compact), a **colour and an icon per project**, and a
**sidebar** that replaces the flat single-column layout: All, Favourites, the
user's groups, Ungrouped, Bin. Selecting an entry changes what the list shows.
`FavoritesModal` and `BinModal` are deleted — they become sidebar views, so
there is one navigation system instead of two.

## 2. Decisions already made, and why

These were settled in brainstorming. They are not open, and the reasoning
matters more than the conclusions — reopen one only with new information.

**Groups are a sidebar, not inline sections.** The first design sectioned the
list into collapsible bands and it was rejected: a collapsed band hides projects
with nothing on screen saying they exist, and persisting that state means the
app starts by concealing things the user has forgotten hiding. A sidebar keeps
the current selection visible. **That is also why the selected view can be
persisted where collapse state could not** — the state is legible, so restoring
it cannot surprise anyone. If you find yourself adding a collapsible section,
re-read this paragraph.

**Colours are stored as palette names, never literals.** A project stores
`"gold"`, not `#e7b64e`. `palette.ts` resolves the name to a
`var(--color-swatch-*)` token at render. This exists so a future theme plugin
recolours every project and group coherently — the theme initiative has already
settled that **every** `@theme` token is user-overridable, so a hardcoded hex
picked against the dark ground could become unreadable under someone's theme.

**Custom icons render through `<img src="data:…">`, never inlined.** The
sanitizer in `core` is the first layer; the `<img>` is the second, and it cannot
execute script or fetch anything regardless of what the sanitizer misses.
Inlining a custom SVG collapses two layers into one. `img-src 'self' data:` is
already present in both content security policies, so nothing needs changing —
but the moment you reach for `{@html svg}`, stop.

**A bundled icon may be inlined and tinted; a custom one may not.** That is the
whole reason the **colour pip**, not the icon, carries project colour: colour has
to read identically for both kinds.

**Group membership is exclusive.** One group per project, so a project appears
under exactly one sidebar entry. Tags remain the non-exclusive mechanism; do not
rebuild tags as groups.

## 3. The API you are building against

All of this exists once the backend plan is merged.

**Types** — already mirrored into `src/lib/api/types.ts` by backend Task 2:

```ts
interface Project {
  // …everything it has today, plus:
  group_id: string | null;
  color: string | null;   // palette name, e.g. "gold"
  icon: string | null;    // "gamepad" | "custom:my-logo"
}

interface UpdateProject {
  // …plus these three; an explicit null clears, an absent key leaves unchanged
  group_id?: string | null;
  color?: string | null;
  icon?: string | null;
}
```

**Types you still have to add** to `types.ts` — the backend produces them but
only `Project`/`UpdateProject` were mirrored:

```ts
export interface Group {
  id: string;
  name: string;
  color: string;     // palette name
  icon: string;      // bundled icon name
  position: number;  // sidebar order
  created_at: string;
  updated_at: string;
}

export interface UpdateGroup {
  name?: string;
  color?: string;
  icon?: string;
}

export interface StoredIcon {
  name: string;
  svg: string;       // sanitized SVG source — NOT a data URI
}
```

**Commands:**

| Command | Args | Returns |
|---------|------|---------|
| `list_groups` | — | `Group[]`, ordered by `position` |
| `create_group` | `name`, `color`, `icon` | `Group` (appended last) |
| `update_group` | `id`, `update: UpdateGroup` | `Group` |
| `delete_group` | `id` | `void` — members become Ungrouped, none deleted |
| `reorder_groups` | `orderedIds: string[]` | `Group[]` in the new order |
| `list_custom_icons` | — | `StoredIcon[]` |
| `import_custom_icon` | `path` | `StoredIcon` |
| `delete_custom_icon` | `name` | `void` |

**Argument-casing gotcha.** Every existing command in `api/projects.ts` takes
single-word parameters (`{ id }`, `{ options }`), so the codebase does not
demonstrate the convention. `reorder_groups` takes `ordered_ids` in Rust —
pass it as **`orderedIds`**, which Tauri 2 converts. If it arrives as `null`,
try the snake_case key instead; do not change the Rust signature to dodge it.

**The eight palette names**, mirroring `crates/core/src/domain/palette.rs` —
that list is the source of truth, and `palette.ts` is its hand-maintained
mirror:

```
cyan  gold  amber  rust  violet  green  blue  pink
```

**Building the data URI** — core deliberately returns SVG source rather than a
data URI, to avoid a base64 dependency for something the browser assembles in
one expression:

```ts
const src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(icon.svg)}`;
```

## 4. The work — spec tasks 10 to 20

Write the plan for these with the `superpowers:writing-plans` skill; the spec
lists them, and the ordering below is the one it settled on.

10. **Swatch palette in `app.css`** (eight `--color-swatch-*` tokens in the
    `@theme` block) and `src/lib/palette.ts` resolving a name to a token, with a
    neutral fallback for an unknown name.
11. **`src/lib/views.ts`** — the pure module: `(projects, groups, view, sort,
    query) → Project[]`, plus per-view sidebar counts. Tested in vitest, no
    components mounted.
12. **Component split** — extract `ProjectActionsMenu` and `ProjectMark` out of
    `ProjectCard`, then the three presentations (`ProjectRow`, `ProjectTile`,
    `ProjectCompactRow`). **No behaviour change in this task**: list mode must
    look exactly as it does today, and that is verified before anything new is
    added.
13. **`Sidebar` + `SidebarEntry`**, with All / groups / Ungrouped only.
14. **`ViewControls`** (mode toggle + search) and `src/lib/viewState.ts`.
15. **Fold Favourites in**; delete `FavoritesModal.svelte`.
16. **Fold Bin in** with `BinActions`; delete `BinModal.svelte`.
17. **`GroupManagerModal`** — create, rename, recolour, re-icon, reorder, delete.
18. **Colour, icon and group pickers** in the create/edit forms.
19. **The bundled icon set** (~24 lucide-style inline SVGs, keyed by name).
20. **Docs** — `checklist.md`, `CHANGELOG.md`, and `architecture.md` where the
    schema is described.

Tasks 15 and 16 are deliberately late and deliberately separate. Each retires a
working feature, so each is verified against the modal it replaces rather than
rewritten in the middle of everything else.

## 5. Things that will bite you

**`ProjectCard.svelte` mixes three responsibilities** — identity, metadata, and
the actions menu — in one component. Three view modes and two action sets make
that untenable, which is why task 12 exists and why it comes before any new
view. Resist building the grid tile first; the split is what makes the tile
cheap.

**Bin's actions are not the standard menu, and its behaviour is easy to lose.**
`BinModal` currently offers Restore and a permanent purge whose button asks for
a second click (`"Delete permanently"` → `"Confirm?"`) rather than stacking a
confirmation dialog on a dialog. That two-click confirm is a **preserved
invariant** in the spec. Bin rows must not offer Open, Edit or Detect type — the
directory is gone. Presentation components take their action set as a snippet
prop; that is what lets one set of row layouts serve both.

**Both modals carry their own `SortControls`.** Once every view shares one list
surface, that duplication disappears — do not port it across.

**Restoring the persisted view has two rules.** A view whose group has since
been deleted falls back to **All** rather than rendering an empty list. And
**Bin is never restored as the landing view** — it is a destination, not a home.

**`style-src` still carries `unsafe-inline`,** because components compute colours
into a `style` attribute (`trackerColor(kind)` most of all). Applying a swatch
token lands in the same bucket. This is *not* licence to interpolate arbitrary
strings into CSS — resolve a name to a token and set a custom property. Moving
the computed colours to custom properties is the prerequisite for ever
tightening this, and this feature is a good opportunity to not make it worse.

**`src/lib/api/types.ts` is a hand-maintained mirror** of the Rust models. It
drifts silently — nothing checks it. Update it in the same commit as anything it
mirrors.

**`pnpm run check` has a known baseline:** 0 errors and 8
`state_referenced_locally` warnings, all in `EditProjectForm.svelte`. That is
`PI-003`, a documented false positive. Do not "fix" them; do not add a ninth.

**Neither CI nor the pre-commit hook launches the app.** `PI-005` compiled,
passed every test, and still exited before showing a window. This half changes
the whole page layout, so run `pnpm tauri dev` and look at it.

**Install the pre-commit hook** if this is a fresh clone: `git config
core.hooksPath .githooks`. It is local git config and is not carried by the
repo. It runs `pnpm run check`, `pnpm test` and `pnpm run build` for web
changes, and only the gates your staged files can affect — a docs-only commit
costs nothing.

**Commit trailer:** every commit ends with a `Co-Authored-By:` line naming the
model that did the work.

## 6. Open questions for this half

Genuinely undecided. None blocks starting; settle each when you reach it.

1. **Sidebar width, and whether it collapses to an icon rail.** A rail is the
   obvious space saving and the obvious way to reintroduce a hiding problem —
   though a rail still shows every entry, which is the property that mattered.
2. **Does the sidebar show counts?** The mockup did. Counts make an empty group
   visible, which is good; they also need recomputing on every change.
3. **Where group management lives.** The spec says `GroupManagerModal`. A case
   exists for inline editing in the sidebar instead, which is nicer and fiddlier.
4. **What an unknown icon name renders as.** The spec requires a fallback glyph
   rather than a failure; which glyph is unspecified.
5. **Keyboard navigation.** The app has a registered global shortcut bound to
   nothing. Moving between sidebar entries is the obvious first binding, but
   that is a separate roadmap item — do not quietly absorb it.

## 7. Reference map

| What | Where |
|------|-------|
| The governing spec, both halves | `docs/superpowers/specs/2026-09-05-project-views-grouping-design.md` |
| The backend plan this depends on | `docs/superpowers/plans/2026-09-05-groups-backend.md` |
| Theme tokens, and where swatches go | `src/app.css` (`@theme` block) |
| Shared Tailwind utility strings | `src/lib/components/styles.ts` |
| The component being split | `src/lib/components/ProjectCard.svelte` |
| The modals being retired | `src/lib/components/{FavoritesModal,BinModal}.svelte` |
| Generic tracker rendering (leave alone) | `src/lib/trackers.ts`, `TrackerPanel.svelte`, `TrackerBadges.svelte` |
| Frontend test precedent | `src/lib/trackers.test.ts` |
| Content security policy | `svelte.config.js`, `src-tauri/tauri.conf.json` |
| Palette source of truth | `crates/core/src/domain/palette.rs` |
| Known issues, including what CI cannot catch | `docs/KNOWN-ISSUES.md` |
| The parked theme-plugin initiative | `docs/handoffs/2026-09-04-plugins.md`, `docs/checklist.md` |
