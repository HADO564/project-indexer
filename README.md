# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Linux setup

Install the system packages Tauri needs to build the WebKitGTK-based webview.

Arch:

```sh
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module librsvg patchelf
```

Debian / Ubuntu (24.04 and newer; on 22.04 the webkit package is
`libwebkit2gtk-4.0-dev` and `libsoup2.4-dev`):

```sh
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

Fedora:

```sh
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel patchelf
sudo dnf group install "C Development Tools and Libraries"
```

You'll also need `rustup` and `pnpm` (both are packaged on most distros, and
`pnpm` can come from `corepack enable`). Then, from the project root:

```sh
pnpm install
pnpm tauri dev    # run in dev mode
pnpm tauri build  # produce a .deb / .rpm / AppImage under src-tauri/target/release/bundle
```

Nothing here is distro-specific at runtime: app discovery and the NVIDIA
workaround below both key off standard paths rather than package names.

### NVIDIA proprietary driver

On the proprietary NVIDIA driver, WebKitGTK's DMABUF renderer can't allocate
GBM buffers. The window comes up blank (`Failed to create GBM buffer`) or, on
Wayland, the app dies at startup with `Error 71 (Protocol error) dispatching to
Wayland display`.

The app detects that driver at startup and sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` for itself, so no manual setup is needed.
Mesa, nouveau and everything else keep the accelerated path. To override the
detection, set the variable yourself — the app leaves an existing value alone:

```sh
WEBKIT_DISABLE_DMABUF_RENDERER=0 pnpm tauri dev  # force the DMABUF renderer on
```

WebKit reads that value rather than just checking whether it is set, so `=0` is
what reinstates the broken path — useful for confirming the driver is the cause,
since on an affected machine it brings the startup crash straight back.

### "Open with" app discovery

The app picker reads `.desktop` files from the standard XDG application
directories (`$XDG_DATA_HOME`, `$XDG_DATA_DIRS`, falling back to
`~/.local/share/applications` and `/usr/share/applications`), so it lists the
same apps your desktop environment's launcher would show — including Flatpak
and Snap, whose exported entries live in those same directories.

Entries are launched with their full `Exec` command line, and the project
directory replaces the entry's `%f`/`%u` placeholder (or is appended when there
isn't one). That's what lets wrapper-based entries work, such as Flatpak's
`flatpak run … --file-forwarding <app-id> @@u %u @@`, where the path has to land
between the file-forwarding markers.
