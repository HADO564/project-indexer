# Tauri + SvelteKit + TypeScript

This template should help get you started developing with Tauri, SvelteKit and TypeScript in Vite.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).

## Linux (Arch) setup

Install the system packages Tauri needs to build the WebKitGTK-based webview:

```sh
sudo pacman -S --needed webkit2gtk-4.1 base-devel curl wget file openssl \
  appmenu-gtk-module libappindicator-gtk3 librsvg patchelf
```

You'll also need `rustup` (`sudo pacman -S rustup && rustup default stable`) and `pnpm`
(`sudo pacman -S pnpm`, or via `corepack enable`). Then, from the project root:

```sh
pnpm install
pnpm tauri dev    # run in dev mode
pnpm tauri build  # produce a .deb / .rpm / AppImage under src-tauri/target/release/bundle
```

The "open with" app picker reads `.desktop` files from the standard XDG application
directories (`$XDG_DATA_HOME`, `$XDG_DATA_DIRS`, falling back to `~/.local/share/applications`
and `/usr/share/applications`), so it lists the same apps your desktop environment's launcher
would show.
