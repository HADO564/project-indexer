# Handoff — the observer CLI (Spec 2)

**Date:** 2026-09-04
**Status:** ready to start. Nothing is blocked; the backend seams exist.
**Prerequisite:** the frontend-agnostic-core refactor, shipped in v0.1.1.

This is the briefing for the next initiative, not its design. It records what
exists, what was already decided, and — most importantly — **what was
deliberately left open**. Read it, then run the brainstorming skill to settle
the open questions before writing a spec.

---

## 1. The idea in one paragraph

`indexer` wraps another command, runs it untouched, and *notices* what it did.

```
indexer git init
  → runs `git init` with inherited stdio and propagates its exit code
  → notices: the working directory is now a git repository
  → records: ensure a project exists for this directory, refresh its trackers
  → exits with git's exit code
```

It never reimplements `git`, `gh`, `mkdir`, or anything else. It spawns the real
tool, then pattern-matches on `argv` + working directory + exit code and writes
inferred project facts through the same `ProjectService` the GUI uses. The user's
framing: *"The CLI does not call these functions itself, it just sees that these
commands were issued, and records that the directory of the project is friction
etc. It does not intervene."*

Because both frontends open the same SQLite database, installing the CLI later
"connects" it to the GUI automatically — no pairing, no IPC.

## 2. Where things stand

**Shipped (v0.1.1, published):** the GUI runs entirely on `indexer-core`, a
library crate the compiler forbids from importing Tauri. All domain logic,
orchestration, and persistence live there. The Tauri layer is ~3-line command
pass-throughs plus one launcher adapter.

That was the whole point of the refactor: **you should not need to change the
backend to add this CLI.** If you find yourself modifying `ProjectService` for
reasons other than genuinely new behaviour, stop and question it — that would
mean the boundary was drawn wrong, which is worth knowing.

**Health at handoff:** `main` green on Windows + Linux in CI, 94 Rust tests, 14
frontend tests, clippy at a known 2-warning baseline, `cargo fmt` clean.

## 3. The backend you're building on

Everything below is public API on `indexer-core` today. None of it needs to
change for the CLI.

```rust
// The one entry point. Construct it, call methods, done.
ProjectService::new(
    repo:      Arc<dyn ProjectRepository>,   // SqliteRepository
    launcher:  Arc<dyn AppLauncher>,         // you supply a CLI-side impl
    detectors: Arc<DetectorRunner>,          // DetectorRunner::default()
)
```

| Method | Use from the CLI |
|---|---|
| `ensure_project(&str) -> Result<Project>` | **the observer's main verb** — get-or-create for a directory |
| `find_by_directory(&str) -> Result<Option<Project>>` | "do we already know this directory?" |
| `refresh_trackers(&str) -> Result<Project>` | re-run detection after `git init` / `git clone` |
| `preview_detection(&str) -> Vec<Tracker>` | detect without persisting |
| `list / list_deleted / list_favorites(SortOptions)` | `indexer list` |
| `get / create / update / delete / restore / untrack` | plain subcommands |
| `open / open_in_explorer(&str)` | `indexer open <id>` |
| `inspect(&str, Option<&str>)` | `indexer show <id>` — per-detector status |
| `delete_directory(&str, bool)` | destructive; gate behind confirmation |

**Ports you implement or reuse:**

- `ProjectRepository` — reuse `SqliteRepository::open(&Path)`. Do **not** write
  a second implementation.
- `AppLauncher` — the GUI's `OpenerLauncher` lives in `src-tauri` and depends on
  `tauri-plugin-opener`, so the CLI needs its own. Most of the logic is already
  Tauri-free in `indexer_core::platform::app_discovery` (`open_with_command` for
  Linux `.desktop` command lines, `open_with_app_available`). If `indexer open`
  is out of scope for v1, a stub that returns `Err` is fine.
- `ProjectReader` — the read-only half, if you want a narrower dependency.

