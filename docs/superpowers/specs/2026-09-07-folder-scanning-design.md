# Folder scanning — design

Point the app at a directory, tick the detectors that matter, choose how deep to
look, and import everything it finds in one pass. The single biggest adoption
gap: somebody with two hundred projects on disk currently adds them one at a
time, which is where they stop.

Settled design lives in `ROADMAP.md` → *Scanning a folder for projects*; this
document is the implementation contract. **Vocabulary, restated because it is
load-bearing:** "autorunner" and "looping detector" both mean *this* feature —
user-triggered, bounded, finite. Nothing here runs unprompted. Background
rescanning is a different, unagreed feature and must not merge into this one.

## Scope

In:

- A one-shot scan: pick a root, choose quick or deep with a user-chosen depth,
  tick detectors, walk, review, commit.
- Rescan on demand: the last scan's settings are remembered in `localStorage`
  and pre-fill the form, and already-tracked directories are filtered out, so a
  rescan reports only what is new.
- Name disambiguation, shared with `ensure_project`.

Out, deliberately:

- **A `scan_roots` table.** Specced and cut — see *Remembering the last scan*.
  No schema migration; this feature leaves `user_version` at 3.
- Rescan at startup, and any background or timer-driven traversal.
- Filesystem watching.
- An mtime cache that lets a rescan skip parts of the walk (considered; see
  *Decisions*).
- Progress events and mid-walk cancellation (considered; see *Decisions*).

## Architecture

Three units, each independently testable.

| Unit | Location | Responsibility |
|---|---|---|
| The walk | `core::domain::scan` | Pure traversal + pruning. Given a request and a detector runner, produce candidates. No database, no store. |
| Name disambiguation | `core::domain::naming` | `disambiguate` — one deterministic function, two callers. |
| Orchestration | `core::application::scan_service` | Walk, filter already-tracked, assign names, commit through `ProjectService`. |

The Tauri layer stays a thin adapter, as everywhere else. `indexer-core` gains
no dependency on Tauri and no new third-party crate — the walk is `std::fs`.

**There is no persistence unit**, which is the notable thing about this list:
the feature writes `Project` rows through machinery that already exists and
stores its own settings in the frontend. No new table, no new port, no
migration.

### Why not on `ProjectService`

`application/service.rs` is 614 lines and already the largest file in `core`;
bulk-import orchestration is a different concern from the per-project surface.
A separate service also keeps the walk testable without a database.

## The walk — `core::domain::scan`

```rust
pub enum ScanMode {
    Quick,
    Deep { depth: u32 },
}

pub struct ScanRequest {
    pub root: String,
    pub mode: ScanMode,
    pub detectors: Vec<String>,   // detector kinds, e.g. ["git"]
    pub include_ignored: bool,
}

pub struct Candidate {
    pub directory: String,
    pub suggested_name: String,
    pub matched_kinds: Vec<String>,
    /// Always `false` as the walk emits it — the walk has no store. Filled in
    /// by `ScanService` from `find_by_directory` before the report reaches the
    /// UI. Kept on `Candidate` rather than in a parallel list so the review
    /// table has one row type.
    pub already_tracked: bool,
}

pub struct ScanReport {
    pub candidates: Vec<Candidate>,
    pub visited: usize,
    pub stopped_early: bool,
}
```

`Quick` is exactly `Deep { depth: 1 }` inside the walker. It stays a distinct
variant because it is a distinct user choice, and spelling it as a magic number
in the UI would be worse.

**Traversal is an iterative BFS** over an explicit queue of `(path, depth)`,
never recursion — a pathological tree must not blow the stack. `walkdir` is not
worth a dependency once pruning is custom.

At each directory:

1. Run `DetectorRunner::inspect_kinds(path, Some(&ticked))`.
2. If anything matched, record a `Candidate` and **do not descend** — a
   repository inside a repository is vendored or a submodule, not a separate
   thing to track.
3. Otherwise, if `depth` is not yet exhausted, enqueue its subdirectories minus
   the prune list.

**Consequence of a filtered walk, named on purpose:** with only git ticked, an
Unreal-only directory is not a project boundary, so the walk descends *through*
it looking for repositories. That is the correct reading of "import my repos,
not my games" — selection decides what counts as a find, and therefore what
counts as a boundary.

**Pruning.** Two lists, and the distinction matters.

*Always pruned, not overridable:* `.git`, `.svn`, `.hg`. These are VCS metadata,
never projects, and `include_ignored` must not reach them — a git repository with
submodules keeps real repositories under `.git/modules`, and a walk that
descended there would detect and offer to import every submodule as a separate
project. That is a correctness rule, not a performance one.

