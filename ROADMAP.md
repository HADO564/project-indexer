# Roadmap

Where Project Indexer is going, why, and — just as usefully — what has been
considered and ruled out. Nothing here carries a date. Items move when the work
that unblocks them lands, not when a quarter ends.

For fine-grained feature status see [`docs/checklist.md`](docs/checklist.md); for
the non-feature quality backlog see
[`docs/architecture.md`](docs/architecture.md).

## Where things stand

**v0.1.1** is the current release. The app tracks projects, detects git and
Unreal Engine trackers, opens projects in your installed applications, and runs
in the background from the system tray. The Rust backend has been restructured so
that all logic lives in `indexer-core`, a library crate the compiler forbids from
importing Tauri — which is what makes everything in the next section possible
without touching the backend.

Windows and Linux are both built and tested in CI. macOS builds in the release
workflow but is not yet functionally complete (see below).

## Next — the `indexer` command-line tool

The single largest planned piece, and the one the last refactor was for. It has
two halves, and only the first is designed in detail.

### Observing

`indexer git init` runs the real `git init`, untouched, propagates its exit code,
and *notices* what happened — then records the project through the same
`ProjectService` the GUI uses. It never reimplements the tools it wraps. Because
both frontends open the same SQLite database, installing the CLI later connects
it to the GUI with no pairing and no IPC.

The backend seams already exist: `ensure_project` and `find_by_directory` have no
GUI caller and were added purely for this, and `projects.directory_normalized` is
indexed so directory lookup is not a table scan.

What is *not* yet decided — deliberately — includes which commands are recognised
first, how the project directory is derived from arguments and working directory
per recognizer, what happens when a directory's inferred name collides with an
existing project, and whether plain subcommands (`indexer list`, `indexer open`)
ship alongside the observer or after it.

The full briefing, including the open questions, is in
[`docs/handoffs/2026-09-04-observer-cli.md`](docs/handoffs/2026-09-04-observer-cli.md).

### Plain subcommands

The unglamorous half: `indexer list`, `show`, `add`, `open`, `untrack`. Each maps
almost one-to-one onto a `ProjectService` method that already exists, so these
are cheap — the work is argument parsing and output formatting, not behaviour.

One thing is genuinely undecided: whether they ship with the observer or after
it, since they are separable.

`indexer list` printing real rows from the shared database is the suggested first
vertical slice for the whole initiative. It proves the premise — same database,
no backend changes — in about twenty lines.

#### The `--json` contract

Human-readable tables are the default. `--json` is a different thing: the moment
it exists, other people's scripts depend on its shape, so it is settled here
rather than by accident at the keyboard.

- **A versioned envelope.** Every response is `{"schema": 1, "data": …}`. One
  integer, bumped only for a change that breaks a reader.
- **Additive-only within a version.** New fields may appear at any time and
  consumers must ignore ones they do not recognise. Nothing is removed or
  retyped without bumping `schema`.
