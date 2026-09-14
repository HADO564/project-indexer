# Project Indexer — Docs

The repository holds two products over one shared core: the **desktop app** and
the **command-line tool**. Each keeps its own docs; what they share lives at the
top of this directory.

Start with [`app/USAGE.md`](app/USAGE.md) if you want to *use* the app, or
[`../CONTRIBUTING.md`](../CONTRIBUTING.md) if you want to work on either product.

## Desktop app — [`app/`](app/)

- [`app/USAGE.md`](app/USAGE.md) — how to use the app, feature by feature.
- [`app/ROADMAP.md`](app/ROADMAP.md) — the app's detailed plans.
- [`app/checklist.md`](app/checklist.md) — what's done and what's still open, as a checklist. Also carries the shared `indexer-core` features that shipped through the app.
- [`app/KNOWN-ISSUES.md`](app/KNOWN-ISSUES.md) — issue triage from platform passes: the Linux build-and-run pass, and macOS (PI-007).
- [`../CHANGELOG.md`](../CHANGELOG.md) — what shipped in each app release.

## Command-line tool — [`cli/`](cli/)

- [`cli/ROADMAP.md`](cli/ROADMAP.md) — the CLI's detailed plans.
- [`cli/checklist.md`](cli/checklist.md) — its milestones, and what's done.
- [`superpowers/specs/2026-09-14-cli-design.md`](superpowers/specs/2026-09-14-cli-design.md) — the design contract: structure, the command layer, releases.
- [`../crates/cli/README.md`](../crates/cli/README.md) and [`../crates/cli/CHANGELOG.md`](../crates/cli/CHANGELOG.md) — the crate, and what shipped in each CLI release (`cli-v*` tags).
- A usage guide joins these once there are commands to document.

## Shared — the repository and `indexer-core`

- [`knowledgebase.md`](knowledgebase.md) — how the pieces work and why, kept current (not a dated snapshot).
- [`architecture.md`](architecture.md) — the shape of the system: invariants worth protecting, detection semantics, recorded decisions, and a prioritized quality backlog.
- [`accomplishments.md`](accomplishments.md) — dated log of what's been completed, across both products.
- [`CHANGELOG-infra.md`](CHANGELOG-infra.md) — contributor-facing log of build, CI, tooling, packaging and release-process changes.
- [`handoffs/`](handoffs/) — briefings for work that hasn't started yet: what exists, what was decided, and what is still open.
  - [`handoffs/2026-09-04-observer-cli.md`](handoffs/2026-09-04-observer-cli.md) — the briefing the CLI design builds on: the backend, the shared database, and the questions it left open.
  - [`handoffs/2026-09-04-plugins.md`](handoffs/2026-09-04-plugins.md) — the plugin initiative, and the trust boundary that gates it. Also carries the state of the tree as of 2026-09-04.
- [`superpowers/`](superpowers/) — design specs and implementation plans, one pair per initiative.

Outside this directory:

- [`../README.md`](../README.md) — what the app is, install, Linux notes.
- [`../ROADMAP.md`](../ROADMAP.md) — the overview: headline plans for both products, what they share, and what was declined.
- [`../CONTRIBUTING.md`](../CONTRIBUTING.md) — development setup, the checks, the structural rules the codebase enforces, and how each product is released.
- [`../SECURITY.md`](../SECURITY.md) — reporting a vulnerability, and what's in scope.
