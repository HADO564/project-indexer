# Project Indexer — Architecture & Quality

Companion to `checklist.md`. That file tracks *features*; this one tracks the
*shape* of the system — the load-bearing decisions worth protecting, and the
quality work that keeps adding the next detector/platform/view cheap rather
than progressively more expensive.

Read `knowledgebase.md` first for how the pieces currently work. This file is
about direction, not mechanics.

## The system today

A Cargo workspace with three crates. `indexer-core` (`crates/core`) holds all
domain logic, orchestration, and persistence and has **no `tauri` dependency**;
`src-tauri` is a thin GUI adapter over it; `crates/cli` is a one-line stub for a
future observer CLI (Spec 2).

```
  ┌──────────────────────┐        ┌──────────────────────┐
  │   src-tauri  (GUI)   │        │   crates/cli  (stub) │
  │  commands/*.rs — ~3- │        │  eprintln! only —    │
  │  line #[tauri::command]│      │  Spec 2 will fill it │
  │  pass-throughs       │        │  in                  │
  │  adapters/opener_    │        └──────────┬───────────┘
  │  launcher.rs         │                   │
  │  lib.rs setup wiring │                   │
  └──────────┬───────────┘                   │
             │        ┌──────────────────────┘
             ▼        ▼
  ┌───────────────────────────────────────────────────────┐
  │            crates/core  «indexer-core»                 │
  │                    NO tauri, NO clap                   │
  │                                                       │
  │  application/  ProjectService — one method per command │
  │                GroupService — group CRUD + reorder     │
  │                inspection (ProjectInspection DTOs)     │
  │      │                                                 │
  │      ├─► ports/    ProjectReader + ProjectRepository   │
  │      │             GroupReader + GroupRepository        │
  │      │             AppLauncher                         │
  │      ├─► domain/   Project · Tracker · UpdateProject   │
  │      │             Group · UpdateGroup · palette        │
  │      │             git · unreal · normalize · sorting  │
  │      │             naming                              │
  │      ├─► detectors/ Detector · DetectorRunner ·        │
  │      │              registry · git/ · unreal/          │
  │      ├─► platform/  filesystem · app_discovery         │
  │      ├─► icons/     sanitize_svg — the SVG sanitizer    │
  │      ├─► infra/     SqliteRepository · IconStore        │
  │      └─► error/     ProjectError (+ the three port errs)│
  └───────────────────────────────────────────────────────┘
             │ impl ProjectRepository, GroupRepository
             ▼
     app_config_dir()/projects.db   (SQLite, WAL, foreign_keys=ON)
```

Dependency direction is compiler-enforced: `src-tauri → indexer-core` and
(later) `crates/cli → indexer-core`, never the reverse, and a `use tauri::` in
`core` fails to compile. `indexer-core` depends only on std, serde, serde_json,
chrono, uuid, thiserror, git2, rusqlite, quick-xml (+ `winreg`/`parselnk` on
Windows).

The GUI's `#[tauri::command]` functions are ~3-line pass-throughs over
`State<Arc<ProjectService>>`; `AppHandle` is gone from every signature. The one
genuine adapter is `adapters/opener_launcher.rs` (`OpenerLauncher impl
AppLauncher`) — the only place `tauri-plugin-opener` is still used.

See `docs/superpowers/specs/2026-09-02-frontend-agnostic-core-design.md` for the
full design, the **devmon** cross-app contract (§"Cross-app compatibility"), and
the updater / release-notification / CLI-install / signing-CI fast-follows
(§"App updates").

## Invariants worth protecting

Load-bearing decisions. Break one only on purpose, and add a test that fails
if it regresses.

