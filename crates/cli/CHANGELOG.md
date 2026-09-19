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

- `indexer list` prints tracked projects as a bordered table — name, `parent/folder`, trackers, last opened — that fills at least 70% of the terminal and sits centred in it.
- `indexer scan <dir>` walks a folder for projects and prints what it found — directory, the name it would get, what matched, and whether it is already tracked — registering nothing. `--import` performs the registration as a second run, `--depth <n>` looks further than the immediate children, `--include-ignored` looks inside `node_modules`, `target` and dot-directories, and `-t/--tracker` restricts the detectors. A walk that hits the 50,000-directory limit says so rather than reading as an empty disk, and a row that fails to import is reported without failing the command.
- `--tracker <kind>` (short `-t`) on `list` and `show` keeps only the projects carrying that tracker, before the query is matched — so `show app --tracker git` finds the git `app` where `show app` alone reports several matches. `git` and `unreal` are the kinds; anything else is rejected with the list of valid ones. Name several by repeating the flag or with commas (`-t git,unreal`), and a project carrying any of them is kept.
- With `--tracker`, the table's TRACKERS column becomes those trackers' own columns — BRANCH and CHANGES for git, ENGINE for Unreal — in the order asked for, with `-` where a project lacks a kind. `show` gains a section per kind under the project's details: branch, changes and remote for git; project, engine and source control for Unreal.
- `indexer list <query>` lists only the projects whose name contains the query — the exact name first, then names starting with it, then the rest — or, for a query with a `/` like `work/app`, whose path ends with it. Nothing matching prints `no projects match "…"` and exits 0.
- `indexer config folder-color` and `indexer config header-color` set the colour of each project's folder name and of the table's header row; `--folder-color` and `--header-color` override them for one run.
- `--json` failures: under `--json`, a failed command writes `{"schema": 1, "error": {"kind", "message", …}}` to stderr instead of prose. `show` reports `not_found`, or `ambiguous` with the matching projects; anything else is `error`. The format is documented for scripts and LLM agents in `docs/cli/agents.md`.
- `indexer show <query>` shows one project: an exact name first, then a full id or an id prefix of 8 or more characters, then a `parent/folder` path ending, then part of a name (ignoring case). When several match, it prints a table of them — name, `parent/folder`, trackers, last opened — and exits 1.

### Changed

- `--json` writes each tracker as `{"kind": "git", …its fields}` instead of `{"Git": {…}}`.
- When several projects match, `show` now says "multiple matches found, which one did you mean?" above the table.