**Also useful:** `indexer_core::domain::naming::suggest_project_name(&[Tracker], &str)`
is the same inference the GUI uses (git remote's repo name, else folder name).
`ensure_project` already calls it.

## 4. Seams already put in place for you

These were added during the refactor *specifically* for this work:

- `ProjectService::find_by_directory` and `ensure_project` — neither has a GUI
  caller. They exist for you.
- `projects.directory_normalized` — an indexed column, so directory lookup is
  not a table scan.
- `ProjectReader` split out of `ProjectRepository`.
- `crates/cli/` — a workspace member with a stub `main.rs`. It currently has
  **no dependency on `indexer-core`**; adding that is step one, and it is also
  what starts enforcing the boundary from this side.
- `domain::naming` — name inference moved out of the Svelte component into core
  so both frontends share it.

## 5. The shared database — read this before writing code

Both frontends must open **the same file** or the entire premise breaks silently
(you get two databases and neither tool sees the other's projects).

The GUI resolves it as `app.path().app_config_dir().join("projects.db")`, which
lands at:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\com.shaer.project-indexer\projects.db` |
| macOS | `~/Library/Application Support/com.shaer.project-indexer/projects.db` |
| Linux | `~/.config/com.shaer.project-indexer/projects.db` |

The CLI has no Tauri, so it must derive this itself — almost certainly
`dirs::config_dir().join("com.shaer.project-indexer")`, which matches Tauri's
behaviour on all three platforms. **Verify it per-platform rather than assuming**;
a mismatch fails quietly, which is the worst failure mode available here. Worth a
test that asserts the CLI's resolved path equals the GUI's on the current OS.

Other properties already handled for you:

- **WAL is on**, with `busy_timeout=5000`. Two processes reading and writing
  concurrently is expected and supported; readers don't block the writer.
- **Writes are synchronous and transactional.** Nothing is buffered.
- **`SqliteRepository::open` refuses a database from a newer binary** (the
  `user_version` skew guard) and returns a clear message. If the CLI and GUI are
  ever on different versions, the older one fails loudly instead of corrupting.
  Surface that message; don't swallow it.
- A `meta(key, value)` table carries `app` and `schema_version` for external
  readers.

## 6. Open design decisions — settle these in brainstorming

Nothing below was decided. Do not treat the examples in §1 as settled scope.

1. **Which recognizers ship first?** `mkdir <name>`, `git init`, `git clone
   <url>`, `gh repo create`, `cargo new` were illustrative examples, not a
   committed list. What is the minimum set that makes this genuinely useful?

2. **Which directory gets registered?** `indexer mkdir friction` runs in `~/code`
   and creates `~/code/friction` — the project is the *new* directory, not the
   cwd. `indexer git init` in `~/code/friction` registers the cwd. The rule for
   deriving "the project directory" from `argv` + cwd is per-recognizer and needs
   writing down.

3. **`indexer cd` cannot work.** `cd` is a shell builtin; a subprocess can't
   change its parent's directory. Decide whether that matters, and whether any
   shell integration (a `direnv`-style hook) is in scope — the user's examples
   all used the explicit `indexer` prefix, which is the tractable model.

4. **Duplicate names.** `ensure_project` creates via `ProjectService::create`,
   which rejects a name already in use. Auto-registering `~/code/api` fails if
   any project is called `api`. Does the CLI disambiguate (`api-2`, a
   parent-qualified name), surface the conflict, or register with a fallback
   name? This is documented as the CLI's decision in `ensure_project`'s doc
   comment.

5. **Are plain subcommands in v1?** `indexer list` / `show` / `add` / `open` are
   straightforward over the service, but they are separate work from the
   observer. Ship both, or observer-first?

6. **Where do recognizers live?** The spec leaned toward `crates/cli` (they are
   CLI-only). A `CommandObserver` trait in core — `(argv, cwd, exit) ->
   Vec<ProjectFact>` — mirroring `Detector` was floated as "likely" but is not
   decided. Putting them in core only pays off if something else consumes them.

7. **Should a running GUI reflect CLI writes live?** Explicitly a nice-to-have,
   not a requirement. Would need DB-file watching.

8. **Output conventions.** Exit-code passthrough is settled (the wrapped
   command's code wins). Still open: what the observer prints (nothing? a
   one-line note to stderr?), `--quiet`, and whether recording failures are
   surfaced or silent.

9. **Distribution.** Spec 1's "App updates" section designed — but did not
   build — a slim CLI-only binary, `indexer self-update` via the `self_update`
   crate, and a GUI action that downloads the CLI on demand and puts it on PATH.
   Decide how much of that lands with the CLI itself.

## 7. Constraints and gotchas

- **Package manager is pnpm.** `pnpm install`, `pnpm run tauri dev`. There is no
  `package-lock.json`; `packageManager` is pinned in `package.json`.
- **Windows:** the linker cannot overwrite `project-indexer.exe` while it runs.
  Kill the app before `cargo build`.
- **Linux is a supported target** and CI compiles it on every push. The
  `#[cfg(target_os = "linux")]` app-discovery code is ~450 lines and only CI
  proves it builds — don't assume a Windows-only green run means anything for it.
- **Clippy baseline is 2 warnings** (`module-inception`, one `sort_by_key`).
  Anything beyond that is new and yours.
- **Commit trailer:** every commit ends with
  `Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>`.
- **`crates/cli` is at version 0.1.1** with the workspace but has no deps yet.

## 8. Suggested route

The refactor used brainstorming → spec → writing-plans → subagent-driven
development, and it worked well: nine tasks, each independently reviewed, one
whole-branch review, one fix wave. Same route is a reasonable default here.

1. **Brainstorm** the open questions in §6. This is architectural — new
   subsystem, new binary — so it warrants the full treatment, ending in a spec at
   `docs/superpowers/specs/`.
2. **Plan** from the spec.
3. **Execute**, ideally on a feature branch. Merge with a green CI run on both
   platforms.

Start by adding `indexer-core` as a dependency of `crates/cli` and getting
`indexer list` to print real rows from the shared database. That single vertical
slice proves the whole premise — same database, no backend changes — in about
twenty lines, and everything else builds on it.

## 9. Reference map

| Document | What it gives you |
|---|---|
| [`../architecture.md`](../architecture.md) | invariants worth protecting, detection semantics, recorded decisions, quality backlog |
| [`../knowledgebase.md`](../knowledgebase.md) | how each piece actually works, module by module |
| [`../superpowers/specs/2026-09-02-frontend-agnostic-core-design.md`](../superpowers/specs/2026-09-02-frontend-agnostic-core-design.md) | the refactor's spec; §"Spec 2 preview" and §"App updates" are the CLI's prehistory |
| [`../superpowers/plans/2026-09-02-frontend-agnostic-core.md`](../superpowers/plans/2026-09-02-frontend-agnostic-core.md) | how the refactor was executed, if you want the task-shaping precedent |
| [`../checklist.md`](../checklist.md) | feature status |
| [`../../CHANGELOG.md`](../../CHANGELOG.md) | what shipped in v0.1.0 / v0.1.1 |

**Related future work:** a separate app, **devmon** (an activity tracker), is
planned to attach to this same `projects.db` read-only for work attribution. The
cross-app contract is in the refactor spec under "Cross-app compatibility". It
shapes nothing the CLI must do, but it is why `ProjectReader` and the `meta`
table exist — don't remove them.