1. **A new detector is "implement + register" — nothing else.** It touches
   its own module, one line in `detectors/registry.rs`, a `Tracker`
   variant, its `*Info` model, and `Detector::kind()`. **Zero frontend
   code:** the generic `TrackerPanel` renders any tracker, inferring each
   field's affordance from its name/shape (`src/lib/trackers.ts`
   `inferType`) — a `https://…` value → link (the key name isn't
   consulted); a `*_root` / `*_path` / `*_dir` key (or one containing
   `directory`) → path; a `*hash*` / `*commit*` key → code; an ssh /
   `git@` value → code; a non-empty array → chips; a bool → flag (shown
   only when true); everything else → text. A `null` / `undefined` /
   empty value (or an empty array) is dropped. No runner, command-layer,
   or `DetectorError` change either — the `Other` variant is the escape
   hatch for a detector's own error type. The new tracker's badge/tab
   colour also comes for free: `trackers.ts` `trackerColor(kind)` gives a
   known kind a hand-picked hue and anything else a stable name-hashed one,
   all at a fixed lightness/saturation so the text always contrasts.
2. **Basic detection stays cheap and bounded.** `detect_project` runs on
   every `create_project` and every browse-prefill keystroke-ish action. A
   detector that needs to walk history, parse a dependency graph, or scan
   assets does *not* belong in that path — see "Fast vs deep detection" below.
3. **Project identity is a stable UUID, never the directory path.** Generated
   once in `Project::new`, never regenerated — there's a test
   (`rejects_a_record_missing_its_identity`) that a record with no `id` fails to
   load rather than getting a fresh one. It's also the `projects` table PK and
   the foreign key an external reader (devmon) stores. Anything built later
   (history, per-project settings, export/import) keys off the UUID, and a
   project survives its directory moving.
4. **Directory normalization is deliberate and case-sensitive.** `C:\Foo\` and
   `C:/Foo` collide; `C:\Foo` and `C:\foo` do not. This is a choice, not an
   oversight (`core::domain::normalize`). The normalized form is now also a
   stored column (`projects.directory_normalized`, indexed) backing
   `find_by_directory` and the dup check. See "Considered and declined".
5. **Validation is advisory; the final filesystem operation is
   authoritative.** `check_directory_health` before an open/refresh is a
   courtesy for a better error message — the actual `open`/`read_dir`/`remove`
   still has to handle the directory vanishing a millisecond later. Don't add
   locking; do make the last operation own the failure.
6. **"What a project is" and "what opens it" stay decoupled.** `Tracker`
   (detected type) and `open_with` / `InstalledApp` (launcher) are separate
   concerns that happen to both touch a directory. A `.uproject` tracker must
   not imply "launch with Unreal".
7. **Detectors are independent and unordered.** The runner consults all of
   them and collects everything that matches; a directory can legitimately be
   git + Unreal + Unity at once. No detector may depend on another's result or
   on running first.
8. **Old stored records keep loading.** New `Project` fields are `Option<T>`
   or `#[serde(default)]` — enforced by
   `loads_a_record_missing_every_absorbable_field`. A shape change serde can't
   absorb is now a numbered `user_version` migration step in
   `SqliteRepository` that rewrites the `data` blobs in a transaction. What's
   gone: the old `serde_json::Value` `migrate()` layer and its `schema_version`
   stamp (see Recorded decisions).
9. **`core` never depends on `tauri`.** Compiler-enforced by the crate graph —
   a `use tauri::` anywhere in `indexer-core` fails to build, and
   `cargo tree -p indexer-core` shows no `tauri`. This is what guarantees "add
   a frontend (CLI, …) without reworking the backend".
10. **All persistence goes through a repository port.** No frontend touches
    SQLite directly; `ProjectService` is the only caller of `ProjectRepository`
    and `GroupService` the only caller of `GroupRepository`. Each has a
    separate reader trait (`ProjectReader`, `GroupReader`) so an external
    consumer (devmon) can depend on read access without the write surface.
    (One exception — see below.)

    *Exception: `IconStore`.* `src-tauri/src/commands/icons.rs` calls
    `indexer_core::infra::IconStore` directly — `State<'_, Arc<IconStore>>`,
    no service, no port — the only place in the codebase that skips the
    commands → application → port → infra chain. This is deliberate, not a
    precedent: icon import/list/delete has no orchestration for a service to
    hold (each command is already a one-line pass-through to the store), and
    a port with exactly one implementation would be speculative abstraction.
    If a second `IconStore` implementation ever shows up, or any orchestration
    grows around icons (validation against something else, cross-referencing
    groups, …), it should adopt the `ProjectReader`/`GroupReader` shape — a
    plain `IconReader` port `IconStore` implements — which is also what would
    give the planned `devmon` app a read-only story for icons, the same way
    it gets one for projects and groups.
