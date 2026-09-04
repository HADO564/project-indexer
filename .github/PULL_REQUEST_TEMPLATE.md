## What this changes

<!-- What the change does, and why. If it fixes something non-obvious, say what
     the cause was — the history here is used as documentation. -->

Fixes #

## How it was verified

<!-- Which checks you ran, and — if this touches startup, the tray, launching
     apps, or anything platform-specific — that you actually ran the app. CI
     compiles and tests the Linux target but never launches it, so a green CI run
     says nothing about whether the window appears. -->

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets` (baseline: 1 warning, `module-inception`)
- [ ] `cargo test --workspace`
- [ ] `pnpm run check` (baseline: 0 errors, 8 `EditProjectForm` warnings — PI-003)
- [ ] `pnpm test`
- [ ] Ran the app

Platforms tried: <!-- Windows / Linux / macOS -->

## Docs

- [ ] `CHANGELOG.md` under `[Unreleased]`, if this is user-visible
- [ ] `docs/checklist.md`, if this changes feature status
- [ ] `docs/accomplishments.md`, for what landed
- [ ] Not needed

## Notes for the reviewer

<!-- Anything deliberately left out, a decision you were unsure about, or a
     structural rule in CONTRIBUTING.md this bumps into. -->
