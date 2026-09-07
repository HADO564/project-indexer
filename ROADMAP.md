# Roadmap

Where Project Indexer is going, why, and — just as usefully — what has been
considered and ruled out. Nothing here carries a date. Items move when the work
that unblocks them lands, not when a quarter ends.

For fine-grained feature status see [`docs/checklist.md`](docs/checklist.md); for
the non-feature quality backlog see
[`docs/architecture.md`](docs/architecture.md).

## Where things stand

**v0.2.0** is the current release, and the first under the new licence. The
app tracks projects, detects git and
Unreal Engine trackers, opens projects in your installed applications, and runs
in the background from the system tray. A project carries a colour, an icon and
any number of user-defined key/value properties, belongs to at most one group,
and is reached through a sidebar — All, Favourites, each group, Ungrouped, Bin —
rather than one flat column. Search covers name, path, tags and property values,
with `client: acme` looking inside one property and a bare `client:` finding
every project that has it.

The Rust backend has been restructured so that all logic lives in
`indexer-core`, a library crate the compiler forbids from importing Tauri —
which is what makes everything in the next section possible without touching the
backend. Storage is SQLite behind numbered `user_version` migrations, currently
at version 3.

Windows and Linux are both built and tested in CI. macOS builds in the release
workflow but is not yet functionally complete (see below).

## Licensing

Released under the [Functional Source License](LICENSE) (`FSL-1.1-ALv2`), with
**v0.2.0 the first release to carry it**. Use is free for everyone, companies
included, and every version converts to Apache 2.0 on its second anniversary.
The one prohibited use is shipping a competing commercial substitute.

**v0.1.0 and v0.1.1 stay MIT**, as does every commit made before the relicense:
an MIT grant cannot be withdrawn, and those commits sit in the public history
with the old `LICENSE` beside them. That is a permanent fork point no later
decision can close, and it is the reason the restriction only starts protecting
anything from v0.2.0 on.

Contributions are gated on the [CLA](CLA.md) — one comment on a first pull
request — because the two-year conversion cannot be honoured for code the
project has no right to relicense.

### Plugins and the licence

