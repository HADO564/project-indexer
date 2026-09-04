# Project Indexer — Known Issues

_Written 2026-08-28 from a Linux build-and-run pass (`PI-001`–`PI-004`, `main`
at `9761e80` plus `761d848`). Extended 2026-09-04 with `PI-005` from the first
Linux run of the post-refactor `main` (`5cf2275`)._

Five issues have been found while getting the Windows-developed app compiling
and running on Linux. They carry deliberately different dispositions: two were
real defects, one is cosmetic log noise, one is a linter false positive, and one
is an inaccurate comment.

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| PI-001 | Sort dropdown unreadable on Linux | Medium — user-visible | **Fixed** |
| PI-002 | Stale `filesystem.ts` 404 in dev log | Trivial — cosmetic | No action needed |
| PI-003 | `state_referenced_locally` warnings ×8 | None — false positive | Not a defect |
| PI-004 | NVIDIA comment understates its own scope | Trivial — comment accuracy | Open |
| PI-005 | Missing appindicator library kills startup | High — blocks launch | **Fixed** |

Nothing here blocks the Linux *build* — `cargo check`, `cargo test`, `pnpm build`
and `pnpm tauri build` all complete cleanly. PI-005 blocked the Linux *run* until
its fix: everything compiled and every test passed, and the app still exited
before showing a window.

---

## PI-001 — Sort dropdown renders unreadable on Linux

**Severity:** Medium (user-visible) · **Status:** Fixed in `761d848` · **Platform:** Linux; latent on Windows

The sort `<select>` in the default view painted as a solid white box with
near-white text, making the selected option invisible. The `↓` direction
toggle beside it was fine, as were every `<input>` in the same form.

**Cause.** The page never declared `color-scheme`. The engine therefore
assumed `light` and painted the `<select>` as a *native light menulist*,
overriding the CSS background — while Tailwind's `dark:text-gray-100` still
applied near-white text to it. The dark dropdown arrow was the giveaway: that
glyph is drawn by the engine's native theme, not by page CSS.

**Why only the select.** `<input>` is not drawn as a native menulist, so
`dark:bg-gray-800` applied to it normally. That is why the Name, Directory,
Description and Tags fields themed correctly in the very same form.

**Why Windows looked fine.** That desktop is light-themed, so
`prefers-color-scheme: light` selected `bg-white` + `text-gray-900`, which
happened to agree with the light native widget. Coincidence, not correctness —
the bug reproduces on Windows switched to dark mode.

**Fix** — `src/app.css`:

```css
html {
  color-scheme: light dark;
}
```

Chosen over `appearance: none` plus a hand-drawn arrow because it addresses the
root cause and covers every native widget at once — scrollbars, spinners, and
the `<select>`s in `BinModal.svelte:104` and `FavoritesModal.svelte:103` — rather
than patching one control and leaving the next to be rediscovered.

**Verified** by rebuilding the release binary and screenshotting the running
app: the control now reads "Last opened" on a dark ground.

**Update (GUI v1, `25eda04`):** the app is now a single committed dark theme
(`color-scheme: dark`, semantic `@theme` tokens, all `<select>`s styled
explicitly). There's no light-mode path left to regress into, so this class
of bug is retired rather than just patched.

---

## PI-002 — Stale `filesystem.ts` 404 in the dev log

**Severity:** Trivial (cosmetic) · **Status:** No action needed · **Scope:** dev server only

```
[404] GET /src/lib/api/filesystem.ts
```

`src/lib/api/filesystem.ts` was deleted in `7f8d8ae`. Nothing references it any
more — a sweep of `.ts`, `.js`, `.svelte`, `.json` and sourcemaps returns only
the unrelated Rust file `crates/core/src/platform/filesystem.rs` (moved there
from `src-tauri/src/utils/` in the frontend-agnostic-core refactor).

The request comes from the WebKit webview's cached module graph, left over from
a dev session predating the deletion. It appears only in the Vite dev log, never
in a production build — which is why `pnpm build` succeeds cleanly. It clears
itself once that cache is evicted.

---

## PI-003 — `state_referenced_locally` warnings in EditProjectForm

**Severity:** None (linter false positive) · **Status:** Not a defect · **Location:** `EditProjectForm.svelte:20-27`

Eight warnings of the form:

```
This reference only captures the initial value of `project`.
Did you mean to reference it inside a derived instead?
```

raised against the field seeds:

```js
let name = $state(project.name);
```

Svelte flags this in case `$derived` was intended. For a form the one-time seed
is correct — fields must not snap back while someone is typing. Three things
confirm it is safe here:

1. **Switching projects gives fresh values.** The form is mounted at
   `ProjectList.svelte:46` under `{#if editingId === project.id}` inside a keyed
   `{#each ... (project.id)}`. Editing a different project destroys the old
   component and constructs a new one, re-running every initializer.
2. **The form closes before any reload.** `handleSaved` (`+page.svelte:66`) and
   `handleCancelEdit` (`:62`) both clear `editingId` first; deleting the edited
   project clears it at `:90`. There is no polling or `setInterval`.
3. **A stale capture cannot clobber backend data.** The save payload
   (`EditProjectForm.svelte:41-50`) carries only user-editable fields — name,
   directory, description, tags, favorite, notes, client, open_with. Fields the
   backend manages, such as `tracker` and `last_opened_at`, are not in it.

