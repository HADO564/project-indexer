# Contributing to Project Indexer

Thanks for taking an interest. This document covers getting a working
development environment, the checks your change has to pass, and the handful of
structural rules the codebase enforces on purpose.

## Contents

- [Getting set up](#getting-set-up)
- [Running the app](#running-the-app)
- [The checks](#the-checks)
- [Project layout](#project-layout)
- [Rules the codebase enforces](#rules-the-codebase-enforces)
- [Adding a detector](#adding-a-detector)
- [Commits and pull requests](#commits-and-pull-requests)
- [Where the documentation lives](#where-the-documentation-lives)

## Getting set up

You need [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20 or
newer, and [pnpm](https://pnpm.io) — `corepack enable` provides it, and the exact
version is pinned by `packageManager` in `package.json`.

**On Linux, install the system packages first** — Tauri needs the WebKitGTK
webview and an appindicator library for the tray. The per-distribution lists are
in the README's [Linux notes](README.md#linux-notes). Skipping the appindicator
package used to kill the app at startup; it now degrades to "no tray" with a
message, but you still want it.

```sh
pnpm install
```

## Running the app

```sh
pnpm run tauri dev     # development, with hot reload
pnpm run tauri build   # installers under target/release/bundle
```

Use one of those two. A plain `cargo build` compiles the binary with the dev
server's URL baked in, so launching `target/debug/project-indexer` on its own
reports "Could not connect to localhost: Connection refused" — it is waiting for
a Vite server that isn't running. That is not a crash, and it catches people out.

## The checks

CI runs all of these on Linux and Windows for every push and pull request, so
run them before opening one.

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets
cargo test --workspace
pnpm run check                     # svelte-check
pnpm test                          # vitest
pnpm run build
```

Or install the pre-commit hook once and let it run them for you:

```sh
git config core.hooksPath .githooks
```

It runs the same commands, in the same order, but only the ones your staged
files can affect — a docs-only commit costs nothing, so there is no reason to
reach for `--no-verify` out of habit. `git commit --no-verify` skips it when you
need to, and `git config --unset core.hooksPath` removes it entirely. If you
change the gates in `.github/workflows/ci.yml`, change `.githooks/pre-commit`
too, or "it passed locally" stops meaning anything.

Two known-noise baselines, so you can tell your output from the existing state:

- **clippy** has one standing warning, `module has the same name as its
  containing module`. Anything beyond that is yours.
- **`pnpm run check`** reports 0 errors and 8 warnings, all
  `state_referenced_locally` in `EditProjectForm.svelte`. They are a documented
  false positive — see `PI-003` in [`docs/KNOWN-ISSUES.md`](docs/KNOWN-ISSUES.md).

**Neither CI nor the hook launches the app.** They compile it and test it, which
says nothing about whether the window actually appears — `PI-005` compiled,
passed every test, and still exited before showing a window. If your change
touches startup, the tray, or anything platform-specific, run the real thing.

## Project layout

```
crates/core/     indexer-core — all domain logic, orchestration, persistence
crates/cli/      indexer-cli — stub for the observer CLI (see ROADMAP.md)
src-tauri/       the desktop app: Tauri commands, adapters, startup
src/             SvelteKit frontend
docs/            architecture, knowledgebase, checklist, handoffs
```

`indexer-core` is where the behaviour lives. `src-tauri` is a thin adapter over
it: each `#[tauri::command]` is a ~3-line pass-through to `ProjectService`.

## Rules the codebase enforces

These are deliberate, and two of them are enforced by the compiler rather than by
review.

1. **`indexer-core` must not depend on Tauri.** A `use tauri::` inside `core`
   fails to build. This is what keeps a second frontend (the planned CLI, and the
   separate devmon app) possible without touching the backend. If a change seems
   to need Tauri in `core`, the boundary is in the wrong place — say so in the PR
   rather than working around it.

2. **Detectors are independent and unordered.** A directory can legitimately be a
   git repository *and* an Unreal project. A detector that fails is reported as
   failed, never as "nothing found", so a malformed `.uproject` is never silently
   mistaken for "not an Unreal project".

3. **`projects.db` is a cross-app contract.** Another application (devmon) is
   planned to attach it read-only. The `meta` table and the `ProjectReader` port
   exist for that reason — don't remove them. The recorded decision is in
   [`docs/architecture.md`](docs/architecture.md).

4. **Schema changes are numbered migrations.** Bump `CURRENT_SCHEMA_VERSION`, add
   a `user_version` step, and ship a test with it. A newer binary opening an older
   database is the normal case once the app self-updates. `SqliteRepository::open`
   already refuses a database written by a *newer* binary.

## Adding a detector

The generic path exists so that a new tracker needs no frontend code at all:

1. Add the info model to `core::domain`, and a variant to `Tracker`.
2. Implement `Detector` — `kind() -> &'static str` and
   `detect(&Path) -> Result<Option<Tracker>, DetectorError>`.
3. Register it in `core::detectors::registry::default_detectors()`. That is the
   one place detectors are registered.
4. Add the variant to `src/lib/types.ts`.

The UI picks it up automatically: `lib/trackers.ts` infers field types from key
names and value shapes, `TrackerPanel` renders them, and `trackerColor(kind)`
assigns a contrast-safe hue. Add unit tests alongside the detector — the existing
ones (`Gitector`, 11; `UnrealDetector`, 10) are the model to follow.

Do not add a `Tracker` variant without a detector behind it. Placeholder variants
for Unity and Blender existed once and were removed for that reason.

## Commits and pull requests

- **Conventional-commit prefixes**: `feat:`, `fix:`, `docs:`, `refactor:`,
  `build:`, `ci:`, `style:`, `test:`. A scope is welcome — `fix(tray): …`.
- **Explain why in the body.** The history here is used as documentation; a
  commit that fixes something non-obvious should say what the cause was, not just
  what changed.
- **Branch off `main`** and open a PR against it. Keep unrelated changes in
  separate commits — a formatting sweep should not ride along with a behaviour
  change.
- **Update the docs in the same PR.** `docs/checklist.md` for feature status,
  `docs/accomplishments.md` for what landed, `CHANGELOG.md` under `[Unreleased]`
  for anything user-visible.

## Where the documentation lives

| Document | What it is for |
|---|---|
| [`README.md`](README.md) | what the app is, install, Linux notes |
| [`docs/USAGE.md`](docs/USAGE.md) | how to actually use it, feature by feature |
| [`ROADMAP.md`](ROADMAP.md) | what is planned, and what was deliberately declined |
| [`docs/architecture.md`](docs/architecture.md) | invariants, recorded decisions, quality backlog |
| [`docs/knowledgebase.md`](docs/knowledgebase.md) | how each piece works, module by module |
| [`docs/checklist.md`](docs/checklist.md) | feature status |
| [`docs/accomplishments.md`](docs/accomplishments.md) | dated log of what landed |
| [`docs/KNOWN-ISSUES.md`](docs/KNOWN-ISSUES.md) | triaged issues from platform passes |
| [`docs/handoffs/`](docs/handoffs/) | briefings for work not yet started |

Read `docs/architecture.md` before a structural change. It records not only what
was decided but what was **considered and declined**, which will save you
proposing something that has already been ruled out with reasons.
