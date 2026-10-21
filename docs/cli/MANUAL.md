# dexily — command manual

`dexily` tracks your projects from the terminal. It reads and writes the same
database as the Dexily desktop app, so what you track in one shows up in the
other — and it works with no app installed.

This is the user-facing reference. For the machine-readable `--json` contract
(for scripts and agents) see [`agents.md`](agents.md); for the full list of
commands the observer recognises and project types it detects see
[`SUPPORTED-COMMANDS-AND-PROGRAMS.md`](SUPPORTED-COMMANDS-AND-PROGRAMS.md).

> `dexily <command> -h` gives a one-line summary of each flag; `--help` adds the
> full detail for each. This manual is the readable overview of the lot.

## Contents

- [Install](#install)
- [Three ways to run it](#three-ways-to-run-it)
- [Global options](#global-options)
- [Naming a project](#naming-a-project)
- [Commands](#commands)
  - [Viewing](#viewing) · [Tracking](#tracking) · [Changing](#changing) ·
    [The bin](#the-bin) · [Groups](#groups) · [Icons](#icons) · [Settings](#settings)
- [Observing commands](#observing-commands)
- [Colours and icons](#colours-and-icons)

## Install

```bash
brew tap HADO564/dexily-tap   # macOS and Linux, once
brew install dexily           # then `brew upgrade dexily` for a new release
cargo install dexily          # or from crates.io (needs a Rust toolchain)
```

Or a prebuilt installer / archive from the latest `dexily-v*` release — the
exact one-liners and platforms are in [`crates/cli/README.md`](../../crates/cli/README.md).

## Three ways to run it

| You type | What happens |
|---|---|
| `dexily` | Opens the full-screen **TUI** — a browser of your projects. Navigate with `j`/`k`, `Tab`/`h`/`l` between panes, `/` to search, `n` to add the folder you started it in, `e` to edit the selected project in a form (`E` for every field), `:` to run any command below, `q` to quit. In the Groups pane, `n`, `e` and `d` make, rename and delete groups. It refreshes by itself when the app, another shell or the observer changes your projects. |
| `dexily <command> …` | Runs **one command** and exits (everything in this manual). |
| `dexily <program> …` | **Observes** a recognised scaffolding command (`dexily git clone …`, `dexily cargo new …`) — runs it, then tracks what it made. See [Observing commands](#observing-commands). |

## Global options

These work on any command (they go before the command's own arguments):

| Option | Meaning |
|---|---|
| `--json` | Print a versioned JSON document instead of human output. See [`agents.md`](agents.md). |
| `-y`, `--yes` | Answer yes to every confirmation prompt. |
| `-q`, `--quiet` | With an observed command, print none of dexily's own lines. |
| `-v`, `--verbose` | Surface otherwise-silent diagnostics on stderr (honours `RUST_LOG`). |
| `--folder-color <c>`, `--header-color <c>` | Override the table colours for this run. |
| `-h`, `--help` · `-V`, `--version` | Help; version. |

## Naming a project

Commands that act on a project take a `<PROJECT>` you can give as, tried in this
order: an **exact name**, a full **id**, an **id prefix** of 8+ characters, a
**`parent/folder` path ending**, or **part of a name**. Names are not unique, so
when two could match, dexily asks — use the id to be exact (`dexily show 087039a7`).

## Commands

### Viewing

**`dexily list [QUERY]`** — list tracked projects as a table.

| Flag | |
|---|---|
| `[QUERY]` | keep projects whose name contains it (or, with a `/`, whose path ends with it) |
| `-s, --sort name\|last-opened` | order the table (default `name`) |
| `-r, --reverse` | flip the sort |
| `--view all\|favorites\|binned` | which set to draw from (default `all`); binned projects appear only under `binned` |
| `-t, --tracker <kind>` | keep only projects with that tracker; repeat or comma-separate |

```bash
dexily list                      # everything, A–Z
dexily list api --tracker git    # git projects whose name contains "api"
dexily list --view favorites -s last-opened
```

**`dexily find <QUERY>`** — search the way the app's search bar does, and print
the same table as `list`. Where `list` matches a name (or a path ending),
`find` matches **any part of** the name, the path, a tag or a property value —
or, written `name: value`, only that one property. Case is ignored.

| Flag | |
|---|---|
| `<QUERY>` | free text, or `name: value` for one property (`client: acme`; `client:` alone finds every project that has a `client`) |
| `-s`, `-r`, `--view`, `-t` | as for `list` |

```bash
dexily find rust                 # name, path, tag or property value contains "rust"
dexily find "client: acme"       # the client property contains "acme"
dexily find work --view binned   # binned projects anywhere under a "work" folder
```

**`dexily show <PROJECT>`** — one project's full detail. `-t <kind>` expands that
tracker's own fields (branch, engine, …).

### Tracking

**`dexily add [DIRECTORY]`** — start tracking a directory (defaults to the
current one); its trackers are detected. Already-tracked is not an error.

**`dexily untrack <PROJECT>`** — stop tracking a project; its files are left
alone. Needs `--yes` when stdin is not a terminal.

**`dexily scan <DIRECTORY>`** — find projects under a folder without tracking
them. `--depth <n>` (default 1 = children only), `--include-ignored` (look in
`node_modules`, `target`, dot-dirs…), `-t <kind>`, and `--import` to register
what it finds.

```bash
dexily scan ~/code --depth 2
dexily scan ~/code --import
```

### Changing

**`dexily edit <PROJECT> [flags]`** — change a project's fields; all the flags
given are saved in one write. With no field flag it opens a full-screen form
(`--full` opens the form with every field).

| Flag | |
|---|---|
| `--name <text>` | rename (an empty or taken name is refused) |
| `--directory <path>` | point at another folder after moving it on disk |
| `--description <text>` · `--notes <text>` · `--open-with <app>` | set these; `""` clears |
| `--add-tag <tags>` · `--remove-tag <tags>` | add/remove tags (repeat or comma-separate; removals apply first) |
| `--set <key=value>` · `--unset <key>` | set/remove a property (repeat; unsets apply first) |
| `--group <name>` · `--ungroup` | put in / take out of a group |
| `--color <colour>` · `--icon <icon>` | the project's mark; `""` clears |

```bash
dexily edit api --add-tag rust,web --set client=acme
dexily edit api --group Work --color violet
```

**`dexily favorite <PROJECT>`** / **`dexily unfavorite <PROJECT>`** — toggle the
favourite flag.

**`dexily open <PROJECT>`** — open the project in its `open-with` app, or the
system default. Stamps its last-opened time.

### The bin

A project reaches the bin when its folder is deleted. The record stays; the
files do not come back.

**`dexily restore <PROJECT>`** — bring a binned project's record back (matched
only against the bin).

**`dexily purge <PROJECT>`** — permanently delete a binned record. Asks first;
cannot be undone. Needs `--yes` when stdin is not a terminal.

### Groups

A project is in one group at a time; put it in one with `edit --group`.

| Command | |
|---|---|
| `dexily group list` | the groups, in sidebar order, with their project counts |
| `dexily group create <name> [--color <c>] [--icon <i>]` | make one (default cyan / briefcase) |
| `dexily group edit <group> <--name\|--color\|--icon …>` | rename or restyle |
| `dexily group delete <group>` | delete it; its projects are kept, ungrouped. Asks first |

### Icons

Your own SVG icons, in the same store the app keeps — one added here shows in
the app, and the other way round. Put one on a project with
`edit --icon custom:<name>`. (Groups take the bundled icons only.)

| Command | |
|---|---|
| `dexily icon list` | your custom icons, as `custom:<name>`, with how many projects use each |
| `dexily icon import <file.svg>` | add one, cleaned as the app cleans it, named after the file (`Logo.svg` → `logo`, or `logo-2` if that is taken) |
| `dexily icon delete <name>` | delete one (`logo` or `custom:logo`). Asks first; projects using it show the folder icon |

### Settings

`dexily config <setting> [value] [--reset]` — show a setting with no value, set
it with one, or `--reset` to the default.

| Setting | |
|---|---|
| `folder-color` / `header-color` | default table colours |
| `icons nerd\|emoji\|off` | how icons draw (off by default; `nerd` needs a Nerd Font) |
| `form-wrap on\|off` | whether the edit form's focus wraps at its ends |
| `track-prompt on\|off` | whether an observed command asks before tracking a folder |

## Observing commands

`dexily <program> …` runs a **recognised** scaffolding command and then tracks
the project it created:

```bash
dexily git clone https://github.com/you/thing
dexily cargo new myapp
dexily mkdir demo
```

Only an allowlist of commands run this way (`mkdir`, project-making `git`
subcommands, `cargo`/`go`/`gh`/`uv`/`npm`/… init); anything else is refused
before it runs. Flags for the new project go **before** the command
(`dexily --group Work git clone …`). The full list and the rules are in
[`SUPPORTED-COMMANDS-AND-PROGRAMS.md`](SUPPORTED-COMMANDS-AND-PROGRAMS.md).

## Colours and icons

**Colours** are a palette name — `cyan`, `gold`, `amber`, `rust`, `violet`,
`green`, `blue`, `pink` — or a `#rrggbb` hex value. **Icons** are a bundled name
(`folder`, `briefcase`, `code`, `rocket`, `terminal`, `gamepad`, …) or
`custom:<name>` for one added in the app or with `dexily icon import`.
`NO_COLOR` in the environment drops all colour.

A terminal draws text, not SVG, so a custom icon shows as the folder glyph in
`list` and `show`. **The TUI draws it as the image itself**, at the top of the
detail pane, in a terminal that can show images and says so in its
environment — Kitty, WezTerm, Ghostty, iTerm2, Konsole, foot — and not inside
tmux or screen. Anywhere else it is the folder glyph there too.