11. **The binary owns forward migration; it never reads a newer DB.**
    `SqliteRepository::open` runs the `user_version` steps up to
    `CURRENT_SCHEMA_VERSION` and returns
    `RepositoryError::Backend("database is from a newer version …")` for
    anything higher, without touching the data. Guarded by
    `refuses_a_newer_database`. This is what makes shipping auto-updates safe —
    a downgraded binary fails loud instead of corrupting the store.

12. **A stored colour is a palette name, and a custom icon is never inlined.**
    `Project.color` and `Group.color` hold one of the eight names in
    `core::domain::palette` (mirrored, hand-maintained, in `src/lib/palette.ts`),
    resolved to a `var(--color-swatch-*)` token at render — never a literal — so
    a theme override recolours every project and group coherently. An unknown
    name falls back to neutral rather than failing, and an unknown *icon* name
    falls back to a default glyph, because `core` owns neither the icon list nor
    the render. A **custom** icon renders only as
    `<img src="data:image/svg+xml;…">`: the sanitizer in `core::icons` is the
    first security layer and the `<img>` is the second, inert regardless of what
    the first one missed. `{@html}` on a stored SVG collapses two independent
    layers into one and must never appear. `swatchVar()` returning one of a
    fixed set of `var(--color-*)` strings is what keeps a stored colour name out
    of the `style` attribute as arbitrary CSS.

## Detection semantics

The runner returns `Detection { outcomes }` — one `DetectorOutcome` per
detector consulted, in registration order — and the taxonomy is now a doc
comment on `DetectorOutcome`. The states:

| State | How it's represented | Example |
|---|---|---|
| **Not mine** | `DetectorOutcome::NotDetected` | a plain directory, to `Gitector` |
| **Detected** | `DetectorOutcome::Detected { tracker }` | a git repo → `Tracker::Git(info)` |
| **Detected, partial** | `Detected` with `None` fields on the tracker | git repo with no remote (`repo_url: None`) — a normal state, not an error |
| **Detector failed** | `DetectorOutcome::Failed { error }` | libgit2 can't read a corrupt repo; `.uproject` is malformed JSON |
| **Path unusable** | `Failed`, or refused earlier by `check_directory_health` | directory deleted mid-operation |

`Detection::trackers()` and `errors()` project the `Detected` / `Failed`
outcomes back out for best-effort callers; `into_result()` is the
all-or-nothing view.

The load-bearing distinction: **"malformed `.uproject`" and "not an Unreal
project" are not the same outcome** and must never collapse into one. A
detector returns `Ok(None)` for "not mine" and `Err` for "mine but broken".

The persist paths (`ProjectService::create`, `ProjectService::refresh_trackers`)
still store only `trackers`. The `/project/[id]` view closed the visibility gap
a different way: it calls `inspect_project` (read-only, live) and renders every
outcome — `● git · ○ unreal — not detected · ▲ unity — <error>` — so a
detector that fails is no longer indistinguishable from one that found
nothing.

## Recorded decisions

Choices that could plausibly have gone the other way, settled on purpose so
they don't drift into "that's just how it ended up". Each is guarded by a
test whose name is the sign that changing it is a real decision.

### Refresh is all-or-nothing

`ProjectService::refresh_trackers` — the explicit, user-triggered "re-scan this
project" (the `refresh_project_trackers` command is a pass-through to it) —
persists the detection result verbatim or not at all. If any registered
detector errors, the call fails and the stored trackers are left untouched; it
does **not** save the detectors that happened to succeed.

*Why:* detection results are stored as-is and drive the detail view. A
persisted tracker set silently missing whatever a failing detector would have
produced is a worse outcome than a visible "refresh failed" the user can
retry. `ProjectService::create` and the browse preview stay best-effort —
there's no prior good state to protect there.

*Revisit when:* there are enough independent detectors that losing an
unrelated tracker to one detector's transient failure is the common case. The
alternative is to persist the `Detected` outcomes and keep the `Failed` ones
as per-detector status (the `/project/[id]` view already shows this live from
`inspect_project`; this would carry it into the stored record too). That's a
deliberate change, not a detector quietly learning to tolerate partial state.

