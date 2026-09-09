# Roadmap

Where Project Indexer is going, why, and — just as usefully — what has been
considered and ruled out. Nothing here carries a date. Items move when the work
that unblocks them lands, not when a quarter ends.

For fine-grained feature status see [`docs/checklist.md`](docs/checklist.md); for
the non-feature quality backlog see
[`docs/architecture.md`](docs/architecture.md).

## Where things stand

**v0.3.0** is the current release; **v0.2.0** was the first under the new
licence. The
app tracks projects, detects git and
Unreal Engine trackers, opens projects in your installed applications, and runs
in the background from the system tray. A project carries a colour, an icon and
any number of user-defined key/value properties, belongs to at most one group,
and is reached through a sidebar — All, Favourites, each group, Ungrouped, Bin —
rather than one flat column. Search covers name, path, tags and property values,
with `client: acme` looking inside one property and a bare `client:` finding
every project that has it. Projects need not be added one at a time: pointing
the app at a folder walks it for existing work and imports what it finds in one
pass, with name collisions resolved automatically and the last scan's settings
remembered for the next one.

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

## Agent access — MCP, or a CLI plus a skill

Drive the app from an agent instead of a window. The premise is the same one the
CLI rests on: `indexer-core` has no Tauri dependency, so an agent surface is a
*fourth frontend* over the same `ProjectService` and the same SQLite file, not a
new backend.

