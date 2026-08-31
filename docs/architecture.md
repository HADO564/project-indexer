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
            └───────────────┬───────────────┘
                            │  invoke()  (lib/api/* mirrors commands 1:1)
            ┌───────────────▼───────────────┐
            │        Tauri commands         │
            │  commands/projects.rs  ·  system.rs
            │  (CRUD + detection orchestration + launch)
            └──────┬─────────────────┬──────┘
                   │                 │
        ┌──────────▼──────┐   ┌──────▼───────────────┐
        │   ProjectStore  │   │   DetectorRunner     │  ← in App::manage
        │  (tauri-plugin- │   │   registry.rs picks  │
        │   store JSON)   │   │   the detector set   │
        └──────┬──────────┘   └──────┬───────────────┘
               │                     │  detect_project → Detection{trackers,errors}
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

1. **A new detector is "implement + register" — nothing else.** A detector
   touches its own module, one line in `detectors/registry.rs`, a `Tracker`
   variant, and its `*Info` model. It must not require changes to the runner,
   the command layer, `DetectorError` (the `Other` variant is the escape
   hatch), or the frontend.
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

The runner already returns `Detection { trackers, errors }`, but the *meaning*
of the states isn't written down anywhere, so the next detector author will
guess. The taxonomy:

| State | How it's represented | Example |
|---|---|---|
| **Not mine** | absent from both lists | a plain directory, to `Gitector` |
| **Detected** | entry in `trackers` | a git repo → `Tracker::Git(info)` |
| **Detected, partial** | entry in `trackers` with `None` fields | git repo with no remote (`repo_url: None`) — a normal state, not an error |
| **Detector failed** | entry in `errors` | libgit2 can't read a corrupt repo; `.uproject` is malformed JSON |
| **Path unusable** | `errors`, or refused earlier by `check_directory_health` | directory deleted mid-operation |

The load-bearing distinction: **"malformed `.uproject`" and "not an Unreal
project" are not the same outcome** and must never collapse into one. A
detector returns `Ok(None)` for "not mine" and `Err` for "mine but broken".

Gap: the frontend consumes `trackers` and drops `errors`. A user whose Unreal
detection fails silently sees "no Unreal tracker", indistinguishable from a
non-Unreal project. Per-detector status needs to reach the UI — see backlog.

## Quality backlog

Curated and reordered from a broader architectural review. Prioritized by
*payoff now*, not by how interesting the problem is.

### Now — cheap, and makes the next detector cheaper

- [ ] **Explicit tracker/detector identity.** Add `Tracker::kind(&self) -> &'static str`
      and a matching `Detector::kind()`. The frontend currently infers the
      kind from the serde shape (`Object.keys(tracker)[0]` in `trackers.ts`) —
      clever, but it couples UI semantics to serialization. Keep the generic
      field rendering; just stop deriving *identity* from JSON structure.
- [ ] **Write down the detection semantics** (the table above) as a doc
      comment on `Detection` / the `Detector` trait, and add a test that a
      malformed descriptor is an `Err`, not `Ok(None)`.
- [ ] **Detector fixtures.** `src-tauri/tests/fixtures/{git,unreal}/…`
      (clean, dirty, unborn, detached; minimal, plugins, source-control)
      instead of building every scenario by hand in each test. Pays for itself
      at Unity/Blender.
- [ ] **Lock the refresh decision with a test.** `refresh_project_trackers` is
      deliberately all-or-nothing (`Detection::into_result`). Add a test so a
      future "just persist what succeeded" change is a conscious one.
- [ ] **Reconcile lockfiles.** Both `package-lock.json` and `pnpm-lock.yaml`
      are committed; there's no `packageManager` field. Pick one, delete the
      other, add `packageManager` to `package.json`. (Recent work used `npm`;
      the Linux pass used `pnpm` — that ambiguity is the whole problem.)
- [ ] **PI-004** — fix the NVIDIA-workaround comment wording in `lib.rs`.

### Next — before or alongside the Unity detector

- [ ] **Per-detector status to the UI.** Surface `Detection.errors` (with the
      detector `kind`) so the detail view can show "Git: detected · Unreal:
      failed · Unity: not detected" rather than only successes.
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
  file that bites here is `commands/system.rs` (730 lines), not `projects.rs`.
- **Migration fixtures.** `fixtures/v1/`, `fixtures/v2/`, `v1→current` tests —
  set up when `CURRENT_VERSION` first goes to 2. Nothing to test until then.
- **Structured detection logging** (`detector · duration · result`). Low value
  at 2–6 detectors; revisit if detection gets slow enough to debug.
- **Frontend page-state extraction** (`lib/stores/*`). `+page.svelte` is 257
  lines — watch it, don't pre-split.

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