The one case where `project` updates while the form stays mounted — another
card triggering `loadProjects()` — is precisely when preserving in-progress
typing is the desired behaviour.

**If the noise is unwanted,** state the intent explicitly rather than
restructuring:

```js
import { untrack } from "svelte";
let name = $state(untrack(() => project.name));
```

This silences the warning without changing behaviour. Purely cosmetic.

---

## PI-004 — NVIDIA workaround comment understates its own scope

**Severity:** Trivial (comment accuracy) · **Status:** Open · **Location:** `src-tauri/src/lib.rs:32`

The comment reads:

> Both paths are created by the proprietary kernel module only, so this is
> distro-independent — no package or driver-version probing needed.

The **open** kernel module creates `/proc/driver/nvidia/version` and
`/sys/module/nvidia/version` too. The test machine runs
`NVIDIA UNIX Open Kernel Module 610.57.04` and both paths exist, so
`disable_dmabuf_renderer_on_nvidia()` engages there.

That is the right outcome — the open module still uses the proprietary
userspace GL stack that has the GBM allocation failure — but the comment
describes a narrower trigger than the code actually has, which could mislead
someone later into "fixing" the detection. The code needs no change; the
wording does.

---

## PI-005 — A missing appindicator library kills the app at startup

**Severity:** High (blocks launch) · **Status:** Fixed · **Platform:** trigger is Linux/BSD; the mishandling was cross-platform

Every check passed — `cargo build` clean, 102 tests green, `pnpm build` fine —
and the app then exited immediately on launch with no window:

```
thread 'main' panicked at libappindicator-sys-0.9.0/src/lib.rs:41:5:
Failed to load ayatana-appindicator3 or appindicator3 dynamic library
```

**Cause.** The tray is built during `setup`, and `libappindicator-sys` calls a
bare `panic!` when it cannot load a library rather than returning an error. So
`setup_tray(app.handle())?` never observed the failure — the `?` was dead code
for it — and the process unwound out of `setup` before a window existed. The
library was absent because the README's Arch package list predates the tray
(v0.1.1) and never gained an appindicator entry; the Debian and Fedora lists
already had theirs.

**Why it was invisible.** The panic reaches stderr and nothing else. Launched
from a `.desktop` entry — the normal way — there is no output anywhere, so the
app simply fails to start with no explanation. This is the same failure shape
`fatal_startup_error` was introduced to prevent for the database in v0.1.1; the
tray call two lines below it kept the bare `?`.

**Why CI did not catch it.** The Linux job installs
`libayatana-appindicator3-dev`, so the library was always present there — and CI
compiles and runs the tests but never launches the app. Neither half of the run
could have reached this. A green CI on Linux says the target builds, not that the
window appears.

**Not just the panic, and not just Linux.** `libappindicator` is a dependency
only on Linux and the BSDs, so that panic cannot happen on Windows or macOS.
But on Windows `TrayIcon::new` returns `Err(Error::OsError(..))` when
`Shell_NotifyIcon` fails (`tray-icon-0.24.2/src/platform_impl/windows/mod.rs:145`),
and that `Err` took the same `?` → `.expect()` route out of `setup`. Uncommon
there — it is the explorer.exe-restarting case — but the symptom is identical:
no window, no message.

**Fix** — `src-tauri/src/lib.rs`, plus `libayatana-appindicator` added to the
README's Arch list:

- `setup_tray_or_warn()` wraps the builder in `catch_unwind`, handling all three
  outcomes (built, returned `Err`, panicked in the loader) and printing a message
  that names the package to install. The panic's own text still reaches stderr
  via the default hook.
- A `TRAY_AVAILABLE` flag gates the `CloseRequested` handler.

**Why the flag is the load-bearing half.** Closing the window hides it, because
the tray is how you get back. Degrading to "no tray" without also changing that
would be worse than the crash — the window would hide with nothing left to
restore it. With the flag, no tray means closing genuinely quits.

**Verified** on both paths by masking all four candidate libraries with bind
mounts in an unprivileged user namespace, and driving the real close path with
`hyprctl dispatch closewindow` (which delivers the same `xdg_toplevel` close as
clicking the titlebar X):

| | Close behaviour | Process | Tray |
|---|---|---|---|
| Library present | window hides | survives | registered on the SNI watcher |
| Library masked | window closes | exits | none; warning printed |

---

## Verification environment

| | |
|---|---|
| OS | Arch Linux, kernel 7.1.9 |
| Session | Hyprland / Wayland |
| GTK theme | `Adwaita-dark`, `color-scheme: prefer-dark` |
| GPU | NVIDIA UNIX Open Kernel Module 610.57.04 |
| WebKitGTK | 2.52.6 |
| Rust | 1.94.0 |
| Node / pnpm | 26.7.0 / 10.32.1 |

The GTK theme matters: `prefer-dark` is what puts the page into dark mode and
exposes PI-001. On a light-themed desktop the app looks correct and the defect
stays hidden.

PI-005 was found on the same machine on 2026-09-04, by which point it ran kernel
7.2.2, Node 26.8.1 / pnpm 11.21.0, and `libayatana-appindicator` 0.6.0-2 (absent
until that pass — which is what exposed the defect).