Settled, because [Plugins](#plugins) is a committed direction rather than a
speculative one, and settling it after an API is published costs a migration.

A plugin is its author's own work. A theme is a file of token values; a
frontend plugin is code against a published host API; a backend plugin is a
crate that depends on `indexer-core` and implements a trait. None of those are
derivatives of the Software, and FSL has no copyleft clause reaching them — its
restriction is on Competing Use of the Software, not on what licence a
dependent work carries. Plugin authors pick their own licence, and a plugin
living in its own repository is not a contribution, so no CLA applies to it.

The collision was elsewhere, and it was real. Because there is deliberately no
runtime loader, a backend plugin only runs in a build of the app that includes
it — and distributing that build is distributing a modified copy of the
Software, which the bare licence permits only for a non-Competing purpose. A
build of Project Indexer with an extra detector substitutes for Project
Indexer, so a plugin author could publish their crate but nothing anyone could
install. "Somebody else adds the next twenty project types" does not survive
that.

The **Additional Permission** in [`LICENSE`](LICENSE) resolves it: a build
differing from a release only by added plugins may be distributed, in source or
binary, provided it is not passed off as official, carries the licence, and
says what it was built from. Source distribution stays the norm — this is a
developer tool and "clone, add the crate, build" is a low bar here — but the
permission means a plugin author is not forced into it.

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

**Ship the re-detect sweep first.** Detection results are persisted, so adding a
detector does nothing for projects already registered — a git+Unity directory
added today stays git-only after a Unity detector ships, with nothing to say it
is incomplete. Shipping Unity without the sweep ships a silent gap in every
existing install. The sweep runs one new detector across existing projects and
is small; `architecture.md` → *Detection semantics* has the shape and the two
traps.

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

- ~~**Where to stop.**~~ **Settled 2026-09-07: two scans, and a depth the user
  picks.** A **quick scan** looks only one level below the chosen directory —
  the `~/code` case, where every child is a project and nothing deeper needs
  visiting. A **deep scan** descends to *n* levels, with *n* chosen by the user
  rather than hardcoded, because the right depth is a property of how somebody
  organises their disk and no default is right for everyone.

  Directories that are conventionally gitignored — `node_modules`, `target`,
  `.venv`, `build`, `dist` — are skipped by default, with a checkbox to include
  them. Default-on because those trees are enormous and never projects;
  overridable because "never" is not quite true and the person scanning knows
  their own disk better than the pruning list does.

  Still holds regardless of mode: stop descending once a directory *is* a
  project. A repository inside a repository is usually vendored or a submodule,
  not a separate thing to track.
- ~~**Which detectors run.**~~ **Settled 2026-09-07: the user checks the ones to
  scan for, and the check decides *whether to register*, not what gets
  recorded.** Pointing the scanner at `~/projects` with only git ticked imports
  the code repositories and leaves the Unity and Godot ones alone — the intent
  is "import my repos, not my games", which is a selection criterion rather
  than a performance knob.

  Once a directory is being kept, **every** installed detector runs against it.
  A directory that is both git and Unity registers **once**, carrying both
  trackers — `Project.trackers` is a `Vec<Tracker>` and `create` already stores
  every match, so this needs no new machinery. Recording only the ticked
  detectors would instead leave permanently half-detected records that fix
  themselves only if somebody remembers to refresh them, and it would buy
  nothing: a detector that does not match costs a couple of `stat` calls and no
  allocation.

  **This is a scan, not a daemon.** It is user-triggered, bounded and finite.
  The continuous-loop version is the open question under *Rescanning* below,
  and it is a different feature; do not let the two merge under one name.
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
- **An autorunner — open, and not yet agreed.** The idea: run the detectors on a
  loop, traversing for projects continuously rather than when asked. The appeal
  is obvious; three things have to be answered before it is worth building.

  What it does on a find. Auto-registering contradicts the review step directly
  below — a bulk registration is meant to be a deliberate act — so realistically
  it queues finds and badges them for review, which is *rescan on a timer plus a
  notification*, a much smaller feature wearing a bigger name.

  What it costs. A loop walking the filesystem is the classic background-indexer
  complaint, and it is paid on battery. Filesystem watching (`notify`) is far
  cheaper than polling, but it is a background task with its own failure modes —
  the same objection that settled theme reloading as restart-to-apply, at
  considerably larger scale.

  Where it looks. It needs remembered roots; scanning the disk is not an option.
  Which means it is gated on rescanning above, not a parallel feature.

  The cheap version worth building first: rescan remembered roots on demand and
  optionally at startup, surfacing "12 new projects found" as a review queue.
  That is most of the value with none of the loop, and it is the honest thing to
  try before deciding continuous scanning is needed.

Two seams already exist for this. `find_by_directory` plus the indexed
`directory_normalized` column make "do we already track this?" cheap enough to
ask once per candidate, and detection is already resilient — one detector failing
on one directory does not abort a sweep.

Four things the implementation will meet, all cheap to know in advance:

- **`ensure_project(directory)` is the scanner's entry point**, not `create`.
  `create` calls `check_for_duplicate_name_or_dir` and returns
  `DuplicateDirectory` for a path already tracked, so re-scanning a folder — or
  scanning one containing projects added by hand — would fail per directory.
  `ensure_project` is get-or-create, is indexed, and was built for the observer
  CLI with no GUI caller yet.
- **Detector selection needs a set, and no such filter exists.** `create` runs
  everything via `detect_project`; the only filtering available is
  `inspect(path, only: Option<&str>)`, a *single* kind, for per-tracker
  re-detect. Selection wants `Option<&[&str]>` or equivalent. Note the
  single-kind form stays necessary for the re-detect sweep — both shapes are
  real.
- **Never `refresh_trackers` in the bulk path.** It uses `into_result()`, which
  is deliberately all-or-nothing: one failing detector discards everything.
  Right for a user pressing refresh on one project, wrong across two hundred
  directories where a single corrupt repository would abort the sweep. Use
  `create`'s best-effort pattern — `trackers()` plus logged errors.
- **A scan is where name collisions actually bite**, not a theoretical concern:
  `~/code/api` and `~/work/api` collide on the first run. Same unresolved
  question as `ensure_project`'s, and it should be answered once for both.

The performance shape is worth getting right early: walking is I/O bound and
cheap, running full detection on every directory is not. Detection should be
gated behind a cheap marker test — does a `.git` or `.uproject` even exist here —
which is the fast-versus-deep split again, arriving from a second direction.

## Project linking

Connect one project to another the way Obsidian connects notes — an explicit,
navigable edge between two entries, and a graph view over the whole set. The
project list answers "what do I have"; links answer "what did I build this
*out of*", which is the question that turns a list into a knowledgebase.

The value shows up where a flat list is weakest: a tool and the game that uses
it, a fork and its upstream, a client engagement and the three repositories it
spans. Groups already give one exclusive band per project — links are the
non-exclusive, many-to-many relation that groups deliberately are not.

The graph is drawn with **Svelte Flow** (`@xyflow/svelte`), the SvelteKit
counterpart to React Flow from the same authors. It has to be a bundled
dependency rather than anything CDN-loaded — the content security policy
forbids fetching code at runtime, and that is a property worth keeping.

What needs deciding:

- **Are links typed, and are they directed?** "depends on" and "forked from"
  have a direction that "related to" does not. Untyped and undirected is the
  cheap start; adding a type later is additive, adding direction later is not.
- **Where links live.** A many-to-many edge does not fit the JSON project blob
  the way a scalar field does, so this is a real table and a schema step. It
  should land on the same migration machinery the groups work builds, not
  invent a second one.
- **Whether the graph is a view or the view.** A focused neighbourhood around
  one project is a panel on the project page. A whole-set canvas is a route of
  its own. They are different features wearing one name.
- **What a link does when a project is deleted.** Soft-deleted projects stay in
  the bin, so their edges should presumably persist and grey out rather than
  vanish — otherwise restoring a project silently loses its connections.

## Plugins

The extension mechanism, and the answer to "who adds the next twenty project
types?" — which should not be us. Two kinds, split by what a plugin is *for*.

### UI plugins — how the app looks

Theme and layout. Frontend only, by definition: nothing new is detected and
nothing new is computed, the same data is just presented differently.

**A UI plugin is data, not code** — a config file, the way a dotfile is. This is
the whole design, and it is worth stating as a rule rather than a default,
because everything good about this category follows from it. A file of values
cannot call `invoke`, cannot reach the filesystem, and cannot be made to; there
is no sandbox to build because there is nothing to sandbox. That is what makes
themes shippable long before the containment work in [Trust](#trust--it-sorts-by-kind-not-evenly)
is done.

The machinery mostly exists. `src/app.css` already declares the whole visual
system as tokens in a `@theme` block — `--color-void`, `--color-panel`,
`--color-phos`, `--color-accent`, the state hues, the eight `--color-swatch-*`
entries every project and group colour resolves through, `--font-display` and
`--font-mono`, and the single 2px `--radius-*`. **A theme is an override set for
those tokens**, read from the app's config directory next to `projects.db` and
applied as custom properties.

Layout is the harder half and the place the rule will come under pressure.
Declarative rearrangement — named regions plus an ordering, a section toggled off
— fits in a config file. Anything that wants to *compute* what to render does not,
and the moment a UI plugin can execute it has stopped being a UI plugin and
become a frontend utility plugin wearing a theme's clothes, inheriting the entire
trust problem it had avoided. Hold the line at declarative.

Two things to get right when this is built:

- **A malformed theme must not be able to stop the app starting.** Fall back to
  the built-in values and surface the error in the UI. `PI-005` is the standing
  reminder that a failure during startup is the one kind users cannot work around.
- **Validate values, do not interpolate strings.** "No code" removes code
  execution, not injection: a token value passed unchecked into CSS could carry
  `;` and further declarations. Parse each token to its expected type — colour,
  length, font stack — and reject the rest. Cheap, and it keeps the "no trust
  problem" claim true.

### Utility plugins — what the app can do

Capability rather than presentation: a richer git experience, a new project type,
a new integration. Three deployment shapes, with quite different trust profiles.

- **Backend only.** A `Detector` that reads something the app cannot see today.
  Already a complete, useful contribution on its own, because the generic
  renderer displays an unfamiliar tracker without any UI work — fields are typed
  by inference (`https://` becomes a link, `git@` becomes copyable text rather
  than a broken link, `*_root` and `*_path` get open and reveal buttons, arrays
  become chips) and `trackerColor(kind)` assigns a contrast-safe hue from a hash
  of the name.
- **Frontend only.** A feature built on data the backend already returns — a
  commit graph instead of a field list, a dependency tree, per-kind actions. This
  is the shape that runs third-party code in the webview, and the only one gated
  on containment.
- **Both.** A detector plus the panel that renders what it found. A Jujutsu
  plugin is the clean example; the halves ship together because neither is much
  use alone. Inherits the frontend half's trust requirements.

The backend seam exists: `Detector` is a two-method trait and
`default_detectors()` is the single registration point. A seam is not a plugin
API, though, and what "installing" a Rust plugin means is a distribution problem
before it is a technical one — see [Trust](#trust--it-sorts-by-kind-not-evenly).

**How a third-party detector is distributed: as WebAssembly, compiled by CI at
publish time.** An author writes Rust against a published API crate and submits
source; the registry compiles it to `wasm32-wasip1` and hosts the artifact; the
app downloads it and instantiates it with Wasmtime. The user installs a file and
needs no toolchain. This is the model [Zed](https://zed.dev/blog/zed-decoded-extensions)
and [Lapce](https://lapce.dev) use.

The alternative — ship source and recompile on the user's machine — was
considered and rejected. It demands a full Rust toolchain and a multi-gigabyte
build tree per user, cannot overwrite a running executable, is wiped by every
official update, and breaks the macOS app signature on rebuild, which would
permanently foreclose ever signing a release.

WASI is what makes this fit rather than merely work. A module reaches exactly
the directories preopened for it, read-only if `dir_perms` says so — the same
shape as the detector contract itself (`docs/architecture.md` → *Detector, or
backend feature?*): one directory, read-only, no side effects. Two
prerequisites, both already on the list: `Tracker` stops being a closed enum,
and `Detector::kind()` returns `&str` rather than `&'static str`. Neither the
runner nor first-party detectors change — the git detector needs libgit2 and
stays native, so the end state is a hybrid behind one trait.

### Version control beyond git is a utility plugin, not a roadmap item

Git is first-party and stays that way. **Mercurial, Subversion, Jujutsu,
Perforce, Fossil** are deliberately *not* first-party work. Shipping and
maintaining detectors for version-control systems most users do not have is a
poor use of the project's time, and the people who need them are far better
placed to write them.

Two facts make that a reasonable thing to ask of a contributor rather than a
brush-off. Detectors are independent and unordered by design, so a Jujutsu
repository colocated with a git one correctly reports *both* — there is no
precedence rule to invent. And these are backend-only utility plugins, the shape
that needs no UI work and no containment design at all.

**Perforce is the likely first one**, because a hook already exists: the Unreal
detector reads the configured source-control provider out of
`SourceControlSettings.ini`, so Unreal projects frequently already tell us they
are on Perforce.

### Trust — it sorts by kind, not evenly

The useful consequence of splitting plugins by purpose is that only one of the
four categories is actually hard.

| Kind | What it is | What it can reach | Gated on |
|------|-----------|-------------------|----------|
| UI — theme, layout | a config file | nothing; it is data | nothing |
| Utility, backend | a crate, compiled in | full user privileges | dependency trust |
| Utility, frontend | JavaScript in the webview | everything `invoke` reaches | containment, below |
| Utility, both | both halves | as above | as above |

**Frontend code is the hard case.** The frontend is a webview, and Tauri exposes
`invoke` to everything inside it; `invoke` reaches `delete_project_directory` and
the application launcher. There is no per-script permission within a webview, so
a plugin has exactly the powers the app has. Three layers, in increasing order of
effort:

1. **A content security policy** stops a plugin fetching more code or phoning
   home. It does not stop it calling `invoke`. Necessary but not sufficient —
   and now in place, in `svelte.config.js` and `tauri.conf.json`.
2. **Capability scoping.** Tauri 2 restricts which commands a window may call,
   and `src-tauri/capabilities/default.json` is already scoped to
   `"windows": ["main"]`. Running plugins in their own webview with a smaller
   set is the first boundary that is enforced rather than agreed to.
3. **No `invoke` for plugins at all.** Plugins get a narrow host API and never
   touch the command bridge. The most work, and the only version that is safe
   rather than merely awkward to abuse.

Layers 2 and 3 are not interchangeable, and the choice between them is decided by
something else: whether a frontend plugin is a module the app imports or a
document in its own webview. A module shares the app's realm and can reach the
IPC bridge on its own, which makes layer 3 a convention rather than a boundary
and rules layer 2 out. That decision is not reversible once an API is published.

**Native code cannot be contained.** A Rust plugin loaded into the process has
the user's full privileges: no CSP applies, no capability list constrains it,
there is no sandbox, and nothing in Tauri changes that. Hence the entry under
[Considered and declined](#considered-and-declined): a runtime loader for
`.so`/`.dll` files should not be built. Rust plugins are distributed as source
and compiled in, and trust is established the way it is for any other dependency.

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
- **Structured detection logging.** Low value at two to six detectors. Trigger:
  detection getting slow enough to need debugging.
- **Frontend page-state extraction.** `+page.svelte` was around 250 lines before
  the views work and is 362 now. Watch it; don't pre-split it.

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
  exchange for a nicer install step. This declines *native* loading only —
  downloading a **WebAssembly** detector is the settled distribution model and
  is not the same decision, because a wasm module is sandboxed by construction
  and a `.dll` is not.
- **Recompiling the app on the user's machine to install a plugin.** A small
  package manager that fetches plugin source, rebuilds, and swaps the
  executable. It keeps plugins as full-power native Rust and needs no new API,
  which is genuinely attractive, but it charges every user a Rust toolchain and
  a multi-gigabyte build tree, cannot replace a running `.exe`, is undone by
  every official update, and breaks the macOS app signature on rebuild —
  foreclosing signed releases permanently. Compiling in CI at publish time buys
  the same outcome and none of that.

## Related work

**devmon** is a separate planned application — an activity tracker that attaches
`projects.db` read-only to attribute work to projects. It shapes nothing Project
Indexer must do, but it is why the `ProjectReader` port and the `meta` table
exist. The cross-app contract is a recorded decision; don't regress it.
