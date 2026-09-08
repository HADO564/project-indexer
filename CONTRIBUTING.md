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
- [Licensing and the CLA](#licensing-and-the-cla)
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

When you change the build, CI, the gates or packaging, record it in
[`docs/CHANGELOG-infra.md`](docs/CHANGELOG-infra.md) — the contributor-facing
companion to the root changelog. The rule is which reader is affected: someone
running the app, or someone working on the repository.

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
  src/tests/       its unit tests, mirroring the module tree
crates/cli/      indexer-cli — stub for the observer CLI (see ROADMAP.md)
src-tauri/       the desktop app: Tauri commands, adapters, startup
src/             SvelteKit frontend
  src/tests/       its vitest suites, mirroring the modules they cover
docs/            architecture, knowledgebase, checklist, handoffs
```

`indexer-core` is where the behaviour lives. `src-tauri` is a thin adapter over
it: each `#[tauri::command]` is a ~3-line pass-through to `ProjectService`.

**Tests live beside the tree they cover, not inside it.** A test for
`crates/core/src/domain/scan.rs` goes in `crates/core/src/tests/domain/scan.rs`;
one for `src/lib/views.ts` goes in `src/tests/lib/views.test.ts`. Same path,
different root.

This is not the Rust default — the language puts `#[cfg(test)] mod tests` at the
foot of each file — and it was changed on purpose, because the crate had reached
roughly as many lines of test as of code and files like `service.rs` (806 lines,
449 of them tests) had stopped reading as source.

Two consequences worth knowing before you add a test:

- **They are still unit tests, not integration tests.** They live under `src/`
  and are declared by `#[cfg(test)] mod tests;` in `lib.rs`, so they compile out
  of release builds and can reach `pub(crate)` internals. `crates/core/tests/`
  is reserved for genuine integration tests — `migrations.rs` is the one there,
  and it exercises only the public API.
- **Reaching an internal means `pub(crate)`, never `pub`.** A test needing a
  private item promotes it to `pub(crate)`, which is crate-internal and does not
  widen the published API. `scan::is_pruned`, `SqliteRepository::lock_conn` and
  `gitector::web_url` are the existing cases, each carrying a comment saying so.
  If a test seems to need full `pub`, that is a signal the test wants the public
  API instead.

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

**First, check it is a detector.** All four have to be true: its only input is a
directory path, its output is worth storing, it observes rather than acts, and
it is cheap — refs and manifests, not history walks or network calls. If any one
fails, what you have is a backend feature and it belongs in a service, a port,
or `platform/`. `docs/architecture.md` → *Detector, or backend feature?* has the
table and the worked examples.

**A detector is one directory plus one edit.** Copy the shape of
`crates/core/src/detectors/git/`:

```
crates/core/src/detectors/unity/
  mod.rs        declares the three below and re-exports them
  detector.rs   the `Detector` impl — kind() and detect()
  info.rs       the struct describing what it found
  error.rs      its error type, plus `impl From<UnityError> for DetectorError`
```

Then two one-line edits:

- `crates/core/src/detectors/mod.rs` — declare `pub mod unity;` and add it to
  `default_detectors()`. That is the only place detectors are registered.
- `crates/core/src/domain/tracker.rs` — add a `Unity(UnityInfo)` variant.

Nothing else changes. Two design choices are what keep it to that, and each is
worth knowing before you fight one:

- **`DetectorError` has no per-detector variant.** The `impl From<..>` in your
  `error.rs` boxes into `Other`, which keeps `?` working without that enum
  growing. `git/error.rs` is the example.
- **The frontend renders unfamiliar kinds already.** `lib/trackers.ts` infers
  each field's affordance from its name and value shape, `TrackerPanel` renders
  them, and `trackerColor(kind)` hashes a contrast-safe hue for a kind it has
  never seen. There is no TypeScript to write.

**`Tracker` stays an enum, deliberately.** It was briefly a generic
`{ kind, data }` struct so that adding a detector touched one file rather than
two. That was the wrong trade and was reverted: a detector is added a handful
of times in a project's life, but a tracker's contents are read for the life of
the project. The enum makes the compiler prove every kind is handled and catch
a renamed field at build time; a string-keyed map compiled cleanly and failed
silently. Do not reach for the map again to save one line.

Do not add a `Tracker` variant without a detector behind it. Placeholder
variants for Unity and Blender existed once and were removed for that reason.

Put the tests in `crates/core/src/tests/detectors/unity/detector.rs`, mirroring
the source path — `Gitector` (11 tests) and `UnrealDetector` (10) are the model.

Pick the `kind` string once and never rename it: it is the key every project
record stores the payload under. The two that predate the generic `Tracker`
use their old enum variant names, `"Git"` and `"Unreal"`.

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

## Licensing and the CLA

Project Indexer is licensed under the [Functional Source License](LICENSE)
(`FSL-1.1-ALv2`). Anyone may use it for free — personally, inside a company, for
teaching or for research — and every version converts to the Apache License 2.0
two years after it is published. The only prohibited use is shipping it, or a
close substitute for it, as a commercial product that competes with this one.

That two-year conversion is a promise the project can only keep if it is able to
relicense all of its code. Copyright is automatic and stays with whoever wrote
the lines, so without an agreement in place every contributor would hold a veto
over the licence, forever. The [CLA](CLA.md) is what avoids that. You keep the
copyright in your work; you grant the project permission to relicense it.

**You do not have to do anything up front.** Open your pull request as normal.
A bot will comment on it asking you to reply with one sentence agreeing to the
CLA, record that against your GitHub account, and never ask you again.

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