- **An unknown tracker serialises; it does not fail.** `Tracker` is a closed enum
  today (`Git`, `Unreal`) and every new project type widens it. A reader written
  against `schema: 1` has to keep working when a kind it has never heard of turns
  up — so `kind` is a string and the payload is a map, which is exactly the shape
  the UI's generic renderer already consumes. This matters more once trackers can
  come from [plugins](#plugins).
- **stdout is data, stderr is prose.** Under `--json`, stdout carries the
  document and nothing else, so `indexer list --json | jq` needs no filtering.
  Errors go to stderr and the exit code — never into stdout as an error object.

The cost is one wrapper struct and a documented rule. The cost of skipping it is
a breaking change the first time somebody adds a project type.

## More project types

Detection is designed so a new tracker costs no frontend code: implement the
`Detector` trait, register it in one place, add the type, and the UI renders it
generically. See [`CONTRIBUTING.md`](CONTRIBUTING.md#adding-a-detector).

- **Unity** — the next detector, and the one the generic path was built for.
- **Blender** — same shape.

Version-control systems beyond git are deliberately not first-party work — they
belong to [Plugins](#plugins). This section is about what *kind of project* a
directory holds.

Placeholder tracker variants without a detector behind them were removed once and
will not come back — a type is added together with the code that produces it.

## Deeper git support

`GitInfo` currently reports the branch, dirty state, detached HEAD, the branch
list, the current commit, and the remote in both raw and browser-openable form.
That answers "where is this project" but not the question you actually have when
you come back to something after a month: **what did I leave unfinished here?**

Candidates, roughly in order of value per unit of cost:

- **Ahead / behind upstream** — "3 ahead, 1 behind `origin/main`". Probably the
  single most useful thing missing. Cheap: it is a ref comparison, not a history
  walk.
- **The last commit itself** — author, date, subject. The commit object is
  already being read for `commit_hash`; only the fields are missing.
- **Stashes** — a count, at least. Work parked and forgotten is exactly what this
  app should surface.
- **Uncommitted work as a number**, not just the `dirty` boolean — "7 modified,
  2 untracked" tells you whether it is a stray file or a half-finished feature.
- **Submodules, worktrees, tags, LFS** — lower value individually, but all cheap
  reads against refs and config.
- **Contributors** — already deferred, and correctly: it needs a full `revwalk`.

Everything above the last item is a refs-and-config read, which is why it can
land without waiting for anything. Contributors is the one that forces the
fast-versus-deep detection split described under [Deferred](#deferred--gated-on-a-trigger-not-a-date),
and it should stay behind it.

## Scanning a folder for projects

Point the app at `~/code` and let it find everything inside, instead of adding
projects one directory at a time. This is the single biggest usability gap for
anyone adopting the app with an existing disk full of work — and adoption is
exactly when the manual path is most painful.

The mechanics that need deciding:

- **Where to stop.** A depth limit, and pruning of directories that are never
  projects but are always enormous — `node_modules`, `target`, `.venv`, `build`.
  Also: stop descending once a directory *is* a project. A repository inside a
  repository is usually vendored or a submodule, not a separate thing to track.
- **Review before committing.** A scan that silently registers two hundred
  entries is hostile. Find, present, let the user deselect, then add. Registering
  a project is a durable act; a bulk one should be a deliberate one.
- **Name collisions.** Projects must have unique names, so scanning `~/code` and
  `~/work` when both contain an `api` folder hits this on the first run. This is
  the *same* unresolved question the CLI's `ensure_project` has — disambiguate,
  prompt, or qualify by parent — and solving it once serves both. Neither should
  invent its own answer.
- **Rescanning.** A remembered root that can be re-scanned to pick up what is new
  since last time, rather than a one-shot import. Watching it live is a further
  step and probably not the first one.

Two seams already exist for this. `find_by_directory` plus the indexed
`directory_normalized` column make "do we already track this?" cheap enough to
ask once per candidate, and detection is already resilient — one detector failing
on one directory does not abort a sweep.

The performance shape is worth getting right early: walking is I/O bound and
cheap, running full detection on every directory is not. Detection should be
gated behind a cheap marker test — does a `.git` or `.uproject` even exist here —
which is the fast-versus-deep split again, arriving from a second direction.

## Plugins

The extension mechanism, and the answer to "who adds the next twenty project
types?" — which should not be us. Two shapes, and the second is much harder than
the first.

### Shape one — a UI plugin over the existing backend

A panel or a view written against data the backend already produces. Nothing new
is detected; something already detected is shown *better*.

The generic renderer sets a decent floor here already. A tracker the UI has never
heard of renders today: fields are typed by inference (`https://` becomes a link,
`git@` becomes copyable text rather than a broken link, `*_root` and `*_path` get
open and reveal buttons, arrays become chips), and `trackerColor(kind)` assigns a
contrast-safe hue from a hash of the name. A UI plugin is for where that floor is
not enough — a commit graph instead of a field list, a scene hierarchy, a
dependency tree — or for views that are not per-tracker at all: a dashboard, a
different grouping of the project list.

### Shape two — a UI plugin and a Rust plugin shipped in tandem

For anything the backend cannot already see. A Jujutsu plugin is the clean
example: a `Detector` that reads `.jj`, plus a panel that renders what it found.
The halves ship together because neither is much use alone.

The backend half has a natural seam already — `Detector` is a two-method trait
and `default_detectors()` is the registry it plugs into — but a seam is not a
plugin API. The open question is what *installing* a Rust plugin means, and the
honest answer is that it is a distribution problem before it is a technical one.
See [Trust](#trust--settle-this-before-the-api).

### Version control beyond git is a plugin, not a roadmap item

Git is first-party and stays that way. **Mercurial, Subversion, Jujutsu,
Perforce, Fossil** are deliberately *not* first-party work. Shipping and
maintaining detectors for version-control systems most users do not have is a
poor use of the project's time, and the people who need them are far better
placed to write them.

Two facts make that a reasonable thing to ask of a contributor rather than a
brush-off. Detectors are independent and unordered by design, so a Jujutsu
repository colocated with a git one correctly reports *both* — there is no
precedence rule to invent. And the UI needs no changes at all, because an
unrecognised tracker already renders from its field names and shapes. A
backend-only plugin is a complete, useful contribution on its own.

**Perforce is the likely first one**, because a hook already exists: the Unreal
detector reads the configured source-control provider out of
`SourceControlSettings.ini`, so Unreal projects frequently already tell us they
are on Perforce.

### Trust — settle this before the API

Both shapes run somebody else's code, and they have completely different
containment stories. This is the part to get right before any of it ships.

**A UI plugin can be contained, if the work is done.** The frontend is a webview,
and Tauri exposes `invoke` to everything running inside it. `invoke` reaches
`delete_project_directory` and the application launcher. There is no per-script
permission within a webview — a plugin has exactly the powers the app has. Three
layers, in increasing order of effort:

1. **A content security policy** stops a plugin fetching more code or phoning
   home. It does not stop it calling `invoke`. Necessary but not sufficient —
   and now in place, in `svelte.config.js` and `tauri.conf.json`.
2. **Capability scoping.** Tauri 2 can restrict which commands a given window may
   call. Running plugins in their own webview with a reduced capability set is
   the first real boundary.
3. **No `invoke` for plugins at all.** Plugins get a narrow host API —
   `host.tracker()`, `host.reveal(path)` — and never touch the command bridge.
   The most work, and the only version that is actually safe rather than merely
   inconvenient to abuse.

**A Rust plugin cannot be contained.** Native code loaded into the process has
the user's full privileges: no CSP, no capability list, no sandbox, and nothing
in Tauri changes that. That has one concrete consequence, recorded under
[Considered and declined](#considered-and-declined): a runtime loader that picks
up `.so`/`.dll` files from a folder should not be built. The realistic model is
source distribution — a plugin is a crate, the user adds it and rebuilds, and
trust is established the way it is for every other dependency. Slower to install,
and the only version that does not hand every plugin author arbitrary code
execution on every user's machine.

## Platform completeness

- **macOS.** Installed-application discovery returns an empty list, so the "open
  with" picker has nothing to offer and launching falls through to the generic
  opener. This is also the natural moment to put `list_installed_apps` behind a
  trait: a third implementation is what makes that seam pay for itself, and until
  then a plain function is honest.
- **Global shortcut.** The plugin is registered but no shortcut is bound to
  anything. Deciding what it should *do* is the open part.

## Updates and distribution

Designed but not built. All of it is fast-follow work on top of the release
pipeline that already exists.

- `tauri-plugin-updater` wiring, with a shared `core::updates::latest_stable`
  helper so the GUI and CLI agree on what "latest" means.
- A dismissible in-app release notification, rather than an interrupting dialog.
- `indexer self-update` for the CLI, plus a throttled hint on stderr.
- An on-demand "download and install the CLI" action in the GUI, minisign
  verified, so the CLI need not be bundled with the installer.
- Tag → signed bundle → GitHub Release in CI.

The obligation this places on the current code — a safe schema-migration path, so
a newer binary opening an older database is routine rather than dangerous — is
already met: migrations are numbered `user_version` steps and `open` refuses a
database written by a newer binary.

## Deferred — gated on a trigger, not a date

These are not "someday". Each has a specific condition that should start it.

- **Fast vs deep detection tiers.** Split cheap marker detection from opt-in deep
  inspection, with a cache keyed on directory and HEAD. Trigger: the first
  detector that genuinely needs expensive work — git contributors, or dependency
  parsing.
- **Migration fixtures.** A `fixtures/` scaffold that seeds a database at
  `user_version = N` and asserts the result of each step. Trigger:
  `CURRENT_SCHEMA_VERSION` going to 2.
- **Structured detection logging.** Low value at two to six detectors. Trigger:
  detection getting slow enough to need debugging.
- **Frontend page-state extraction.** `+page.svelte` is around 250 lines. Watch
  it; don't pre-split it.

## Considered and declined

Recorded so they are not re-proposed without new information. The full reasoning
for each is in [`docs/architecture.md`](docs/architecture.md).

- **Platform-aware case folding for directory identity.** It would prevent
  registering `C:\Foo` and `C:\foo` as two projects on Windows — a rare,
  self-correcting problem, since the user can see both. Not worth forking a
  simple, well-tested normalisation function across operating systems.
- **Detector metadata, priority, or short-circuiting.** Detectors are independent
  by design, so there is no contention to arbitrate. Revisit only if two
  detectors genuinely need to coordinate.
- **Encrypting `projects.db`.** It holds paths and labels, not credentials, and
  being readable by other local tools is a deliberate contract, not an oversight.
- **A runtime loader for native plugins.** Dropping a `.so` or `.dll` into a
  plugins folder and having the app load it at startup. Native code in-process
  has the user's full privileges and cannot be sandboxed, so this would hand
  every plugin author arbitrary code execution on every user's machine in
  exchange for a nicer install step. Rust plugins are distributed as source and
  compiled in; see [Plugins → Trust](#trust--settle-this-before-the-api).

## Related work

**devmon** is a separate planned application — an activity tracker that attaches
`projects.db` read-only to attribute work to projects. It shapes nothing Project
Indexer must do, but it is why the `ProjectReader` port and the `meta` table
exist. The cross-app contract is a recorded decision; don't regress it.
