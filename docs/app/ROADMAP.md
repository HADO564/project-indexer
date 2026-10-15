# Roadmap — desktop app

The desktop app's detailed plans. The [main roadmap](../../ROADMAP.md) has where
things stand, the headline plans for both products, and everything shared
through `indexer-core` — detectors, git support, storage, and plugins, themes
included.

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

## Project linking

> **Deferred — a later release, not before the TUI.** Held back on purpose: the
> [TUI](../cli/ROADMAP.md#the-tui--every-change-is-a-command) is the larger
> milestone and should land first, and linking must not be allowed to weigh the
> app down. When it is picked up, build it in two steps (below) — the lean
> data/CLI layer first, the whole-set graph canvas only if real, accumulated
> links prove it worth the weight.

Connect one project to another the way Obsidian connects notes — an explicit,
navigable edge between two entries, and a graph view over the whole set. The
project list answers "what do I have"; links answer "what did I build this
*out of*", which is the question that turns a list into a knowledgebase.

The value shows up where a flat list is weakest: a tool and the game that uses
it, a fork and its upstream, a client engagement and the three repositories it
spans. Groups already give one exclusive band per project — links are the
non-exclusive, many-to-many relation that groups deliberately are not.

A link **names itself.** The edge carries a short label the user writes — not a
fixed type from a menu but the actual relationship. The motivating case: a Unity
game whose C# code lives in one folder and whose art and scene assets live in
another, tracked as two separate projects (each detected on its own terms, both
already in the database), joined by an edge the user calls "assets" — or
"scripts", read from the other end. The flat list and the detectors can see each
folder; only the named edge records that they are one thing worked on in two
places.

Links are worth exposing to **agents and scripts**, not just the canvas. An edge
in `show`'s `--json` lets an agent asked about the game jump straight to the
assets folder it was built from — and back — instead of guessing from names or
paths. That makes linking a producer for the
[agent surface](../cli/ROADMAP.md#agent-access--mcp-or-a-cli-plus-a-skill): the
CLI manages edges (`dexily link <a> <b> --as assets`, `dexily unlink`,
`dexily links <project>`) and carries them in every project's `--json`, so the
GUI's graph and an agent's lookup read the same relations.

**Two steps, by weight.** First the lean layer: the edges as core data (a small
table and a `LinkService`), the CLI to manage them, the edges in each project's
`--json`, and at most a "Related" list on the project page. That carries the
whole value above — the named relationship, the agent traversal — and adds
nothing for anyone who never links. The whole-set graph *canvas* is the second
step and the costly one (a route of its own and a bundled drawing library), so
it waits for its own trigger: enough links in real use that seeing them laid out
earns its keep. Until then the app stays exactly as light as it is now.

The graph is drawn with **Svelte Flow** (`@xyflow/svelte`), the SvelteKit
counterpart to React Flow from the same authors. It has to be a bundled
dependency rather than anything CDN-loaded — the content security policy
forbids fetching code at runtime, and that is a property worth keeping.

What needs deciding:

- **The label, and whether the edge is directed.** The user writes the label,
  so it is free text (`assets`, `depends on`, `forked from`), not a fixed menu.
  But some read one way — `forked from` — and the two ends may want different
  words (`scripts` ↔ `assets`), which is really a direction question. A single
  undirected label is the cheap start; a per-direction second label and an
  explicit direction are additive-then-not, in that order — so decide early
  whether an edge is one labelled line or two.
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
- **One model, two surfaces.** The edges are core data — a real table and a
  `LinkService`, built on the same migration machinery as groups — surfaced by
  the GUI canvas *and* by the CLI and `--json` alike, not a GUI-only feature.
  Settling the core shape (the two project ids, the label, any direction) before
  either frontend is drawn keeps them reading the same relations and lets agents
  traverse links the moment they exist.

## Global shortcut

The plugin is registered but no shortcut is bound to anything. Deciding what it
should *do* is the open part.

## Updates

Designed but not built. All of it is fast-follow work on top of the release
pipeline that already exists.

- `tauri-plugin-updater` wiring, with a `core::updates::latest_stable` helper
  that defines "latest" in one place — and considers only the app's `v*` tags,
  since CLI releases (`dexily-v*`) publish to the same repository.
- A dismissible in-app release notification, rather than an interrupting dialog.
- Tag → signed bundle → GitHub Release in CI.

The obligation this places on the current code — a safe schema-migration path, so
a newer binary opening an older database is routine rather than dangerous — is
already met: migrations are numbered `user_version` steps and `open` refuses a
database written by a newer binary.

The CLI is updated by whichever package manager installed it; its side of this —
and why the earlier self-update and download-the-CLI designs were dropped — is in
the [CLI's roadmap](../cli/ROADMAP.md#distribution-and-releases).

## Deferred — gated on a trigger, not a date

- **Frontend page-state extraction.** `+page.svelte` was around 250 lines before
  the views work and is 362 now. Watch it; don't pre-split it.
