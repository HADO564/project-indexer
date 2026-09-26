# Roadmap — command-line tool

The CLI's detailed plans. The [main roadmap](../../ROADMAP.md) has where things
stand, the headline plans for both products, and everything shared through
`indexer-core`. The design contract is
[`2026-09-14-cli-design.md`](../superpowers/specs/2026-09-14-cli-design.md).

**Status: the first commands work** — `list`, `show`, `config` and `--json`; see
[`checklist.md`](checklist.md). The binary is called `indexer`
throughout; that name is a placeholder and will change.

**Released on its own cycle.** The CLI is versioned and tagged (`cli-v*`)
separately from the desktop app, with its own
[changelog](../../crates/cli/CHANGELOG.md). What ties the two together is the
database, not a version number — see
[Distribution and releases](#distribution-and-releases).

## The `indexer` command

The single largest planned piece, and the one the last refactor was for: an
observer, plain subcommands, and a keyboard-driven TUI over both.

### Observing

`indexer git init` runs the real `git init`, untouched, propagates its exit code,
and *notices* what happened — then records the project through the same
`ProjectService` the GUI uses. It never reimplements the tools it wraps. Because
both frontends open the same SQLite database, installing the CLI later connects
it to the GUI with no pairing and no IPC.

The backend seams already exist: `ensure_project` and `find_by_directory` have no
GUI caller and were added purely for this, and `projects.directory_normalized` is
indexed so directory lookup is not a table scan.

Still open: which commands are recognised first, and how each recognizer derives
the project directory from its arguments and the working directory. Two things
are settled: a name collision is resolved by `domain::naming::disambiguate`, the
function the scanner already shares with `ensure_project`; and plain subcommands
ship before the observer (below).

The full briefing, including the open questions, is in
[`docs/handoffs/2026-09-04-observer-cli.md`](../handoffs/2026-09-04-observer-cli.md).

### Plain subcommands

The unglamorous half: `indexer list`, `show`, `add`, `open`, `untrack`. Each maps
almost one-to-one onto a `ProjectService` method that already exists, so these
are cheap — the work is argument parsing and output formatting, not behaviour.

**Settled 2026-09-14: both ship, subcommands first.** They are separable, and
they are the layer the observer and the TUI both run through.

`indexer list` printing real rows from the shared database is the suggested first
vertical slice for the whole initiative. It proves the premise — same database,
no backend changes — in about twenty lines.

#### Scanning is part of the first release

`indexer scan ~/code` is how somebody with a disk full of projects gets a
corpus at all; `add` one directory at a time is not a substitute. It was
originally grouped with groups and the bin as "the rest of milestone 3", and
moved up on 2026-09-19 for that reason.

It is two steps in core already — `ScanService::scan` walks and registers
nothing, `import` registers a reviewed selection — and the CLI keeps that
split rather than collapsing it: **the walk prints what it found and exits;
registering takes `--import`.** Bulk registration is durable and a terminal
gives nothing back if it is wrong, so the deliberate step stays. The
`MAX_DIRECTORIES` bound is reported rather than swallowed, and a row that
fails to import is listed without failing the command.

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
  come from [plugins](../../ROADMAP.md#plugins).
- **A tracker is `{"kind": "git", …its fields}`.** The kind sits under one fixed
  key beside the tracker's own fields, not serde's default `{"Git": {…}}`, and
  it is built from the tracker's serde shape rather than a `match`, so a new
  detector needs no change here.
- **stdout is data, stderr is everything else.** Under `--json`, stdout carries
  the document and nothing else, so `indexer list --json | jq` needs no
  filtering. A failure never reaches stdout: it goes to stderr and the exit code.
- **Failures are documents too.** Under `--json`, stderr gets
  `{"schema": 1, "error": {"kind", "message", …}}` instead of prose, so a script
  can tell `not_found` from `ambiguous` (which carries the candidates) without
  parsing a sentence. Anything without a kind of its own is `error`. Usage
  errors from argument parsing stay prose, with exit code 2.

The shapes, field by field, are in [`agents.md`](agents.md).

The cost is one wrapper struct and a documented rule. The cost of skipping it is
a breaking change the first time somebody adds a project type.

#### Interactive picking

Three commands want the same thing, and it is **one component with three
callers**, not three features:

- `show app` where several projects match — choose, instead of printing the
  table and exiting 1
- `scan ~/code` — choose which candidates to import, instead of all or nothing
- `list --pick` — choose a project from the table, filtering as you type

Built once as "draw a list, filter, return the selection". Three bespoke
prompts would end up disagreeing about the keys and about what `Esc` does, and
that is the kind of disagreement nobody notices until a user hits it.

**It follows `fzf`'s split, which the [`--json` contract](#the---json-contract)
already sets.** The interface is drawn on stderr and the *selection alone* goes
to stdout, so picking composes with everything else:

```
indexer open "$(indexer list --pick)"
cd "$(indexer list --pick --print path)"
```

Gated on a terminal: with stdin piped, each of these keeps today's behaviour —
the table, the exit code, all-or-nothing — because a script cannot answer a
prompt, and reading its input as an answer would be worse than refusing. The
same rule `confirm.rs` already follows.

**Open: whether this overlaps the TUI**, which is already a list with `/`
search and `Enter` to focus. The distinction to hold onto is that a picker is
composed *inside* another command and then exits, while the TUI is somewhere
the user stays. Both can exist — but decided by default rather than
deliberately, it leaves the project with two project lists to keep in step.

#### Group colour and icons in a terminal

A group carries a `name`, a `color` and an `icon`, and the first two cross over
to a terminal better than they look.

**Colour is nearly free.** `core::domain::palette` already bounds the value to
one of eight swatch names or exactly `#rrggbb`, and seven of those eight names
(`cyan`, `gold`, `amber`, `violet`, `green`, `blue`, `pink`) are already
variants of the CLI's own `Color`; only `rust` is missing. `Color` also already
emits truecolor (`Code::Rgb` → `38;2;r;g;b`), so a hex literal has a rendering
path today. What is needed is a `Color::from_swatch` and a hex parser, not
colour support.

**Icons need a decision and a fix first.**

- *The decision:* there is no way to ask a terminal whether its font carries
  Nerd Font glyphs, so it cannot be detected — it is told.
  `indexer config icons nerd|emoji|off`, in `cli-settings.json` beside the
  colours, with a per-run `--icons`. `off` is a real choice, not a degraded one.
- *The renderer:* a fallback chain — stored name → the style's glyph → the
  style's default (`folder`) → nothing. The 25 bundled names all have obvious
  equivalents; `custom:my-logo` is an uploaded image and always falls back,
  which is why the chain has to exist at all.
- *The fix:* **`text_width` counts `chars()`, not display columns.** An emoji is
  one char and two columns, so an emoji in a table cell under-pads its column
  and shifts every column to its right on that row. This is already a latent
  bug for CJK project names, it is just rare. `unicode-width`'s
  `UnicodeWidthStr::width` inside that one helper fixes both, and every caller
  already goes through it.

So the order is: colour first (no blockers), `unicode-width` next, then icons
in the TUI sidebar and optionally in `list`.

### The TUI — every change is a command

`indexer` with no arguments opens a terminal view of the same database. It is
deliberately **not** a terminal copy of the GUI: no forms, no buttons, no
dialogs. Keys and menus are shortcuts; every change goes through a command.

**One exception, decided 2026-09-27: the edit form.** `indexer edit <project>`
with no field flags opens a full-screen form of the project's fields, the way
the GUI's edit form shows them, and saving it makes the same single `update`
that `edit --description … --add-tag …` makes — so it is a way of typing an
`edit` command, not a second path to the database. It comes after the flags,
which everything non-interactive needs, and it is the natural first piece of
`ratatui` in the crate, which the TUI then builds on. Piped, or under
`--json`, a bare `edit` is a usage error rather than a form nobody can see.

- **Panes.** The sidebar views — All, Favourites, each group, Ungrouped, Bin —
  with counts, the project list, and the selected project's detail: its
  identity and per-detector status, the same data `inspect` returns.
- **Keybinds for movement.** `j`/`k`, `gg`/`G`, `/` to search, `Tab` between
  panes, `Enter` to focus a project.
- **Letter shortcuts for common commands.** `o` runs `:open` on the selected
  project, `d` runs `:untrack` (with its `y/n`), and `n` opens the `:` line with
  `:add ` already typed — the current directory pre-filled, `Tab` completing
  paths.
- **An action menu per project**, LazyVim / which-key style, on `Space` and on
  right-click where the terminal reports mouse events: Open, Favourite, Move to
  group…, Untrack. Choosing an entry runs, or pre-fills, that command. The menu
  is built from the `Command` definitions, so a new command appears in it
  without menu code. Every entry also has a key, because not every terminal or
  SSH session passes the mouse through.
- **A `:` command line for everything else.** `:add ~/code/foo`, `:open`,
  `:group work`. The line is parsed by the same definitions as the shell, so
  `:open` and `indexer open` are one command rather than two that happen to
  agree. Where the shell needs an explicit project, the TUI supplies the
  selected one.
- **Destructive commands confirm on the command line** — a `y/n` prompt, not a
  dialog.
- **It stays current.** A write from another process — the GUI, a shell, the
  observer — shows up without a restart. SQLite's `PRAGMA data_version` changes
  whenever another connection commits, so a cheap poll does the job a file
  watcher would.

**What had to move into `indexer-core` first — done.** The GUI's views, counts
and search — the `name: value` property syntax included — used to live in
TypeScript (`src/lib/views.ts`). A TUI that disagreed with the GUI about what
"Ungrouped" or `client: acme` means would be a bug with no single place to fix
it, so that logic moved to `core::domain::views`, and the GUI switched over in
the same change.

Still open: the exact keymap, whether there is a help overlay or a
which-key-style hint line, and how much sort and filter state is remembered
between runs.

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
  writer; the second writer gets `SQLITE_BUSY` unless a busy timeout is set — and
  one is: every connection sets `busy_timeout = 5000`, so a second writer waits
  up to five seconds rather than failing. An agent is the first consumer likely to write *while* the
  GUI is open, so this stops being theoretical the moment this work starts.
- **What an agent is allowed to do.** Reading and registering are
  uncontroversial. `delete_project_directory` reached from a tool call is not.
  The defensible default is that the agent surface is read-plus-register, and
  destructive operations stay behind a human — the same argument that gives
  [storage](../../ROADMAP.md#storage--what-a-project-costs-on-disk) its review step.
- **Whether the GUI remains the primary face.** Asked plainly, because if the
  answer is no, that reorders most of this document rather than adding to it.

## Distribution and releases

**Packaging comes after the package.** Nothing here is started, and none of it
blocks building the CLI.

- **The desktop app provides `indexer` too, from its own binary.** Someone who
  installs only the app still gets the command — and so does any AI assistant
  on their machine. The app's binary runs the CLI when invoked as `indexer`,
  before Tauri starts, and an "Install command-line tool" action puts that name
  on `PATH`. Not a bundled sidecar: `indexer` is ~1.7 MB compressed, almost all
  of it core and SQLite the app already has, while the TUI adds only ~0.15 MB,
  so it isn't split out either (measured 2026-09-15). The brief, with the
  Windows console and startup-time pitfalls, is
  [`../handoffs/2026-09-15-gui-provides-indexer.md`](../handoffs/2026-09-15-gui-provides-indexer.md).

- **Installed through package managers, never downloaded by hand** — Homebrew,
  winget, and likely Scoop; Linux channels are undecided. Homebrew means the
  project's own tap: its core repository only accepts DFSG-compatible licences,
  and FSL is source-available rather than open source. Package managers still
  install binaries that CI builds — winget in particular cannot build from
  source — so a CLI release publishes per-platform archives. Users just never
  fetch them directly.
- **Its own release cycle.** Tags are `cli-v<version>`, which the app's `v*`
  release workflow does not match. Anything that asks GitHub for "the latest
  release" must filter by prefix, because both products publish to one
  repository — the app's planned updater included.
- **The database is the compatibility contract.** `SqliteRepository::open`
  refuses a database written by a newer schema, so a CLI older than the app
  that migrated the database stops with a clear error. Loud and safe, but only
  acceptable if it is brief — so **a schema bump ships with a CLI release that
  understands it.**

**Superseded by this.** The 2026-09-02 design had the CLI update itself
(`indexer self-update` plus a throttled stderr hint) and the GUI download the
CLI on demand and put it on `PATH`. A binary that replaces itself fights
`brew upgrade` and `winget upgrade`, and a GUI placing executables on `PATH`
duplicates the package manager's job. Both are dropped; the GUI can at most show
the right install command.

## Deferred — gated on a trigger, not a date

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