**Whether it needs to be MCP is the actual question, and it is open.** Once
[plain subcommands](#plain-subcommands) and the `--json` contract above exist, an
agent skill is a markdown file that documents `indexer list --json` — no crate,
no protocol, no server lifetime, and no second output shape to keep in step with
the first. MCP buys typed tool schemas and a discovery handshake, and charges a
`crates/mcp` crate, a transport, and a parallel contract that will drift from
`--json` the first time one of them changes.

That settles the order rather than the outcome: **plain subcommands and `--json`
first, then judge.** If a skill over the CLI proves sufficient, MCP is never
built and nothing is lost. If it does not, MCP is then built over a JSON shape
already proved against a real consumer, which is the cheaper way round.

Three things have to be answered before either ships.

- **Concurrent writers.** Exactly one process writes today. A GUI, a CLI and a
  long-lived agent server means three. SQLite in WAL takes many readers and one
  writer; the second writer gets `SQLITE_BUSY` unless a busy timeout is set, and
  none is set today. An agent is the first consumer likely to write *while* the
  GUI is open, so this stops being theoretical the moment this work starts.
- **What an agent is allowed to do.** Reading and registering are
  uncontroversial. `delete_project_directory` reached from a tool call is not.
  The defensible default is that the agent surface is read-plus-register, and
  destructive operations stay behind a human — the same argument that gives
  [storage](#storage--what-a-project-costs-on-disk) its review step.
- **Whether the GUI remains the primary face.** Asked plainly, because if the
  answer is no, that reorders most of this document rather than adding to it.

## More project types

Detection is designed so a new tracker costs no frontend code: implement the
`Detector` trait, register it in one place, add the type, and the UI renders it
generically. See [`CONTRIBUTING.md`](CONTRIBUTING.md#adding-a-detector).

- **Unity** — the next detector, and the one the generic path was built for.
- **Blender** — same shape.

**Ship the re-detect sweep before either of these.** It does not gate the
scanner — the two are independent — but scanning first raises what it is worth:
importing two hundred projects before Unity ships turns a handful of stale
records into two hundred. Detection results are persisted, so adding a
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

**Shipped.** Point the app at `~/code` and it finds everything inside, instead
of adding projects one directory at a time. This closed the single biggest
usability gap for anyone adopting the app with an existing disk full of
work — and adoption is exactly when the manual path was most painful.

The mechanics that had to be decided:

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

  **Vocabulary.** "Autorunner" and "looping detector" both mean *this* — the
  user-triggered bulk import — and neither implies a background process. The
  word **background** is reserved for work that actually runs unprompted, and
  the only such idea here is *background rescanning* under *Rescanning* below.
  They are different features and must not merge under one name: this one is
  user-triggered, bounded and finite.
- **Review before committing.** A scan that silently registers two hundred
  entries is hostile. Find, present, let the user deselect, then add. Registering
  a project is a durable act; a bulk one should be a deliberate one.
- ~~**Name collisions.**~~ **Settled 2026-09-07: qualify by parent, editable in
  review, one function shared with `ensure_project`.** `domain::naming::
  disambiguate(preferred, parent_dir, taken)` returns `api` if free, else
  `work/api`, else `work/api (2)` as the terminating fallback. `taken` holds
  existing project names *plus* the names already assigned earlier in the same
  scan, which is what stops two rows of one report colliding with each other.
  Rows whose name was changed are flagged in the review list and editable.

  `ensure_project` uses the same function, which is the "solve it once" the
  question asked for — and fixes a real bug, since it currently hands an
  underived name to `create` and fails with `DuplicateName` the second time the
  observer CLI meets an `api` folder.
- ~~**Rescanning.**~~ **Settled 2026-09-07: a pre-filled form, not a stored
  entity.** The last scan's settings — root, mode, depth, detectors, pruning —
  persist in `localStorage` and pre-fill the scan form, written on commit rather
  than on scan. Rescanning is then: open the modal, press Scan. Already-tracked
  directories are filtered out, so what comes back is only what is new.

  **A `scan_roots` table was designed and cut.** The reasoning is worth keeping,
  because it is the argument that killed it: the *path* was never the friction —
  anybody scanning `~/projects` knows where `~/projects` is, so a remembered
  root does not save them anything a text field would not. And a rescan cannot
  skip the disk regardless, since a project that appeared yesterday is
  discoverable only by looking. What is actually worth remembering is the
  **settings**, because a rescan that quietly ran depth 2 instead of depth 4
  gives a different answer with nothing on screen saying why — and a pre-filled
  form solves exactly that for none of the cost of a table, two ports, root CRUD
  and a schema migration. It also keeps a list of one user's scan folders out of
  `projects.db`, which the devmon cross-app contract prefers.

  **What brings the table back:** several distinct scan locations —
  `~/projects`, `~/work`, `D:\clients` — each wanting its own settings and its
  own rescan. One set of defaults cannot serve three roots; a list can. Until
  somebody has that disk it is speculative, and nothing blocks adding it later,
  since the settings are already one serialisable struct — the migration would
  move where it is stored, not invent its shape.

  An mtime cache that skips unchanged subtrees was considered and declined
  separately: up to 50,000 stored rows to optimise a walk that is already
  milliseconds, when the expensive part is detection, which only runs on
  directories that survive pruning.

  Watching a root live is a further step and still not the first one.
- **Background rescanning — open, and not yet agreed.** The idea: traverse for
  projects unprompted rather than when asked. (Filed here as "an autorunner"
  before that word was pinned to the bulk scan above; it is *not* that feature.)
  The appeal is obvious; three things have to be answered before it is worth
  building.

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

Four things the implementation met, all known in advance because they were
written down here first:

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

The performance shape was worth getting right early: walking is I/O bound and
cheap, running full detection on every directory is not. Detection is gated
behind a cheap marker test — does a `.git` or `.uproject` even exist here —
which is the fast-versus-deep split again, arriving from a second direction.

**All four held.** `ScanService::scan` and `ScanService::import` sit over
`ProjectService::ensure_project`; the set filter shipped as
`DetectorRunner::inspect_kinds(path, Option<&[&str]>)`, with the single-kind
`inspect` reimplemented over it rather than retired; `import` collects
per-row failures instead of calling `refresh_trackers` or `into_result()`; and
`domain::naming::disambiguate` is what resolves `~/code/api` against
`~/work/api`, shared with `ensure_project` as predicted.

## Storage — what a project costs on disk

The index already knows where every project is, so it is most of the way to
knowing what each one *costs* and which parts of it are rebuildable. Turning that
into a report — largest first, with the reclaimable share named — is a small step
from data already held.

**It does not make this a disk cleaner, and the distinction is load-bearing.**
The README says the app "does not move your files, manage your repositories, or
replace your editor. It is an index." A feature that deletes directories
contradicts that sentence, and the sentence should not be quietly edited to
accommodate it. The reading that keeps both: **the index reports; the user
disposes.** It measures, ranks, and explains what is reclaimable and why.
Removal stays explicit, per item, and user-initiated, with the same review step
that makes a bulk scan a deliberate act rather than a surprise.

**The seam already exists.** The scanner's prune list — `node_modules`, `target`,
`.venv`, `build`, `dist` — is precisely the set of directories that are large,
rebuildable, and never projects. Today it means *do not descend into these*. A
storage report inverts it: *these are what you can get back*. One list, two uses,
and no second taxonomy to invent or maintain.

**Safe deletion is a prerequisite, and a gap today.**
`platform::filesystem::remove_directory` is `std::fs::remove_dir_all` — permanent
— and it is what `delete_directory`, and therefore the GUI's delete, already
calls. Nothing in this app reaches a recycle bin. The `trash` crate covers the
Windows Recycle Bin, the XDG trash and macOS Trash behind one call, so the change
is small, but it is not free: trashing fails outright on network and some
removable drives, and it does not free the space until the bin is emptied.
Both need an honest message. **A silent fallback to `remove_dir_all` when
trashing fails must never be written** — that is the one shape of this feature
that is worse than not having it.

**Measuring costs a full walk**, and a walk over `node_modules` stats hundreds of
thousands of files. That belongs nowhere near detection — invariant 2 in
[`docs/architecture.md`](docs/architecture.md), *basic detection stays cheap and
bounded*, rules it out of the fast path directly. So sizing is opt-in, explicitly
triggered, cancellable, and its result is stored rather than recomputed per view.

**The reported number will be wrong unless three things are decided.** Logical
size and size on disk differ, and neither is the obviously right one to show.
Hardlinks are counted once per link, which is not a corner case here: pnpm
hardlinks its content store into every `node_modules`, so a naive sum can promise
several gigabytes and free a few hundred megabytes. And junctions and symlinks
must not be followed, or the walk both double-counts and can loop. A confidently
wrong "4.2 GB reclaimable" is worse than reporting nothing.

Still undecided: whether staleness — last commit, last mtime — is part of this
report or a separate axis alongside size; whether it covers only tracked projects
or arbitrary directories, the latter making it a general disk tool and a far
larger thing than an index feature; and whether it surfaces as a GUI view, a CLI
subcommand, or only through the
[agent surface](#agent-access--mcp-or-a-cli-plus-a-skill).

### Prior art — TreeMap Disk Visualizer

[TreeMap](https://github.com/Prithvi-Web/TreeMap-Disk-Visualizer) is a
Node/Electron disk visualiser that ships a stdio MCP server alongside its GUI, so
an agent can drive a read-only audit and propose reclaims. It is a different
product from this one, and most of it is not worth copying — but it has already
paid for the answers to several questions above, and those are worth taking.

**Worth lifting.**

- **Deletion always routes through the OS trash; permanent removal is not an
  option the code has.** That is the same conclusion reached above, reached
  independently, which is the useful kind of corroboration.
- **Hard links are counted once, by design.** A defensible default for the
  accounting question above, and it settles the pnpm case in the honest
  direction. Copy-on-write clones stay an acknowledged over-count — worth
  documenting rather than pretending to solve.
- **Destructive operations are confined to the scanned root, with a system
  directory blocklist on top.** Cheap to implement, and exactly the right shape
  for an agent surface: it bounds what any tool call can reach regardless of what
  the agent was persuaded to ask for.
- **Every destructive operation is audit-logged.** The natural companion to a
  trash-only policy, and the thing that makes "nothing destructive" checkable
  rather than merely asserted.
- **Never leave zero copies.** Its duplicate finder refuses to delete every
  member of a group. Generalised: no automated pass may reduce a thing to none.

**Deliberately not copied.** Seventeen visualisation modes, cloud-account
scanning, network fleet monitoring, and a 50-endpoint REST API are a separate
product. **Autopilot policies — scheduled unattended cleanup — are the direct
opposite of the framing above** and should not arrive by imitation. Its Time
Capsule (copy the file, verify by SHA-256, then trash the original) is a genuinely
good idea in the wrong context here: it needs free space in order to free space.

**One thing to read it for rather than borrow.** Its scanner is a packed
structure-of-arrays — names in a UTF-8 pool, directories as contiguous id ranges
in breadth-first order, ~52 bytes per file — walked by a threadpool capped at
sixteen threads. The lesson transfers even though the code does not: size a tree
in one pass into flat arrays rather than building a tree of owned structs, and
cap concurrency rather than spawning per directory.

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
- **Portfolio and résumé export.** Turn tracked projects into the raw material
  for a CV or a GitHub profile: what each one is, what it is built with, when it
  was worked on, and how much of it is yours. The app should not write the prose
  — an agent or the user does — so its job is to hand over structured facts,
  which makes this a *consumer* of the [`--json` contract](#the---json-contract)
  and the [agent surface](#agent-access--mcp-or-a-cli-plus-a-skill) rather than a
  feature with machinery of its own. Trigger: **deep detection**, because the
  facts that make the output worth having — commit range, contributors, language
  mix — are exactly the expensive ones, which is why `GitInfo.contributors` is
  deliberately empty today. Until then an export would say little that the folder
  name does not.

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
