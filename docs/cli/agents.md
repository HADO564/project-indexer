# Using `indexer` from a script or an agent

Every `indexer` command takes `--json`. With it, the command prints one JSON
document instead of a table. This page describes those documents, for shell
scripts and for LLM agents that call the CLI as a tool. It is the "skill over
the CLI" that [`ROADMAP.md`](ROADMAP.md#agent-access--mcp-or-a-cli-plus-a-skill)
weighs against building an MCP server.

The rules behind the format are in
[`ROADMAP.md` → *The `--json` contract*](ROADMAP.md#the---json-contract). The
code is `crates/cli/src/output/json.rs`, and its tests are in
`crates/cli/src/tests/output/json.rs`.

## The rules a reader can rely on

- **A successful command prints `{"schema": 1, "data": …}` on stdout**, and
  nothing else goes to stdout, so `indexer list --json | jq` needs no filtering.
- **A failed command prints `{"schema": 1, "error": …}` on stderr** and exits
  non-zero. Nothing goes to stdout.
- **`schema` is bumped only for a change that breaks a reader.** Within one
  schema version, fields are only ever added. Ignore any field you don't
  recognise.
- **A tracker kind you have never seen is still a map with a `kind` key.** Skip
  it, don't fail on it. New detectors and, later, plugins add kinds.

### Exit codes

| Code | Meaning | Output |
|---|---|---|
| `0` | Success | `data` document on stdout |
| `1` | The command failed | `error` document on stderr |
| `2` | The arguments were wrong (unknown flag, missing argument) | clap's usage message on stderr, as **prose**, even under `--json` |

## Commands

| Command | `data` |
|---|---|
| `indexer list --json` | an array of [projects](#a-project), in the default sort order |
| `indexer list <query> --json` | an array of the projects whose name contains `query` (exact names first, then names starting with it, then the rest), or whose path ends with it when `query` contains `/`. An empty array, exit 0, when nothing matches |
| `indexer list --tracker <kind> --json`, `indexer show <query> --tracker <kind> --json` | the same, narrowed to projects carrying that tracker (`git` or `unreal`) before the query is matched. Name several by repeating the flag or with commas (`-t git,unreal`) to keep projects carrying any of them. An unknown kind is a usage error, exit 2. The flag picks columns and detail sections in human output; the JSON document is the same with or without it, and always carries every tracker in full |
| `indexer show <query> --json` | one [project](#a-project). `query` is an exact name, a full id, an id prefix of 8+ characters, a `parent/folder` path ending, or part of a name, tried in that order. No match or several matches is an [error](#errors) |
| `indexer scan <dir> --json` | `{"candidates": [{"directory", "suggested_name", "matched_kinds", "already_tracked", "disambiguated"}], "visited", "stopped_early"}`. Nothing is registered. `stopped_early` means the walk hit its 50,000-directory limit and the list is incomplete |
| `indexer scan <dir> --import --json` | `{"imported": [projects], "skipped", "failures": [{"directory", "message"}]}`. Registers every candidate; an already-tracked directory counts in `skipped`, and a row that fails is listed in `failures` without failing the command (exit 0) |
| `indexer config folder-color --json` | `{"folder_color": "cyan"}` — the colour in effect after the command |
| `indexer config header-color --json` | `{"header_color": "magenta"}` |

`scan` takes `--depth <n>` (1, the default, visits only the folder's
children), `--include-ignored` and `-t/--tracker` to narrow what it looks for.

`add`, `open`, `untrack`, the observer and the TUI are not written yet. They
fail with an error of kind `error`.

**To act on one project, use its id.** `show <id>` is exact, and names are not
unique. Two projects called `app` in different folders is normal.

## A project

```json
{
  "id": "6c5d931e-0000-4000-8000-000000000001",
  "name": "app",
  "description": "",
  "directory": "/Users/me/work/app",
  "created_at": "2026-09-01T00:00:00Z",
  "updated_at": "2026-09-01T00:00:00Z",
  "last_opened_at": "2026-09-15T10:00:00Z",
  "favorite": false,
  "is_deleted": false,
  "tags": ["client"],
  "properties": {"client": "acme"},
  "group_id": null,
  "color": null,
  "icon": null,
  "open_with": null,
  "notes": null,
  "trackers": [
    {
      "kind": "git",
      "repo_root": "/Users/me/work/app",
      "curr_branch": "main",
      "dirty": true,
      "detached_head": false,
      "repo_url": "git@github.com:me/app.git",
      "web_url": "https://github.com/me/app",
      "branches": null,
      "commit_hash": null,
      "contributors": []
    }
  ]
}
```

| Field | Type | Notes |
|---|---|---|
| `id` | string | A UUID. Stable for the life of the project |
| `name` | string | Not unique |
| `directory` | string | Absolute path |
| `created_at`, `updated_at` | string | RFC 3339, UTC |
| `last_opened_at` | string or null | null if never opened |
| `favorite`, `is_deleted` | bool | `is_deleted` means the project is in the bin |
| `tags` | string array | |
| `properties` | object of strings | The user's own key/value facts |
| `group_id` | string or null | null means Ungrouped |
| `color`, `icon` | string or null | A palette name or `#rrggbb`; a bundled icon name or `custom:<name>` |
| `open_with`, `notes` | string or null | |
| `trackers` | array | What detection found. See below |

### Trackers

Each tracker is an object with a `kind` and that kind's own fields beside it.
Today there are two kinds:

| `kind` | Fields |
|---|---|
| `git` | `repo_root`, `curr_branch`, `dirty`, `detached_head`, `repo_url`, `web_url`, `branches`, `commit_hash`, `contributors` |
| `unreal` | `project_root`, `project_name`, `uproject_path`, `engine_association`, `category`, `description`, `modules`, `plugins`, `vcs_provider` |

`contributors` is empty on purpose until deep detection exists.

## Errors

```json
{
  "schema": 1,
  "error": {
    "kind": "ambiguous",
    "message": "multiple matches found, which one did you mean?",
    "query": "app",
    "matches": [ {"id": "6c5d931e-…", "name": "app", "directory": "/Users/me/work/app", …} ]
  }
}
```

| `kind` | When | Extra fields |
|---|---|---|
| `not_found` | No project matches the query | `query` |
| `ambiguous` | Several projects match the query | `query`, and `matches`: the candidates as full [projects](#a-project), best first |
| `error` | Anything else: a stub command, a database that can't be opened, a database written by a newer app | none |

`message` is always there and is meant for a person. Branch on `kind`, not on
the message text. For `ambiguous`, pick a candidate and run the command again
with its `id`, or ask the user which one they meant.

An `error` whose message mentions "database is from a newer version of Project
Indexer" means the desktop app has migrated the database past what this CLI
understands. The fix is upgrading the CLI, not retrying.
