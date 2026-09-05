# Project views, colour, icons and groups — design

**Date:** 2026-09-05
**Status:** approved, ready for implementation planning

## Goal

Make the default project view something you *scan* rather than read. Three view
modes (list, grid, compact), a colour and an icon per project, and first-class
**groups**, surfaced as a **sidebar** that also absorbs the Favourites and Bin
modals.

Two of these are cosmetic and one is not: groups are a new domain entity with
their own table, which makes this the project's **first schema migration**.

## Motivation

The main view is a single flat `<ul>` of full-width rows. That is the right
shape for eight projects and the wrong one for eighty — and eighty is exactly
what the folder-scanning roadmap item will produce. Everything that
distinguishes one row from another today is text: a name, a path, some tracker
badges. There is no pre-attentive signal, so finding a project means reading.

Colour and an icon give each project a signal you can find without reading.
Groups give the *set* a shape — "client work", "personal", "archived" — which a
flat list cannot express at any length. Tags exist, but they are non-exclusive
labels: they can filter, they cannot organise, and a project with four tags
belongs under four headings or none.

The sidebar also fixes something that predates this work. Favourites and Bin are
modals, which makes them a second navigation system sitting on top of the first.
Once groups exist, there are five ways to slice the same list, and four of them
being a sidebar while two are modals is incoherent. They become sidebar entries.

This is also the moment the deferred migration-fixture work becomes due.
`ROADMAP.md` gates it on `CURRENT_SCHEMA_VERSION` going to 2; groups take it
there.

## Non-goals (YAGNI)

- **Inline group sections / collapsible bands.** Considered and rejected — see
  "Why a sidebar" below. There is no banding, no section header, no collapse
  state anywhere in this design.
- **Multi-group membership.** Membership is exclusive by decision. Tags remain
  the non-exclusive mechanism.
- **Nested groups.** One flat level. No trees, no group-in-group.
- **Drag-and-drop assignment.** A project's group is set in the create/edit
  form like every other field. Dragging a card onto a sidebar entry is an
  obvious later addition and is not built here.
- **Free-form hex colours.** Colours are named palette entries resolved through
  theme tokens. A literal `#rrggbb` is a possible later additive change, since
  the stored field is a string either way.
- **Tinting custom icons.** A custom SVG renders through `<img>` and keeps its
  own colours. The pip carries project colour instead.
- **Persisting sort.** Sort state is unpersisted today. Leaving it that way
  rather than widening scope; view mode and selected view are new state and do
  persist.
- **New backend queries for group membership.** Filtering by group happens on
  the frontend over the list already fetched. Project counts are small, and a
  `get_projects_in_group` command would be a second source of truth for
  something `getAllProjects` already returns.
- **Project linking / the graph view.** Recorded in `ROADMAP.md` under *Project
  linking*, and explicitly not built here.
- **UI plugins / themes.** Parked mid-brainstorm; the two settled decisions are
  recorded, and this spec is written so it does not contradict them.

## Decisions locked during brainstorming

1. **Three view modes** — list (today's rows), grid (tiles), compact (dense
   single-line rows).
2. **Colours are named palette entries**, resolved to theme tokens at render,
   not stored literals. A theme swap recolours every project coherently.
3. **Two colour levels** — a **primary** on the group, a **secondary** on the
   project distinguishing one project from another.
4. **Icons: a curated bundled set, plus user-supplied SVGs.**
5. **Custom SVGs render as `<img src="data:…">`**, sanitized on import. Inert by
   construction; keeps its own colours; cannot be tinted.
6. **Groups are first-class with exclusive membership** — a `groups` table plus
   a `group_id` on the project.
7. **Groups are navigation, not sections.** A sidebar lists **All**,
   **Favourites**, the groups, **Ungrouped** and **Bin**; selecting one changes
   what the main list shows.
8. **`FavoritesModal` and `BinModal` are retired**, becoming sidebar views.

### Why a sidebar

The first design sectioned the list into collapsible bands. It was rejected, and
the reason is worth recording so it is not re-proposed.

A collapsed band hides projects with nothing on screen indicating they exist, and
persisting that state across launches means the app starts by concealing things
the user has forgotten they hid. A sidebar inverts that: the current selection is
always visible, and so is the fact that other groups exist. **That is also why
the selected view can be persisted where collapse state could not** — the state
is legible, so restoring it cannot surprise anyone.

Sidebar navigation is additionally the standard pattern where a scalable set of
sections is switched between frequently, and it is what Obsidian, VS Code,
Todoist and Linear all use for this exact job. The cost, accepted: you see one
group at a time instead of the whole set at once.

