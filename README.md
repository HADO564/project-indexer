# Project Indexer

A desktop app that keeps track of the projects scattered across your disk — what
they are, where they are, and what opens them.

[![CI](https://github.com/HADO564/project-indexer/actions/workflows/ci.yml/badge.svg)](https://github.com/HADO564/project-indexer/actions/workflows/ci.yml)

Point it at a directory and it works out what kind of project lives there — a
git repository, an Unreal Engine project — and records the details. From then on
it is one list of everything you work on, with the branch you left it on, the
editor you open it with, and a single click to get back into it.

Built with [Tauri 2](https://tauri.app), a Rust backend, and a SvelteKit
frontend. Windows, macOS, and Linux.

---

## Contents

- [Why](#why)
- [Features](#features)
- [What it detects](#what-it-detects)
- [Install](#install)
- [Where your data lives](#where-your-data-lives)
- [Architecture](#architecture)
- [Building from source](#building-from-source)
- [Linux notes](#linux-notes)
- [Project layout](#project-layout)
- [Roadmap](#roadmap)
- [License](#license)

---

## Why

Projects accumulate. They end up under `~/code`, `D:\Projects`, a Documents
folder, an external drive — some are git repos, some are game projects, some are
just a folder with a few files. Finding the one you want means remembering where
you put it and which app opens it.

Project Indexer keeps that in one place. It does not move your files, manage your
repositories, or replace your editor. It is an index: a list that knows where
everything is and how to open it.

## Features

**Tracking** — register a directory with a name, description, tags, notes, and a
client. Names are suggested automatically from the git remote (or the folder
name) when you browse to a directory.

**Detection** — project type is detected on add and can be re-run at any time.
Each detected tracker gets its own tab in the project view with its full details.

**Opening** — launch a project in a specific application or the system file
explorer. The app picker is populated from your installed applications: Start
Menu shortcuts and registry App Paths on Windows, `.desktop` entries on Linux.

**Organising** — favourites, tags, and sorting by name or last-opened, in either
direction.

**Housekeeping** — a recycle bin with restore, "untrack" to forget a project
without touching its files, and a marker on any project whose directory has been
deleted or moved out from under it.

**Background operation** — closing the window hides the app to the system tray
rather than quitting. Left-click the tray icon to bring it back; right-click for
Show and Quit.

## What it detects

| Tracker | Detected from | Reported |
|---|---|---|
| **Git** | a repository at or above the directory | current branch, dirty state, detached HEAD, branch list, current commit, remote URL and its browser-openable form |
| **Unreal Engine** | a `.uproject` file in the directory | project name, engine association, category, description, modules, enabled plugins, configured source-control provider |

Detectors are independent and unordered — a directory can legitimately be both
at once, and each is reported separately. A detector that fails is shown as
failed rather than silently reported as "nothing found", so a malformed
`.uproject` is never mistaken for "not an Unreal project".

Adding a new one (Unity, Godot, …) means writing the detector and registering it
in one place; the UI renders any tracker generically, so no frontend work is
needed.

## Install

Download the installer for your platform from the
[latest release](https://github.com/HADO564/project-indexer/releases/latest):

| Platform | File |
|---|---|
| Windows | `project-indexer_<version>_x64-setup.exe` or `_x64_en-US.msi` |
| macOS (Apple Silicon) | `project-indexer_<version>_aarch64.dmg` |
| macOS (Intel) | `project-indexer_<version>_x64.dmg` |
| Linux (Debian/Ubuntu) | `project-indexer_<version>_amd64.deb` |
| Linux (Fedora/RHEL) | `project-indexer-<version>-1.x86_64.rpm` |
| Linux (portable) | `project-indexer_<version>_amd64.AppImage` |

Builds are unsigned, so Windows SmartScreen and macOS Gatekeeper will warn on
first launch.

## Where your data lives

Projects are stored in a SQLite database, `projects.db`, in the platform config
directory:

| Platform | Path |
|---|---|
| Windows | `%APPDATA%\com.shaer.project-indexer\` |
| macOS | `~/Library/Application Support/com.shaer.project-indexer/` |
| Linux | `~/.config/com.shaer.project-indexer/` |

Writes are synchronous and transactional — nothing is buffered and lost if the
app is killed. Your project directories themselves are never modified except by
the explicit "delete directory" action.

## Architecture

The backend is deliberately split so the GUI is one frontend rather than the only
one:

```
  src-tauri (GUI)              crates/cli (planned)
  thin #[tauri::command]       a second frontend, same backend
  pass-throughs + adapters
            │                            │
            └──────────┬─────────────────┘
                       ▼
         crates/core  «indexer-core»
         no tauri dependency — enforced by the crate graph

         application/  ProjectService — all orchestration
         ports/        ProjectRepository · AppLauncher
         domain/       Project · Tracker · naming · sorting
         detectors/    Detector · DetectorRunner · registry
         platform/     filesystem · installed-app discovery
         infra/        SqliteRepository
                       │
                       ▼
              projects.db (SQLite, WAL)
```

`indexer-core` holds every piece of domain logic, orchestration, and persistence,
and cannot import Tauri — the compiler enforces it. The Tauri layer is a thin
adapter: each command is a few lines that call a service method. A command-line
frontend can therefore be added without changing the backend at all.

See [`docs/architecture.md`](docs/architecture.md) for the invariants this
protects and the reasoning behind them.

## Building from source

Requires [Rust](https://rustup.rs) (stable), [Node.js](https://nodejs.org) 20+,
and [pnpm](https://pnpm.io) (`corepack enable` will provide it). On Linux, see
[Linux notes](#linux-notes) for the system packages Tauri needs first.

```sh
pnpm install
pnpm run tauri dev     # run in development
pnpm run tauri build   # produce installers under target/release/bundle
```

Run it through one of those two commands, not by launching the built binary
directly. A plain `cargo build` bakes in the dev server's URL, so
`target/debug/project-indexer` started on its own shows "Could not connect to
localhost: Connection refused" — it is waiting for the Vite server that
`tauri dev` would have started. `tauri build` is what produces a binary that
serves the bundled frontend.

Tests and checks:

```sh
cargo test --workspace     # Rust: domain, detectors, service, repository
cargo clippy --workspace
cargo fmt --all --check
pnpm run check             # svelte-check
pnpm test                  # frontend unit tests
```

CI runs all of these on Windows and Linux for every push and pull request.

## Linux notes

### System packages

Tauri needs the WebKitGTK-based webview and its build dependencies.

Debian / Ubuntu (24.04 and newer; on 22.04 the webkit package is
`libwebkit2gtk-4.0-dev` and `libsoup2.4-dev`):

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Arch:

```sh
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module librsvg patchelf libayatana-appindicator
```

Fedora:

```sh
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel patchelf
sudo dnf group install "C Development Tools and Libraries"
```

`libayatana-appindicator` is what the system tray loads at runtime (the Debian
and Fedora lines above already cover it). Without it the app still runs, but it
has no tray icon, and closing the window quits instead of hiding to it.

Nothing is distro-specific at runtime: app discovery and the NVIDIA workaround
below both key off standard paths rather than package names.

### NVIDIA proprietary driver

On the proprietary NVIDIA driver, WebKitGTK's DMABUF renderer can't allocate GBM
buffers. The window comes up blank (`Failed to create GBM buffer`) or, on
Wayland, the app dies at startup with `Error 71 (Protocol error) dispatching to
Wayland display`.

The app detects that driver at startup and sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` for itself, so no manual setup is needed. Mesa,
nouveau, and everything else keep the accelerated path. To override the
detection, set the variable yourself — the app leaves an existing value alone:

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=0 pnpm run tauri dev  # force the DMABUF renderer on
```

WebKit reads that value rather than just checking whether it is set, so `=0` is
what reinstates the broken path — useful for confirming the driver is the cause,
since on an affected machine it brings the startup crash straight back.

### "Open with" app discovery

The app picker reads `.desktop` files from the standard XDG application
directories (`$XDG_DATA_HOME`, `$XDG_DATA_DIRS`, falling back to
`~/.local/share/applications` and `/usr/share/applications`), so it lists the
same apps your desktop environment's launcher would show — including Flatpak and
Snap, whose exported entries live in those same directories.

Entries are launched with their full `Exec` command line, and the project
directory replaces the entry's `%f`/`%u` placeholder (or is appended when there
isn't one). That's what lets wrapper-based entries work, such as Flatpak's
`flatpak run … --file-forwarding <app-id> @@u %u @@`, where the path has to land
between the file-forwarding markers.

## Project layout

```
crates/core/      indexer-core — domain, services, ports, detectors, SQLite
crates/cli/       stub for the planned command-line frontend
src-tauri/        the Tauri desktop app: commands, adapters, tray, wiring
src/              SvelteKit frontend
docs/             architecture, knowledgebase, checklist, known issues
```

Further reading: [`docs/architecture.md`](docs/architecture.md) for the shape of
the system and the decisions behind it, [`docs/knowledgebase.md`](docs/knowledgebase.md)
for how the pieces actually work, and [`CHANGELOG.md`](CHANGELOG.md) for what
changed between releases.

## Roadmap

- A command-line frontend that observes shell activity and registers projects
  automatically, sharing the same database as the GUI.
- More detectors — Unity, Blender, and others.
- In-app update notifications with releases pulled from GitHub.
- Git contributor listing (currently deliberately deferred; it needs a full
  history walk and a cache).

## License

MIT.