*Pruned unless `include_ignored`:* `node_modules`, `target`, `.venv`, `venv`,
`build`, `dist`, and any other directory whose name starts with `.`.
Default-on because those trees are enormous and rarely projects; overridable
because "never" is not quite true and the person scanning knows their own disk
better than the prune list does.

**Bounding.** `MAX_DIRECTORIES = 50_000`. On reaching it the walk stops and sets
`stopped_early`, which the UI surfaces as "stopped after 50,000 directories —
narrow the root or reduce the depth". This is what makes a scan pointed at `C:\`
terminate instead of appearing to hang.

**Resilience.** An unreadable directory is logged and skipped. A permissions
error partway through `C:\Users` must not discard the 190 projects already
found. Symlinked directories are not followed, which is also the cycle
guarantee.

**Determinism.** Directory entries are sorted before enqueueing, and candidates
are returned sorted by directory. The same tree scanned twice yields the same
report in the same order — which is what makes name assignment reproducible.

## Detector selection — `inspect_kinds`

`DetectorRunner` gains one method:

```rust
pub fn inspect_kinds(&self, path: &Path, only: Option<&[&str]>) -> Detection
```

`None` means every registered detector. `Some(&[])` matches nothing. An unknown
kind matches nothing rather than erroring — remembered settings naming a
detector that has since been removed degrade to finding less, not to a failure.

The existing single-kind `inspect(path, Option<&str>)` **stays**: the re-detect
sweep needs exactly one detector, and both shapes are real. It is reimplemented
as a delegation to `inspect_kinds` so there is one traversal of the detector
list.

**The tick decides whether to register, not what gets recorded.** A directory
that survives the walk is registered through the normal `create` path, which
runs *every* installed detector and stores every tracker that matched. A
git+Unity directory lands once, carrying both. Recording only the ticked
detectors would leave permanently half-detected records.

## Names and collisions

```rust
pub fn disambiguate(preferred: &str, parent_dir: &str, taken: &HashSet<String>) -> String
```

`taken` holds case-insensitively normalised names: every active project's name
*plus* every name already assigned earlier in this same scan. The second clause
is what stops two rows of one report colliding with each other.

Resolution order:

1. `preferred` if free — `api`.
2. Else `parent/preferred` — `work/api`.
3. Else `parent/preferred (n)`, `n` from 2 upward — `work/api (2)`.

Step 3 is the terminating fallback. The function always returns a free name and
never loops.

The base name still comes from `suggest_project_name`, so a git remote continues
to beat the folder name.

**`ensure_project` uses the same function.** It currently hands an underived
name to `create` and returns `DuplicateName` the second time it meets an `api`
folder — a real bug on a path the observer CLI will drive, and the roadmap
requires this question be answered once for both callers.

In the review list a disambiguated row is flagged, so the rename is visible
rather than silent, and the name is editable. Editing re-checks against `taken`
live.

## Remembering the last scan

**Nothing is persisted to `projects.db`, and there is no `scan_roots` table.**
The last-used scan settings live in `localStorage` and pre-fill the form:

```ts
type StoredScanSettings = {
  root: string;
  mode: "quick" | "deep";
  depth: number;
  detectors: string[];
  includeIgnored: boolean;
};
```

Written on commit, not on scan — scanning is exploratory, importing is the
commitment, and a scan abandoned at the review screen leaves nothing behind.

**Rescanning is therefore not a stored entity, it is a pre-filled form.** Open
the scan modal and the root, mode, depth and ticks are already as you left them;
press Scan. `already_tracked` is set from `find_by_directory` and those rows are
hidden by default, so what you see is only what is new since last time.

*Why not a table.* A `scan_roots` table with ports, CRUD and `user_version` 4
was specced and cut. The path itself was never the value — anybody scanning
`~/projects` knows where `~/projects` is, so the folder picker is not the
friction. The value is the *settings*: a rescan that quietly ran depth 2 instead
of depth 4 gives a different answer with nothing on screen saying why. A
pre-filled form solves exactly that, and localStorage is already where this app
keeps UI state (`viewState.ts`) — which also keeps it out of `projects.db`, per
the devmon cross-app contract, where a list of one user's scan folders does not
belong.

*What would bring the table back.* Several distinct scan locations —
`~/projects`, `~/work`, `D:\clients` — each wanting its own remembered
settings and its own Rescan button. One set of pre-filled defaults cannot serve
three roots; a list can. That is the trigger, and until somebody has that disk
the table is speculative. Nothing in this design blocks adding it later: the
settings are already a single serialisable struct, so the migration is to move
where it is stored, not to invent its shape.

## Commit

```rust
pub struct ImportSelection { pub directory: String, pub name: String }

pub struct ImportReport {
    pub imported: Vec<Project>,
    pub skipped: usize,                        // already tracked
    pub failures: Vec<(String, String)>,       // directory, message
}
```