*Guarded by:* `refresh_all_or_nothing_leaves_stored_trackers_on_detector_failure`
in `application/service.rs` (plus the runner-level
`into_result_discards_partial_trackers_on_any_error` and the
`Detection::into_result` doc comment).

### SQLite as a document store

`Project` is stored as its full serde JSON in a `data TEXT` blob (source of
truth), with four columns *promoted* out of it for querying —
`is_deleted` (list filtering), `directory_normalized` (dup check /
`find_by_directory` / activity attribution), `updated_at` (sort, RFC3339 UTC),
`group_id` (the project's group membership, filtering by sidebar view). `tags`
is additionally mirrored into a `project_tags(project_id, tag)` table,
rewritten on every `save` inside the same transaction — a derived projection for
future SQL-level tag queries (devmon, search); the blob stays authoritative and
nothing reads `project_tags` back yet.

Groups get their own table, `groups(id, name, color, icon, position,
created_at, updated_at)` — a real row per group rather than a blob, since a
group has no sum-type payload to dodge and the sidebar needs to query and order
them directly. `projects.group_id` is a foreign key onto it
(`ON DELETE SET NULL` as a safety net; `GroupRepository::delete_group` clears
the blob's copy and the column together in one transaction, since the
constraint alone would leave the blob's `group_id` stale). This is
`user_version` 2, the first schema migration since the SQLite move — see
`run_migrations` in `SqliteRepository` and the "Migration fixtures" entry
below.

*Why not fully relational (trackers):* `trackers` is a `Vec<Tracker>` where
`Tracker` is a sum type with per-variant payloads (`GitInfo`, `UnrealInfo`, …).
Normalizing it means a satellite table and a migration *per detector*, which
breaks invariant 1 ("add a detector = zero persistence change"). SQL querying
of tracker internals ("all dirty git repos") is a known deferred
`user_version` migration — promote fields or add a `project_trackers` table
when a feature needs it. Until then the blob is scanned in Rust. Groups didn't
face this problem, hence the different treatment.

*Guarded by:* the `SqliteRepository` round-trip / upsert / cascade / tag tests,
the group round-trip / position-ordering / cascade-ungroup tests, and
`fresh_db_is_at_current_schema_version`.

### Tauri-free `core` crate

The GUI is one frontend. All domain logic, orchestration and persistence live in
`indexer-core`, which cannot `use tauri` (invariant 9). A future CLI (Spec 2)
and a separate activity tracker (devmon) attach via the same crate and the same
`projects.db` — no IPC, no pairing. `src-tauri` keeps only the Tauri Builder
wiring, the ~3-line command wrappers, and the one `OpenerLauncher` adapter.

*Why:* a crate boundary the compiler enforces is the only way to *guarantee*
"add a frontend without reworking the backend"; a convention wouldn't hold.

### No `serde_json::Value` migration layer

The old `migrations/` module (`migrate()`, `CURRENT_VERSION`, the
`schema_version` field stamp) walked stored JSON to upgrade legacy records. It
was **deleted, not moved**: the app has no production data, so there were no
legacy records to upgrade. Schema evolution now has exactly one mechanism — the
numbered `user_version` runner in `SqliteRepository` — and field additions still
rely on `#[serde(default)]` / `Option<T>` (invariant 8).

### `projects.db` is a cross-app contract — devmon

`projects.db` is project-indexer's alone. A planned separate app, **devmon** (an
activity/work tracker), will `ATTACH` it **read-only** to attribute observed
activity to projects. The persistence design keeps that possible without rework:
stable UUID PK, indexed `directory_normalized`, a `meta(key, value)` table
carrying `app` and `schema_version` rows an external reader checks before
joining, RFC3339 UTC timestamps
throughout, WAL + `busy_timeout` for concurrent read-while-write, and a
read-only `ProjectReader` trait devmon can depend on (ideally via the
`indexer-core` crate, reusing `normalize` + `find_by_directory` rather than
reimplementing them). devmon's own activity/metric tables live in `devmon.db` —
never here. Full contract: the spec's §"Cross-app compatibility — devmon".

### Windows: launch `open_with` apps ourselves, not via the shell

`OpenerLauncher::open` on Windows (the body moved verbatim from the old
`system.rs::open_in_app`) spawns a chosen executable with
`std::process::Command` (env scrubbed of `ELECTRON_RUN_AS_NODE` /
`ELECTRON_NO_ATTACH_CONSOLE`, detached, no console), rather than routing
through the opener plugin's `ShellExecuteExW`.

*Why:* `ShellExecuteExW` gives the child the caller's environment with no way
to change it. When Project Indexer is started from a VS Code terminal it
inherits `ELECTRON_RUN_AS_NODE=1`, and every Electron `open_with` target
(VS Code, Cursor, Slack, …) then runs as plain Node — `Code.exe <folder>`
tries to `require()` the folder and exits, while `ShellExecuteExW` still
reports success. The packaged app launched normally never has the variable;
this only bites when running from a dev shell, but it's a whole class of
"editor won't open" with a one-line cause.

*Scope:* only concrete executable paths (`open_with` contains a separator).
Bare command names and the system-default open keep the opener plugin, which
resolves them via the registry's App Paths / PATHEXT.

## Quality backlog

Curated and reordered from a broader architectural review. Prioritized by
*payoff now*, not by how interesting the problem is.

### Now — cheap, and makes the next detector cheaper

- [x] **Explicit tracker/detector identity.** `Detector::kind() -> &'static str`
      (`"git"`, `"unreal"`) lands with each detector and tags every
      `DetectorOutcome`; `inspect_project` surfaces it to the frontend as a
      real `kind` string. `trackers.ts` still reads the *variant* name off the
      serde shape for tab labels, but detection identity no longer rides on
      JSON structure. (`Tracker::kind()` itself wasn't needed — the outcome
      `kind` covers every call site.)
- [ ] **Write down the detection semantics** (the table above) as a doc
      comment on `Detection` / the `Detector` trait, and add a test that a
      malformed descriptor is an `Err`, not `Ok(None)`.
- [ ] **Detector fixtures.** `src-tauri/tests/fixtures/{git,unreal}/…`
      (clean, dirty, unborn, detached; minimal, plugins, source-control)
      instead of building every scenario by hand in each test. Pays for itself
      at Unity/Blender.
- [x] **Reconcile lockfiles.** Done during the core refactor: committed to
      **pnpm** — `package-lock.json` deleted, `packageManager: pnpm@11.21.0`
      pinned in `package.json`, `beforeDevCommand`/`beforeBuildCommand` are
      `pnpm dev`/`pnpm build`. With `node_modules` pnpm-managed and the lockfile
      matching, `pnpm dev` no longer relocates packages into
      `node_modules/.ignored` (the blank-screen bug that bit GUI v1).
- [ ] **PI-004** — fix the NVIDIA-workaround comment wording in `lib.rs`.

### Next — before or alongside the Unity detector

- [x] **Per-detector status to the UI.** `inspect_project` returns one
      `DetectorResult { kind, status, tracker?, error? }` per detector and the
      `/project/[id]` view renders the full strip — "Git: detected · Unreal:
      failed · Unity: not detected" — not just successes. Read-only and live;
      the stored record is still successes-only (see "Refresh is
      all-or-nothing").
- [x] **Extract detection orchestration** — done, and further than this item
      scoped. The frontend-agnostic-core refactor lifted *all* orchestration
      (not just detection) into `core::application::ProjectService`, one method
      per command, callable with no Tauri `State`. The `#[tauri::command]`
      functions are now ~3-line pass-throughs.
- [x] **Command-layer integration tests.** Done as `ProjectService` tests
      (`application/service.rs`): in-memory SQLite + a `FakeLauncher` drive
      create / dup-reject / open (missing dir, missing app, success +
      `mark_opened`) / all-or-nothing refresh / bin-only delete guard /
      `delete_directory` both branches / restore / inspect-bad-dir /
      `ensure_project` idempotency.
- [x] **Migration fixtures.** Trigger fired: `CURRENT_SCHEMA_VERSION` went to 2
      for the groups schema (`user_version` 1→2, see "SQLite as a document
      store"). `crates/core/tests/migrations.rs` seeds a `user_version = 1`
      database with representative rows, runs `open`, and asserts the v2
      result — `groups` present, existing projects intact and left ungrouped,
      `meta.schema_version` at `2`. Also asserts an already-v2 database opens
      unchanged and that `schema_version` is stamped regardless of which steps
      ran. The version-skew guard (invariant 11) was already tested and is
      unaffected.

### Deferred — gated on a concrete trigger, not a date

- **Fast vs deep detection tiers.** When the first detector genuinely needs
  expensive work (Git contributors via revwalk, dependency parsing), split
  "cheap marker/metadata detection" from opt-in "deep inspection" — probably a
  separate command and a cache keyed on directory + HEAD. Until then, one tier.
- **Platform provider traits.** *Partially done.* `AppLauncher` is now a real
  port (`core::ports::launcher`) with `OpenerLauncher` as the GUI impl; all
  installed-app discovery and the filesystem checks moved into
  `core::platform`. Still a plain function, not a trait behind a port:
  `core::platform::list_installed_apps()` (`app_discovery.rs`). Give it a trait
  *as* the macOS work — that's when a third impl makes the seam pay.
- **Structured detection logging** (`detector · duration · result`). Low value
  at 2–6 detectors; revisit if detection gets slow enough to debug.
- **Frontend page-state extraction** (`lib/stores/*`). `+page.svelte` is now
  ~330 lines and holds the projects, the deleted projects, the groups, the
  custom-icon map, the selected view, the view mode and the query. The pure
  logic is already out — `views.ts`, `viewState.ts`, `palette.ts`, `icons.ts`,
  all tested without mounting anything — so what remains is fetch-and-refetch
  orchestration plus the prop drilling of `groups` / `customIcons` down to
  `ProjectMark`. `/project/[id]` now fetches the same two lists for its copy of
  the edit form, which is the first real duplication. Still watch it rather than
  pre-splitting; the trigger is a third consumer of that state.

### Considered and declined

- **Platform-aware case folding for directory identity** (invariant 4). The
  failure it prevents — registering `C:\Foo` and `C:\foo` as two projects on
  Windows — is rare and self-correcting (the user sees both). Making
  normalization OS-dependent adds a cross-platform behavior fork to a
  currently simple, well-tested function. Keep it case-sensitive; add a test
  that pins the current behavior per platform and move on.
- **Detector metadata / capabilities / priority / short-circuiting.**
  Detectors are independent (invariant 7); there's no contention to arbitrate.
  Revisit only if two detectors genuinely need to coordinate, which none do.
- **Reworking the contributors deferral.** Already correctly deferred
  (`checklist.md`); the plan (revwalk → `Vec<Contributor>`, with caching)
  already accounts for the cost. No change needed now.

## Cross-app & updates — next initiatives

Named here so the seams aren't rediscovered. Full designs are in the spec
(`docs/superpowers/specs/2026-09-02-frontend-agnostic-core-design.md`).

- **Observer CLI (Spec 2).** `crates/cli` is a stub today. Spec 2 fills it in:
  `indexer <cmd>` wraps a real command, then matches argv + cwd + exit code
  against recognizers (`git init`, `git clone`, `cargo new`, …) and records
  inferred project facts through the same `ProjectService`
  (`ensure_project` / `find_by_directory` / `refresh_trackers`, already added).
  Plain subcommands (`indexer list`, …) too. No IPC with the GUI — both open the
  same `projects.db`.
- **devmon.** A separate activity tracker that `ATTACH`es `projects.db`
  read-only for activity attribution. The persistence contract that keeps this
  possible is a Recorded decision above; do not regress it.
- **Self-update fast-follows** (all deferred, not started): `tauri-plugin-updater`
  wiring, a shared `core::updates::latest_stable(repo)` helper, a dismissible
  GUI "▲ vX.Y.Z" release-notification chip, the CLI `self-update` command + a
  throttled stderr hint, the GUI's on-demand "download & install the CLI"
  action (minisign-verified), and the tag → signed-bundle → GitHub-Release CI.
  This refactor's only obligation to them — a safe schema-migration path
  (version-skew guard + tested `user_version` steps) — is met (invariant 11,
  "Migration fixtures" above).
