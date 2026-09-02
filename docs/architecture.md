# Project Indexer — Architecture & Quality

Companion to `checklist.md`. That file tracks *features*; this one tracks the
*shape* of the system — the load-bearing decisions worth protecting, and the
quality work that keeps adding the next detector/platform/view cheap rather
than progressively more expensive.

Read `knowledgebase.md` first for how the pieces currently work. This file is
about direction, not mechanics.

## The system today

```
            ┌───────────────────────────────┐
            │            Svelte UI          │
            │  +page.svelte orchestrates:   │
            │  list · modals · TrackerBadges │
            │  · AppPicker · ErrorBanner     │
            │  · /project/[id] view          │
            │    (TrackerPanel)              │
            └───────────────┬───────────────┘
                            │  invoke()  (lib/api/* mirrors commands 1:1)
            ┌───────────────▼───────────────┐
            │        Tauri commands         │
            │  commands/projects.rs · system.rs · inspect.rs
            │  (CRUD + detection orchestration + launch;
            │   inspect.rs = read-only live detection)
            └──────┬─────────────────┬──────┘
                   │                 │
        ┌──────────▼──────┐   ┌──────▼───────────────┐
        │   ProjectStore  │   │   DetectorRunner     │  ← in App::manage
        │  (tauri-plugin- │   │   registry.rs picks  │
        │   store JSON)   │   │   the detector set   │
        └──────┬──────────┘   └──────┬───────────────┘
               │                     │  detect_project → Detection{outcomes}
        ┌──────▼──────┐     ┌────────┼────────┐
        │ migrations/ │     ▼        ▼        ▼
        │ (stamp v1)  │  Gitector  Unreal   (Unity…)
        └─────────────┘
```

What's real: everything drawn. What's *not* here yet: a service layer between
commands and the domain (the commands do orchestration directly), and any
platform abstraction for `system.rs` (it's `#[cfg(target_os)]` branches).

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
   once in `Project::new`, never regenerated (migrations included — there's a
   test that a record with no `id` fails to load rather than getting a fresh
   one). Anything built later (history, per-project settings, export/import)
   keys off the UUID, and a project survives its directory moving.
4. **Directory normalization is deliberate and case-sensitive.** `C:\Foo\` and
   `C:/Foo` collide; `C:\Foo` and `C:\foo` do not. This is a choice, not an
   oversight (`utils/normalize.rs`). See "Considered and declined".
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
   or `#[serde(default)]`; a shape change that can't be absorbed that way gets
   a `migrations::migrate` step and a `CURRENT_VERSION` bump. Enforced by
   `loads_a_record_missing_every_absorbable_field`.

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

The persist paths (`create_project`, `refresh_project_trackers`) still store
only `trackers`. The `/project/[id]` view closed the visibility gap a
different way: it calls `inspect_project` (read-only, live) and renders every
outcome — `● git · ○ unreal — not detected · ▲ unity — <error>` — so a
detector that fails is no longer indistinguishable from one that found
nothing.

## Recorded decisions

Choices that could plausibly have gone the other way, settled on purpose so
they don't drift into "that's just how it ended up". Each is guarded by a
test whose name is the sign that changing it is a real decision.

### Refresh is all-or-nothing

`refresh_project_trackers` — the explicit, user-triggered "re-scan this
project" — persists the detection result verbatim or not at all. If any
registered detector errors, the command fails and the stored trackers are
left untouched; it does **not** save the detectors that happened to succeed.

*Why:* detection results are stored as-is and drive the detail view. A
persisted tracker set silently missing whatever a failing detector would have
produced is a worse outcome than a visible "refresh failed" the user can
retry. `create_project` and the browse preview stay best-effort — there's no
prior good state to protect there.

*Revisit when:* there are enough independent detectors that losing an
unrelated tracker to one detector's transient failure is the common case. The
alternative is to persist the `Detected` outcomes and keep the `Failed` ones
as per-detector status (the `/project/[id]` view already shows this live from
`inspect_project`; this would carry it into the stored record too). That's a
deliberate change, not a detector quietly learning to tolerate partial state.

*Guarded by:* `into_result_discards_partial_trackers_on_any_error` (and the
`Detection::into_result` doc comment).

### Windows: launch `open_with` apps ourselves, not via the shell

`open_in_app` on Windows spawns a chosen executable with
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
- [ ] **Reconcile lockfiles.** Both `package-lock.json` and `pnpm-lock.yaml`
      are committed; there's no `packageManager` field. Pick one, delete the
      other, add `packageManager` to `package.json`. This actively bit GUI v1:
      `tauri.conf.json`'s `beforeDevCommand` is `pnpm dev`, so `npm run tauri
      dev` triggers a `pnpm install` that moves every npm-installed package to
      `node_modules/.ignored` and dirties `pnpm-lock.yaml`. Was worked around
      by hand each time. **Bumped from "cheap" to "do next".**
- [ ] **PI-004** — fix the NVIDIA-workaround comment wording in `lib.rs`.

### Next — before or alongside the Unity detector

- [x] **Per-detector status to the UI.** `inspect_project` returns one
      `DetectorResult { kind, status, tracker?, error? }` per detector and the
      `/project/[id]` view renders the full strip — "Git: detected · Unreal:
      failed · Unity: not detected" — not just successes. Read-only and live;
      the stored record is still successes-only (see "Refresh is
      all-or-nothing").
- [ ] **Extract detection orchestration** out of `commands/projects.rs` into a
      testable function/module seam. *Not* a full `services/` layer — the file
      is 300 lines, that's premature. Just make the create/refresh/preview
      logic callable without a live Tauri `State`.
- [ ] **Command-layer integration tests.** Now feasible: build a
      `DetectorRunner` + a temp `ProjectStore` and drive create → prefill →
      edit → favorite → delete → restore → refresh. The managed-state wiring
      made this slightly harder and more worth doing.

### Deferred — gated on a concrete trigger, not a date

- **Fast vs deep detection tiers.** When the first detector genuinely needs
  expensive work (Git contributors via revwalk, dependency parsing), split
  "cheap marker/metadata detection" from opt-in "deep inspection" — probably a
  separate command and a cache keyed on directory + HEAD. Until then, one tier.
- **Platform provider traits.** `InstalledAppProvider` / `AppLauncher` with
  Windows/Linux/macOS impls — do this *as* the macOS work, not before. The
  file that bites here is `commands/system.rs` (~760 lines), not `projects.rs`.
- **Migration fixtures.** `fixtures/v1/`, `fixtures/v2/`, `v1→current` tests —
  set up when `CURRENT_VERSION` first goes to 2. Nothing to test until then.
- **Structured detection logging** (`detector · duration · result`). Low value
  at 2–6 detectors; revisit if detection gets slow enough to debug.
- **Frontend page-state extraction** (`lib/stores/*`). `+page.svelte` is
  ~250 lines — watch it, don't pre-split.

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