Commit calls `ProjectService::ensure_project` per row, honouring the reviewed
name rather than re-deriving one.

**Per-row failures are collected, never fatal.** A corrupt repository at row 47
must not cost rows 48–200.

**`refresh_trackers` is never called in this path.** It uses `into_result()`,
which is deliberately all-or-nothing: right for one project, catastrophic across
two hundred directories where one corrupt repository would discard the sweep.
Detection at commit goes through `create`'s best-effort pattern — `trackers()`
plus logged errors.

## Command surface

Two commands, plus one that exists so the UI need not hardcode detector names.

| Command | Shape |
|---|---|
| `scan_folder` | `ScanRequest -> Result<ScanReport, ProjectError>` |
| `import_scanned` | `(selections: Vec<ImportSelection>) -> Result<ImportReport, ProjectError>` |
| `list_detector_kinds` | `-> Vec<String>` |

`list_detector_kinds` returns the registered `Detector::kind()` values so the
tick-list is built from what the binary actually has. Without it, shipping the
Unity detector would mean editing a hardcoded array in a Svelte file — which
would break invariant 1, "a new detector is implement + register, zero frontend
code".

`scan_folder` is `async` so the walk runs on Tauri's pool and the window stays
responsive. It is blocking from the frontend's point of view: an indeterminate
spinner, no progress events, no cancellation. `MAX_DIRECTORIES` is what makes
that acceptable — worst-case duration is bounded.

## UI

A `ScanFolderModal` reached from the same place as "Add project", in three
steps within one modal:

1. **Configure** — `DirectoryField` for the root (reusing the existing Browse
   dialog), quick/deep radio, depth number input shown only for deep, a checkbox
   per registered detector kind, and the "include ignored directories" checkbox.
2. **Review** — the candidate table: checkbox, editable name, directory,
   matched tracker badges. Already-tracked rows are hidden behind a "show N
   already tracked" toggle. Select-all / none. Disambiguated names flagged.
   `stopped_early` renders as a banner.
3. **Result** — counts, and the failure list if non-empty.

The configure step opens pre-filled from `localStorage`, so a rescan is: open,
press Scan. There is no separate roots list and no rescan button — rescanning
*is* re-running a form that already knows your answers.

Nothing here needs per-detector frontend code: the checkbox list is built from
`list_detector_kinds`, and matched trackers render through the existing generic
badge component.

## Testing

**Walk** (`tempfile` trees): quick stops at one level; deep honours `n`; a
project directory is a boundary and its children are not visited; the prune list
applies and `include_ignored` defeats it; an unreadable directory is skipped
without ending the walk; `MAX_DIRECTORIES` sets `stopped_early`; a symlink loop
terminates; two runs over one tree produce identical reports.

**`disambiguate`** (pure): free name passes through; collision parent-qualifies;
double collision suffixes; two candidates in one scan do not collide with each
other; case-insensitive matching.

**Service** (in-memory SQLite): already-tracked directories are flagged and not
re-imported; a failing row does not stop the rest; a second scan of the same
tree after committing reports every candidate as already tracked.

**`ensure_project`**: two `api` directories under different parents both
register, named `api` and `work/api`.

**Frontend** (Vitest): settings round-trip through `localStorage` and a corrupt
or absent stored value falls back to defaults rather than throwing; the review
list's live name re-check.

No migration test, because there is no migration.

## Decisions

**Progress events and cancellation — declined for now.** The codebase has no
event channel, and adding the first one brings cancellation state nobody owns
and a new class of partial-state failure. `MAX_DIRECTORIES` bounds the worst
case instead; a quick scan is milliseconds. Revisit if a real scan is measured
as slow enough to need it.

**An mtime skip cache — declined.** Storing a mtime per visited directory would
let a rescan skip `read_dir` and detection on unchanged subtrees. It works, but
it means up to 50,000 stored rows and a table, to optimise something already
fast — the expensive part of a scan is detection, which already runs only on
directories that survive pruning. Doubly moot now that nothing is stored at all.

**Rescan cannot avoid the walk.** New projects are discoverable only by looking
at the disk; a rescan that read no filesystem would return exactly what the last
one did. This is why "remembering" was narrowed all the way down to a pre-filled
form: the path was never the friction — anybody scanning `~/projects` knows
where `~/projects` is — and the walk cannot be skipped, so the only thing left
worth keeping is the settings.

**Run-all-and-intersect — considered, not taken.** Selection could have been
applied to the result of an unfiltered `detect_project`, needing no new API,
since an unmatched detector is a couple of stats. Rejected because "cheap" is a
guarantee of the current first-party detectors only; a wasm plugin detector
makes running unticked detectors on every directory of a 50,000-directory walk
an unbounded cost. The filter is the shape that survives plugins.
