# Changelog — indexer-cli

All notable changes to the Project Indexer command-line tool are documented here.
The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The CLI is released independently of the desktop app, under tags of the form
`cli-v<version>`. The app's changes are in the root
[`CHANGELOG.md`](../../CHANGELOG.md).

## [Unreleased]

Nothing released yet.

### Added

- The project table gains an ID column — the first 8 characters of each project's id, the shortest prefix `show` accepts — so a row can be copied from by id as well as by `parent/folder`.
- `indexer list --view <set>` draws from `all` (the default), `favorites` or `binned`. One flag with a value rather than a `--favorite` and a `--binned` flag, because the sets are alternatives — and because the GUI already models them as one `View` type with groups as a further case. Binned projects appear under `binned` and nowhere else: favouriting is stored independently of binning, so a favourited project that is binned leaves `favorites` and returns on restore. An empty view now says which emptiness it is — `the bin is empty`, `no favourites yet` — rather than always claiming nothing is tracked.
- `indexer list --sort <field>` orders the table by `name` (A to Z) or `last-opened` (most recent first), with `-r/--reverse` to flip it. Each field has its own natural direction, so `--reverse` means "the opposite of this field", not "descending" — `--sort last-opened` answers most-recent-first, and only `--sort last-opened --reverse` reaches for the stalest.
- `indexer show` reports the group a project belongs to. Ungrouped projects show no group line at all, the way a tracker section a project lacks is skipped rather than printed empty.
- `indexer add [dir]` starts tracking a directory, defaulting to the current one — `indexer add` inside a project folder is the common case. The path is resolved to an absolute one before it is stored, so `add .` and `add ../app` record where they actually point; a path that does not exist, or that is a file, is refused. A directory that is already tracked is reported as such rather than duplicated or treated as an error.
- `indexer open <query>` opens a project in its application — the one in `open_with`, or the system default — and records that it was opened. A directory that has been deleted or moved, or an `open_with` app that is no longer installed, is reported before anything is launched.
- `indexer untrack <query>` forgets a project's metadata and leaves its directory on disk alone. It asks for confirmation first; `--yes` answers in advance, and with stdin piped it refuses rather than reading a script's input as consent. Answering no prints `cancelled` and exits 0.
- `indexer list` prints tracked projects as a bordered table — name, `parent/folder`, trackers, last opened — that fills at least 70% of the terminal and sits centred in it.
- `indexer scan <dir>` walks a folder for projects and prints what it found — directory, the name it would get, what matched, and whether it is already tracked — registering nothing. `--import` performs the registration as a second run, `--depth <n>` looks further than the immediate children, `--include-ignored` looks inside `node_modules`, `target` and dot-directories, and `-t/--tracker` restricts the detectors. A walk that hits the 50,000-directory limit says so rather than reading as an empty disk, and a row that fails to import is reported without failing the command.
- `--tracker <kind>` (short `-t`) on `list` and `show` keeps only the projects carrying that tracker, before the query is matched — so `show app --tracker git` finds the git `app` where `show app` alone reports several matches. `git` and `unreal` are the kinds; anything else is rejected with the list of valid ones. Name several by repeating the flag or with commas (`-t git,unreal`), and a project carrying any of them is kept.
- With `--tracker`, the table's TRACKERS column becomes those trackers' own columns — BRANCH and CHANGES for git, ENGINE for Unreal — in the order asked for, with `-` where a project lacks a kind. `show` gains a section per kind under the project's details: branch, changes and remote for git; project, engine and source control for Unreal.
- `indexer list <query>` lists only the projects whose name contains the query — the exact name first, then names starting with it, then the rest — or, for a query with a `/` like `work/app`, whose path ends with it. Nothing matching prints `no projects match "…"` and exits 0.
- `indexer config folder-color` and `indexer config header-color` set the colour of each project's folder name and of the table's header row; `--folder-color` and `--header-color` override them for one run.
- `--json` failures: under `--json`, a failed command writes `{"schema": 1, "error": {"kind", "message", …}}` to stderr instead of prose. `show` reports `not_found`, or `ambiguous` with the matching projects; anything else is `error`. The format is documented for scripts and LLM agents in `docs/cli/agents.md`.
- `indexer show <query>` shows one project: an exact name first, then a full id or an id prefix of 8 or more characters, then a `parent/folder` path ending, then part of a name (ignoring case). When several match, it prints a table of them — name, `parent/folder`, trackers, last opened — and exits 1.

### Changed

- `show`, `open` and `untrack` resolve their query through one shared `find_one`, so the same query finds the same project whichever verb is in front of it. `open` and `untrack` accept `-t/--tracker` for the same reason `show` does.
- `--json` writes each tracker as `{"kind": "git", …its fields}` instead of `{"Git": {…}}`.
- When several projects match, `show` now says "multiple matches found, which one did you mean?" above the table.
