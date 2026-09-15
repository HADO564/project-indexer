# Roadmap

Where Project Indexer is going, why, and — just as usefully — what has been
considered and ruled out. Nothing here carries a date. Items move when the work
that unblocks them lands, not when a quarter ends.

The repository holds two products over one shared core, and each keeps its own
detailed roadmap. This file is the overview.

| Roadmap | Covers |
|---|---|
| **This file** | where things stand, the headline plans for both products, and everything they share through `indexer-core` — licensing, detection, git, storage, plugins, what was declined |
| [`docs/app/ROADMAP.md`](docs/app/ROADMAP.md) | the desktop app — folder scanning, project linking, the global shortcut, app updates |
| [`docs/cli/ROADMAP.md`](docs/cli/ROADMAP.md) | the command-line tool — the observer, subcommands and `--json`, the TUI, agent access, distribution |

Feature status is tracked per product, in
[`docs/app/checklist.md`](docs/app/checklist.md) and
[`docs/cli/checklist.md`](docs/cli/checklist.md); the non-feature quality backlog
is in [`docs/architecture.md`](docs/architecture.md). The CLI's design contract is
[`docs/superpowers/specs/2026-09-14-cli-design.md`](docs/superpowers/specs/2026-09-14-cli-design.md).

## Where things stand

**v0.3.1** is the current release; **v0.2.0** was the first under the new
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
which is what lets the CLI share it without touching the backend. Storage is SQLite behind numbered `user_version` migrations, currently
at version 3.

Windows and Linux are both built and tested in CI. macOS builds in the release
workflow but is not yet functionally complete (see below).

The command-line tool is still a stub in `crates/cli`, but its direction is
settled — see [its roadmap](docs/cli/ROADMAP.md). It is released on its own
cycle, separately from the app.

## Highlights

**Desktop app** — [full roadmap](docs/app/ROADMAP.md)

- **Project linking** — explicit, navigable edges between projects, and a graph
  view over them.
- **Updates** — `tauri-plugin-updater` and a dismissible release notification.
- **Global shortcut** — registered but unbound; deciding what it does is the
  open part.

**Command-line tool** — [full roadmap](docs/cli/ROADMAP.md)

- **The observer** — `indexer git init` runs the real command and records the
  project it created. This is what makes the CLI more than a second GUI.
- **Plain subcommands and `--json`** — most of what the GUI does, scriptable,
  with an output contract settled before the code.
- **A view-only TUI** — keybinds and a `:` command line, LazyVim-style; every
  change is a CLI command.
- **Package-manager installs** — Homebrew, winget and the like. After the
  package itself works.

**Shared — `indexer-core`**, detailed below

- **The re-detect sweep, then Unity and Blender.** The sweep's backend half
  shipped in 0.3.1 and nothing triggers it yet; Unity must not ship before it
  does.
- **Deeper git support** — ahead/behind upstream first.
- **Plugins** — themes as data first, then backend-only detectors as
  WebAssembly.
- **macOS completeness** — installed-application discovery, which both
  frontends need.
- **Storage reporting** — what a project costs on disk, reported rather than
  cleaned.

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
[agent surface](docs/cli/ROADMAP.md#agent-access--mcp-or-a-cli-plus-a-skill).

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

The global shortcut, which only the app has, is in the
[app's roadmap](docs/app/ROADMAP.md#global-shortcut).

- **macOS.** Installed-application discovery returns an empty list, so the "open
  with" picker has nothing to offer and launching falls through to the generic
  opener. This is also the natural moment to put `list_installed_apps` behind a
  trait: a third implementation is what makes that seam pay for itself, and until
  then a plain function is honest.

## Deferred — gated on a trigger, not a date

These are not "someday". Each has a specific condition that should start it.

- **Fast vs deep detection tiers.** Split cheap marker detection from opt-in deep
  inspection, with a cache keyed on directory and HEAD. Trigger: the first
  detector that genuinely needs expensive work — git contributors, or dependency
  parsing.
- **Structured detection logging.** Low value at two to six detectors. Trigger:
  detection getting slow enough to need debugging.

Deferred items that belong to one product live in its own roadmap: frontend
page-state extraction in the
[app's](docs/app/ROADMAP.md#deferred--gated-on-a-trigger-not-a-date), and
portfolio export in the
[CLI's](docs/cli/ROADMAP.md#deferred--gated-on-a-trigger-not-a-date).

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