## Architecture

### Domain — `crates/core/src/domain/group.rs`

```rust
pub struct Group {
    pub id: String,          // uuid v4, as Project does
    pub name: String,
    pub color: String,       // palette name, e.g. "cyan"
    pub icon: String,        // bundled icon name, e.g. "briefcase"
    pub position: i64,       // order in the sidebar
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

`icon` is present because a sidebar entry is icon + colour + label. (It was cut
in the sectioned design on the grounds that a heading is already text; that
reasoning died with the sections.) Group icons come from the bundled set only —
a custom SVG cannot be tinted, and a sidebar entry must take its group's colour.

Validation mirrors `Project`: name non-empty after trim, unique
case-insensitively, colour must be a known palette name, and icon must be
non-empty — core does not own the bundled icon list, so an unknown icon name
falls back to a default glyph at render rather than failing. `UpdateGroup`
follows the `UpdateProject` shape — `Option<T>` per field, absent means
unchanged.

A new group takes `position = max(position) + 1`, so it appends rather than
displacing existing entries. `set_group_positions` is the only thing that
renumbers, and it rewrites the whole ordering rather than patching one row.

**Errors.** Spec 1 settled the taxonomy: one public application error
(`ProjectError`) plus small port-level errors mapped into it. Groups and icons
follow it rather than introducing a parallel hierarchy — group validation
failures become `ProjectError` variants, and a new port-level `IconError`
(malformed SVG, oversize input, nothing drawable left) maps into it the way
`RepositoryError` already does.

### Domain — three new `Project` fields

```rust
#[serde(default)] pub group_id: Option<String>,
#[serde(default)] pub color: Option<String>,   // secondary
#[serde(default)] pub icon: Option<String>,    // "gamepad" | "custom:my-logo"
```

All three are `Option<T>`, so every record written by an older build still
loads — the contract documented at the top of `project.rs` and guarded by
`loads_a_record_missing_every_absorbable_field`, which gains assertions for
these three.

`UpdateProject` gains the same three as **double options**
(`Option<Option<String>>` with `deserialize_some`), matching `open_with` /
`notes` / `client`. All three are clearable, so "key absent" and "key present
but null" must stay distinguishable.

### Persistence — migration to `user_version = 2`

`CURRENT_SCHEMA_VERSION` becomes `2`, and `run_migrations` gains a `from < 2`
step:

```sql
BEGIN;
CREATE TABLE groups (
  id         TEXT PRIMARY KEY,
  name       TEXT NOT NULL,
  color      TEXT NOT NULL,
  icon       TEXT NOT NULL,
  position   INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE UNIQUE INDEX idx_groups_name_nocase ON groups(name COLLATE NOCASE);
CREATE INDEX idx_groups_position ON groups(position);
ALTER TABLE projects ADD COLUMN group_id TEXT
  REFERENCES groups(id) ON DELETE SET NULL;
CREATE INDEX idx_projects_group_id ON projects(group_id);
COMMIT;
```

`ADD COLUMN` with a `REFERENCES` clause is legal in SQLite provided the default
is `NULL`, which it is.

**Storage follows the existing house pattern: the JSON blob is the truth, the
column is a queryable mirror.** `project_tags` and `directory_normalized`
already work this way. `group_id` therefore lives in `data` *and* in the new
column, written together in the same statement as every other project save.

The `meta` mirror of `schema_version` already updates itself from
`CURRENT_SCHEMA_VERSION` on every open, so it needs no change — that was written
in anticipation of exactly this step.

### Group deletion is one transaction

`ON DELETE SET NULL` is a safety net, not the mechanism: it would fix the column
and leave a stale `group_id` inside the JSON blob. So `delete_group` does the
whole thing explicitly in a single transaction — load the affected projects,
clear `group_id` in each blob, write blob and column back, then delete the group
row.

**Deleting a group never deletes a project.** Its members become Ungrouped, and
the sidebar selection falls back to All.

### Ports — `crates/core/src/ports/repository.rs`

Split read from write exactly as `ProjectReader` / `ProjectRepository` are, so
devmon's read-only story stays consistent:

```rust
pub trait GroupReader: Send + Sync {
    fn get_group(&self, id: &str) -> Result<Option<Group>, RepositoryError>;
    fn list_groups(&self) -> Result<Vec<Group>, RepositoryError>;  // by position
}

pub trait GroupRepository: GroupReader {
    fn save_group(&self, group: &Group) -> Result<(), RepositoryError>;
    /// Clears membership and deletes the group in one transaction. Idempotent.
    fn delete_group(&self, id: &str) -> Result<(), RepositoryError>;
    /// Rewrites `position` to match the given order, in one transaction.
    fn set_group_positions(&self, ordered_ids: &[String]) -> Result<(), RepositoryError>;
}
```

The cross-blob transaction lives in the `SqliteRepository` implementation, not in
a service — one connection, one transaction, no way for a caller to get it
half-done.

### Application — `GroupService`

Holds `Arc<dyn GroupRepository>`. Create, rename, recolour, re-icon, reorder,
delete, list. Assigning a project to a group is a `ProjectService` concern
(`UpdateProject.group_id`) and needs no new method.

### Commands — `src-tauri/src/commands/groups.rs`

`create_group`, `update_group`, `delete_group`, `list_groups`, `reorder_groups`.
Thin, matching the existing command style. No group-membership query command —
see Non-goals.

### Custom icons

**Sanitizer** — `crates/core/src/icons/sanitize.rs`, pure logic, no Tauri (the
compiler enforces this). Allow-list, never deny-list:

- **Elements:** `svg`, `g`, `path`, `circle`, `ellipse`, `rect`, `line`,
  `polyline`, `polygon`, `title`, `desc`.
- **Attributes:** `viewBox`, `d`, `cx`, `cy`, `r`, `rx`, `ry`, `x`, `y`, `x1`,
  `y1`, `x2`, `y2`, `width`, `height`, `points`, `transform`, `fill`, `stroke`,
  `stroke-width`, `stroke-linecap`, `stroke-linejoin`, `fill-rule`, `clip-rule`,
  `opacity`.
- **Stripped unconditionally:** `script`, `style`, `foreignObject`, `use`,
  `image`, `animate*`, every `on*` handler, `href` / `xlink:href`, and any
  attribute whose value contains a `url(` or a `javascript:` scheme.
- **Caps:** 256 KB input; reject a result with no drawable element.

Parsing uses `quick-xml` — a new `core` dependency, justified by the rule that
you do not hand-roll an XML parser at a trust boundary. Its event API suits
allow-list filtering directly: read events, drop what is not permitted, write the
rest.

**Store** — `crates/core/src/infra/icon_store.rs`, an `IconStore` over a
directory. `src-tauri` resolves `app_config_dir()/icons/` and passes the path in,
so core never learns about Tauri. Operations: `import(&Path)`, `list()`,
`read(name)`, `delete(name)`.

**Commands** — `list_custom_icons`, `import_custom_icon(path)`,
`delete_custom_icon(name)`. The frontend receives `{ name, data_uri }` and
renders `<img>`. `img-src 'self' data:` is already present in **both** the
SvelteKit and Tauri policies, so no CSP change is required.

Defence in depth: even if the sanitizer misses a vector, an SVG referenced by an
`<img>` cannot execute script or load external resources. The sanitizer is the
first layer; the render path is the second.

### Colour

`app.css` gains a documented swatch palette in the `@theme` block — the existing
semantic tokens (`--color-accent`, `--color-gold`, …) are too few to distinguish
many projects, and using them for this would overload their meaning:

```css
--color-swatch-cyan:   …;   --color-swatch-violet: …;
--color-swatch-gold:   …;   --color-swatch-green:  …;
--color-swatch-amber:  …;   --color-swatch-blue:   …;
--color-swatch-rust:   …;   --color-swatch-pink:   …;
```

Eight, all tuned to read on the dark ground the way `trackerColor` already
guarantees for tracker hues. A stored colour is the bare name (`"cyan"`);
`src/lib/palette.ts` resolves it to `var(--color-swatch-cyan)` and falls back to
a neutral for an unknown name — the same ignore-what-you-do-not-know rule the
`--json` contract uses.

Because these are theme tokens, a future theme plugin restyles every project and
group colour coherently. That is the whole reason for storing names.

**Where each colour lands:**

| Level | Stored on | Renders as |
|-------|-----------|------------|
| Primary | `Group.color` | the group's sidebar entry, and a left edge on cards |
| Secondary | `Project.color` | a pip on the card, beside the icon |

The left edge earns its place in the mixed views — **All** and **Favourites**,
where projects from different groups sit together and the edge is what tells you
which is which. Inside a single group's view it is uniform, which is harmless.

The pip carries project colour rather than the icon, because a custom `<img>`
icon cannot be tinted — this keeps colour reading identically for bundled and
custom icons. Bundled icons additionally tint to the secondary colour, since they
are inline and can.

### Views

The sidebar selects a `View`, which is a data source plus an action set:

```ts
type View =
  | { kind: "all" }
  | { kind: "favorites" }
  | { kind: "group"; id: string }
  | { kind: "ungrouped" }
  | { kind: "bin" };
```

| View | Source | Row actions |
|------|--------|-------------|
| All | `getAllProjects` | standard `⋯` menu |
| Favourites | `getFavoriteProjects` | standard `⋯` menu |
| Group | `getAllProjects`, filtered by `group_id` | standard `⋯` menu |
| Ungrouped | `getAllProjects`, filtered to `group_id == null` | standard `⋯` menu |
| Bin | `getDeletedProjects` | Restore / Delete permanently |

**Bin keeps the behaviour the modal had**, and this is the part most at risk of
being lost in the move: restore, and a permanent purge whose button asks for a
second click (`"Delete permanently"` → `"Confirm?"`) rather than stacking a
confirmation on top. It never offers Open, Edit or Detect type — the directory
is gone. Favourites keeps open and un-favourite via the standard menu.

Because every view now shares one list surface, sort, search and view mode apply
uniformly. The two modals each carried their own `SortControls`; that duplication
disappears.

### Frontend structure

`ProjectCard.svelte` currently mixes identity, metadata and the actions menu.
Three modes and two action sets make that untenable, so presentation splits from
shared behaviour:

| File | Responsibility |
|------|----------------|
| `Sidebar.svelte` | the rail: All, Favourites, groups, Ungrouped, Bin |
| `SidebarEntry.svelte` | one entry — icon, colour, label, count |
| `ProjectActionsMenu.svelte` | the standard `⋯` menu, extracted once |
| `BinActions.svelte` | Restore / Delete permanently, with the two-click confirm |
| `ProjectMark.svelte` | icon + colour pip |
| `ProjectRow.svelte` | list presentation (today's card) |
| `ProjectTile.svelte` | grid presentation |
| `ProjectCompactRow.svelte` | compact presentation |
| `ProjectList.svelte` | flat list of the selected view |
| `ViewControls.svelte` | mode toggle + search box |
| `GroupManagerModal.svelte` | create / rename / recolour / re-icon / reorder / delete |

Presentation components take their actions as a snippet prop, so Bin supplies
`BinActions` where every other view supplies `ProjectActionsMenu`. That is what
keeps one set of row layouts serving both.

`FavoritesModal.svelte` and `BinModal.svelte` are deleted.

Two pure modules, so the logic is testable without mounting components:

- `src/lib/views.ts` — `(projects, groups, view, sort, query) → Project[]`, plus
  the per-view sidebar counts.
- `src/lib/palette.ts` — colour name → CSS custom property, with fallback.

`src/lib/viewState.ts` persists the selected view and the view mode in
`localStorage`. **Not** in `projects.db`: the database is a cross-app contract
that devmon attaches read-only, and UI preferences are none of its business.

Two rules on restoring it. A persisted view that no longer exists — a deleted
group — falls back to **All** rather than showing an empty list. And **Bin is
never restored as the landing view**; it is a destination you go to, not a place
to start. Both default to All.

## Data flow

1. `+page.svelte` loads projects (`getAllProjects`) and groups (`listGroups`),
   and reads the persisted view from `viewState`.
2. `Sidebar` renders All / Favourites / groups / Ungrouped / Bin with counts
   derived from the same fetched list.
3. `views.ts` resolves the selected view to a project array, applying the search
   query and sort. Favourites and Bin fetch from their own commands instead.
4. `ProjectList` renders that array in the active view mode, passing the view's
   action set to each row.
5. Editing colour, icon or group goes through `updateProject` and refetches, as
   every other edit already does. Group edits go through the group commands and
   refetch both.

## Invariants

### Preserved

- `indexer-core` does not import Tauri; the compiler enforces it.
- The app is the only writer of `projects.db`.
- The JSON blob is the truth; columns are queryable mirrors.
- A new `Project` field is `Option<T>` or `#[serde(default)]`.
- `open` refuses a database written by a newer binary.
- No component uses a raw colour; everything resolves through a theme token.
- Purging from the Bin still takes a second, deliberate click.

### New

- A stored colour is a **palette name**, never a literal. An unknown name falls
  back to neutral rather than failing.
- A custom icon is sanitized **on import**, not on render — the stored file is
  already safe, and nothing renders an unsanitized SVG.
- A custom icon renders only through `<img>` with a `data:` URI. Never inline.
- A group icon comes from the bundled set only, so it can take the group colour.
- Deleting a group never deletes a project.
- Group membership is exclusive: a project has zero or one group.
- No view can hide projects without saying so: the sidebar always shows every
  view and its count.

## Testing

Test-driven throughout, per the repo's practice.

**Rust**

- **Migration fixtures** — the scaffold `ROADMAP.md` gates on this exact version
  bump. Seed a database at `user_version = 1` with known rows, run `open`, assert
  the v2 result: tables present, existing projects intact, `group_id` null,
  `meta.schema_version` at `2`. Also assert a v2 database opens unchanged, and
  that a v3 database is still refused.
- **Group CRUD** — create, rename, recolour, duplicate-name rejection
  (case-insensitive), reorder, delete.
- **Delete clears membership** — members survive, `group_id` cleared in *both*
  the blob and the column; asserted by reading each back.
- **Project field absorption** — the legacy-record test gains the three new
  fields.
- **The sanitizer**, the heaviest suite here: `<script>` stripped, `onload=` and
  every `on*` stripped, `xlink:href` and `href` stripped, `foreignObject`
  stripped, `<use>` and `<image>` stripped, `url(…)` values rejected,
  `javascript:` rejected, entity-encoded and nested-CDATA payloads, oversize
  input rejected, an empty result rejected, and a well-formed icon surviving
  intact.

**Frontend (vitest, following `trackers.test.ts`)**

- `views.ts` — each view resolves to the right projects, Ungrouped catches
  `group_id == null`, query matches name/path/tags, sort applies within a view,
  counts are correct.
- `viewState.ts` — a persisted view for a deleted group falls back to All; Bin
  is never restored as the landing view.
- `palette.ts` — known name resolves, unknown name falls back.

**Manual, before calling it done.** Neither CI nor the pre-commit hook launches
the app, and `PI-005` is the standing reminder that a build can pass every gate
and still fail to show a window. This change touches startup (a migration runs on
open), so run the real thing on the platforms available. Specifically re-check
restore and purge from the Bin view, since that behaviour moved.

## Tasks

Ordered so each step is independently verifiable.

1. `Group` domain type + `UpdateGroup`, with validation tests.
2. Three new `Project` fields + `UpdateProject` double-options; extend the
   absorption test; mirror all of it into `src/lib/api/types.ts`, which is a
   hand-maintained mirror of the Rust models and drifts silently otherwise.
3. Migration fixtures scaffold, against the current v1 schema.
4. `CURRENT_SCHEMA_VERSION = 2` + the migration step; fixtures go green.
5. `GroupReader` / `GroupRepository` ports + `SqliteRepository` implementation,
   including the transactional delete.
6. `GroupService`.
7. `commands/groups.rs` + registration in `lib.rs`.
8. SVG sanitizer in core, with its full suite.
9. `IconStore` + the three icon commands + config-dir wiring.
10. Swatch palette in `app.css`; `palette.ts`.
11. `views.ts` + its tests.
12. Component split: `ProjectActionsMenu`, `ProjectMark`, then the three
    presentations. No behaviour change at this step — list mode must look as it
    does today.
13. `Sidebar` + `SidebarEntry`, with All / groups / Ungrouped only.
14. `ViewControls` (mode + search) + `viewState.ts`.
15. Fold Favourites into the sidebar; delete `FavoritesModal.svelte`.
16. Fold Bin into the sidebar with `BinActions`; delete `BinModal.svelte`.
    Verify restore and purge, including the two-click confirm.
17. `GroupManagerModal`.
18. Colour, icon and group pickers in the create/edit forms.
19. Bundled icon set.
20. Docs: `checklist.md`, `CHANGELOG.md`, and `architecture.md` where the schema
    is described.

## Risks and mitigations

| Risk | Mitigation |
|------|------------|
| The sanitizer misses a vector | The `<img>` render path is inert regardless — two independent layers, and the spec says never to inline a custom SVG |
| First migration corrupts a real database | Fixtures assert each step; `open` already refuses a newer database; the migration is one transaction |
| Blob and column diverge on `group_id` | Both written in the same statement; the transactional delete is the only other writer; tests read both back |
| Retiring the modals loses behaviour | Bin and Favourites are folded in as separate late tasks (15, 16), each verified against the modal it replaces, rather than rewritten alongside everything else |
| The component split regresses list mode | Task 12 is explicitly a no-behaviour-change refactor, verified before any new mode or view is added |
| Eight swatches are too few | Additive — more tokens, no stored-data change, because the stored value is a name |
| Scope creep into project linking | Recorded in `ROADMAP.md` as separate work with its own open questions |
