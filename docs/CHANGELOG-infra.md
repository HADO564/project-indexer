# Infrastructure Changelog

Changes to how the project is **built, tested, packaged and worked on**. This is
the contributor-facing companion to the root
[`CHANGELOG.md`](../CHANGELOG.md), which records what changed for people *using*
the app.

**The dividing line:** if it changes what someone running the app experiences, it
goes in the root changelog. If it changes what someone working on the repository
experiences — the toolchain, CI, the gates, packaging, the local dev loop — it
goes here. A few things legitimately belong in both; duplicate them rather than
picking one and hoping the reader looks in the other place.

Entries are dated rather than versioned, because infrastructure does not ship on
the app's release cadence — a contributor gets it when they pull, not when a
version is tagged. Newest first.

---

## 2026-09-05

### Changed

- **Bumped the GitHub Actions still running on the Node 20 shim.**
  `actions/checkout` v4 → v7, `actions/setup-node` v4 → v7,
  `pnpm/action-setup` v4 → v6. GitHub was already forcing all three onto Node 24
  and warning about it on every run; they worked, but a forced shim is a
  transitional state and its removal would have surfaced at the worst possible
  time. All three are input-compatible with how they are used here.

  Deliberately left alone: `Swatinem/rust-cache@v2` and
  `tauri-apps/tauri-action@v0` already run on Node 24 despite the low version
  numbers, so neither is shimmed. `tauri-action` v1 exists but drops inputs
  (`distPath`, `appName`, `includeDebug`, `assetNamePattern` among them) for no
  benefit this project would collect. `dtolnay/rust-toolchain@stable` is not a
  Node action.

  Note that `release.yml` only runs on a `v*` tag push, so its share of this
  change was verified by reading each action's interface at the new tag rather
  than by executing it.

## 2026-09-04

### Added

- **A pre-commit hook** at `.githooks/pre-commit`, opt in with
  `git config core.hooksPath .githooks`. It runs the same gates as
  `.github/workflows/ci.yml`, in the same order, before the commit exists.

  Only the gates the staged files can affect are run, which is the point: a
  docs-only commit costs nothing, so there is no reason to reach for
  `--no-verify` out of habit and end up skipping it when it matters.

  **`core.hooksPath` is local git config and is not carried by the repository.**
  The hook file is committed; the setting that activates it is not. Every clone
  needs the command run once, including on a second machine.

  It does **not** catch whether the app starts — see the next entry.

### Known issues

- **`PI-006`: `tauri build` cannot produce an AppImage on Arch.** It exits 1
  after successfully building the binary, the `.deb` and the `.rpm`. Two
  unrelated incompatibilities, neither of them this project's: linuxdeploy's
  bundled `strip` does not understand the `SHT_RELR` sections modern Arch
  libraries carry, and its GTK plugin copies a gdk-pixbuf loader directory that
  no longer exists now that the loaders are built into the library.

  Build `--bundles deb,rpm` locally. The release workflow builds AppImages on
  `ubuntu-22.04`, where neither problem occurs, so published artifacts are
  unaffected. Full detail in [`KNOWN-ISSUES.md`](KNOWN-ISSUES.md).

- **Neither CI nor the pre-commit hook launches the app.** `PI-005` compiled,
  passed every test, and still exited before showing a window. Recorded here
  because it is a permanent property of the setup rather than a one-off: for this
  project, "CI is green" and "it starts" are separate claims, and only one of them
  is automated.

## 2026-09-03

### Added

- **CI and release workflows.** CI runs `cargo fmt`, `clippy` and `test` on
  ubuntu-22.04 and windows-latest, plus the frontend `check` / `test` / `build`,
  on every push to `main` and every pull request. This was also the first time
  the Linux target was compiled at all — the core refactor had moved the
  `#[cfg(target_os = "linux")]` launch code across crates on a Windows machine.

  Release builds Tauri bundles for Windows, macOS (arm64 and x64) and Linux on a
  `v*` tag via `tauri-apps/tauri-action`, and opens a **draft** GitHub Release
  rather than publishing one.

## 2026-09-02

### Changed

- **Settled the JavaScript toolchain on pnpm.** There was a brief detour to npm
  while chasing a blank-white-screen on `tauri dev`; the real cause was a mixed
  install — `beforeDevCommand` ran `pnpm dev` against an npm-installed
  `node_modules`, so pnpm relocated it to `node_modules/.ignored` and did a
  partial install, leaving Vite unable to resolve `@sveltejs/kit`'s runtime.

  The resolution is pnpm everywhere: `pnpm-lock.yaml` is the lockfile, there is
  no `package-lock.json`, and `packageManager` is pinned in `package.json`. Do
  not mix package managers in this repository — that is the failure mode it
  produces, and it does not look like a toolchain problem when it happens.

### Added

- **Cargo workspace** covering `src-tauri`, `crates/core` (`indexer-core`) and
  `crates/cli` (`indexer-cli`). The tracked `Cargo.lock` lives at the workspace
  root and `/target` is gitignored — a stale `src-tauri/target` from before this
  change is dead weight and safe to delete, which is worth knowing if you are
  short on disk.

---

_This log starts on 2026-09-02, the first change that was clearly
infrastructure rather than product. Earlier history is in `git log`._
