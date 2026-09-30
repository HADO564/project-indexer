# Project Indexer — Known Issues

_Written 2026-08-28 from a Linux build-and-run pass (`main` at `9761e80` plus
`761d848`). Extended 2026-09-04 with `PI-005` from the first Linux run of the
post-refactor `main` (`5cf2275`). `PI-004`, an inaccurate comment on the NVIDIA
workaround, was fixed and retired the same day. Re-verified 2026-09-08 on `main`
at `5bb818f`: every gate below still passes on Arch, and `PI-006` still
reproduces — its previously untested workaround is now tested, and replaced with
one that needs no root. Extended 2026-09-14 with `PI-007`, the first entry from
a macOS run (`main` at `ba59c61`). Extended 2026-09-30 with `PI-008`, which no
run on this machine could have found: it was reported by the appimage.github.io
catalog test against the published `v0.3.1` artifact (`main` at `5ee4b63`)._

Six issues are tracked here from getting the Windows-developed app compiling,
running and packaging on Linux. They carry deliberately different dispositions:
three were real defects — one of them in the packaging toolchain rather than in
our code — one is cosmetic log noise, one is a linter false positive, and one is
the host distribution's problem rather than the app's. `PI-007` is macOS-only and
was reported separately, as issue #4.

| ID | Issue | Severity | Status |
|----|-------|----------|--------|
| PI-001 | Sort dropdown unreadable on Linux | Medium — user-visible | **Fixed** |
| PI-002 | Stale `filesystem.ts` 404 in dev log | Trivial — cosmetic | No action needed |
| PI-003 | `state_referenced_locally` warnings ×7 | None — false positive | Not a defect |
| PI-005 | Missing appindicator library kills startup | High — blocks launch | **Fixed** |
| PI-006 | AppImage bundling fails on Arch | Low — local packaging only | Environmental |
| PI-007 | Cmd+W in fullscreen leaves a black screen | Medium — user-visible | **Fixed** |
| PI-008 | Published AppImage starts only for its builder | High — blocks launch | **Fixed** in `v0.3.2` |

Nothing here blocks the Linux *build* — `cargo check`, `cargo clippy`,
`cargo fmt --check`, `cargo test`, `pnpm run check`, `pnpm test` and `pnpm build`
all complete cleanly, and `pnpm tauri build` produces the binary, the `.deb` and
the `.rpm`. On Arch it then exits 1 bundling the AppImage (PI-006) — a linuxdeploy
limitation, not a compilation failure, and absent on the `ubuntu-22.04` runner
that builds the published AppImage.

PI-005 blocked the Linux *run* until its fix: everything compiled and every test
passed, and the app still exited before showing a window.

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

**Severity:** None (linter false positive) · **Status:** Not a defect · **Location:** `EditProjectForm.svelte:38-46`

**Seven** warnings of the form (it was eight until the `client` field was
retired in favour of the open-ended `properties` map; the count tracks the
number of plainly-seeded fields and carries no other meaning):

```
This reference only captures the initial value of `project`.
Did you mean to reference it inside a derived instead?
```

raised against the field seeds:

```js
let name = $state(project.name);
```

The three fields added later — `color`, `icon`, `group_id`, and now
`properties` — seed through `untrack(() => project.x)` instead. That says the
same thing the plain form says, but deliberately, and does not raise a
warning. The seven below are left alone rather than converted: they are not
wrong, and rewriting working code to satisfy a linter it is already known to
be wrong about is churn.

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

## PI-006 — `tauri build` cannot produce an AppImage on Arch

**Severity:** Low (local packaging only) · **Status:** Environmental, no fix planned; local workaround verified · **Platform:** Arch and other rolling distributions

`pnpm run tauri build` exits 1 with `failed to run linuxdeploy` after having
already produced a working binary, `.deb` and `.rpm`. The AppImage target is the
only thing that fails, and nothing in this project causes it.

Two separate incompatibilities, in order:

1. **`strip` is too old for the host's libraries.** linuxdeploy ships its own
   binutils, which does not understand `SHT_RELR` (`unknown type [0x13] section
   '.relr.dyn'`) — a section modern Arch libraries are built with. It fails on
   nearly every system library it copies in. `NO_STRIP=1` skips stripping and
   clears this one.
2. **The GTK plugin assumes gdk-pixbuf loader modules exist on disk.** It runs
   `cp` on the `gdk_pixbuf_binarydir` that `pkg-config` reports —
   `/usr/lib/gdk-pixbuf-2.0/2.10.0` — which **does not exist** on Arch's
   `gdk-pixbuf2 2.44.7`, because the loaders are built into the library now.
   `cp: cannot stat …: No such file or directory`, and the plugin aborts.

**Why this is not worth fixing here.** An AppImage built on Arch links against a
glibc newer than its intended audience, so it would not run on the older
distributions AppImages exist to serve. The release workflow builds it on
`ubuntu-22.04`, where both problems are absent — **published AppImages are
unaffected**, and this only ever bites someone running the full bundle locally.

**If you need one locally anyway:** build `--bundles deb,rpm` and let CI produce
the AppImage, or run the bundle in an `ubuntu-22.04` container.

**Both causes can also be worked around directly, without root** (verified
2026-09-08 on `main` at `5bb818f`; produced a valid 103 MB AppImage that
extracts and carries the expected binary). `NO_STRIP=1` clears the first. For
the second, the earlier suggestion here was `sudo mkdir -p
/usr/lib/gdk-pixbuf-2.0/2.10.0` — that was never tested, and it is the worse
option: it puts an unowned directory in `/usr/lib` that no package will ever
clean up. Shadowing the `.pc` file instead keeps the whole thing in a temp
directory, because the plugin only ever learns the path by asking pkg-config:

```sh
shim=$(mktemp -d)
mkdir -p "$shim/pkgconfig" "$shim/pixbuf/2.10.0/loaders"
: > "$shim/pixbuf/2.10.0/loaders.cache"
sed "s|^gdk_pixbuf_binarydir=.*|gdk_pixbuf_binarydir=$shim/pixbuf/2.10.0|" \
  /usr/lib/pkgconfig/gdk-pixbuf-2.0.pc > "$shim/pkgconfig/gdk-pixbuf-2.0.pc"

NO_STRIP=1 PKG_CONFIG_PATH="$shim/pkgconfig" pnpm run tauri build --bundles appimage
```

An empty loader directory is the honest stand-in rather than a trick: on Arch
there genuinely are no loader modules left to copy, since `gdk-pixbuf2 2.44.7`
builds them into the library. **This does not make the result shippable** — the
glibc argument above is unchanged, and the AppImage this produces still only
runs on distributions as new as the one that built it. It is for reproducing a
bundling problem locally, not for release.

---

## PI-007 — Cmd+W in fullscreen leaves a black screen on macOS

**Severity:** Medium (user-visible) · **Status:** Fixed in `b52e9d4` (issue #4) · **Platform:** macOS

With the main window fullscreen, pressing Cmd+W (or the red close button) left
the screen black. The app kept running, but its fullscreen Space stayed open
with nothing in it, instead of the window hiding to the tray and returning you
to the previous Space.

**Cause.** The `CloseRequested` handler called `api.prevent_close()` and then
`window.hide()`. On macOS a fullscreen window lives in its own Space, and hiding
the window doesn't take it out of fullscreen — so the Space outlived the window.

**Why the obvious fix didn't work.** Leaving fullscreen first and hiding on the
next `Resized` event whose `is_fullscreen()` was `false` still only left
fullscreen; the window never hid. tao sets its fullscreen state to `None` the
moment `set_fullscreen(false)` is called
(`tao-0.35.3/src/platform_impl/macos/window.rs:1263`) and starts the animation
asynchronously, and Tauri's `is_fullscreen()` reads that state
(`tauri-runtime-wry-2.11.4/src/lib.rs:3406`). A temporary log in the `Resized`
handler showed the sequence:

```
Resized: pending=false fullscreen=Ok(true)    ← entering fullscreen
Resized: pending=false fullscreen=Ok(true)
Resized: pending=true fullscreen=Ok(false)    ← mid-animation: hide() ignored
Resized: pending=false fullscreen=Ok(false)   ← animation done, flag already cleared
```

The hide ran one event early, and macOS ignores a hide during the transition.
Counting `Resized` events instead would be fragile: how many arrive depends on
the window and on "Reduce motion".

**Why not hide the whole app.** `AppHandle::hide()` (Cmd+H) was tried too. It
does switch back to the previous Space, but the window stays fullscreen and its
Space stays open.

**Fix** — `src-tauri/src/tray.rs`, `src-tauri/src/lib.rs`, and macOS-only
`objc2`, `block2`, `objc2-foundation` and `objc2-app-kit` dependencies in
`src-tauri/Cargo.toml` (all already in the tree via tao):

- `hide_main_window()` hides a normal window as before. For a fullscreen one on
  macOS it sets `HIDE_AFTER_FULLSCREEN_EXIT` and calls `set_fullscreen(false)`.
- `watch_fullscreen_exit()`, registered once in `setup`, observes AppKit's
  `NSWindowDidExitFullScreenNotification` for the main window — posted only
  once the transition has actually finished — and hides the window if the flag
  is set, clearing it.

**Why the flag.** It limits the hide to exits started by a close. Leaving
fullscreen with the green button or Ctrl+Cmd+F posts the same notification and
must leave the window visible.

**Verified** manually on macOS 26.5 (Darwin 25.5.0) with `pnpm tauri dev`:

| Action | Result |
|---|---|
| Cmd+W in fullscreen | leaves fullscreen, then hides; previous Space shown |
| Red close button in fullscreen | same as Cmd+W |
| Tray icon click afterwards | window returns, not fullscreen |
| Green button or Ctrl+Cmd+F | leaves fullscreen, stays visible |
| Cmd+W when not fullscreen | hides immediately, as before |
| Quit from tray and relaunch | window restores normally |

The Dock icon stays while the window is hidden. That is the app's normal
close-to-tray behaviour on macOS, not part of this issue.

---

## PI-008 — The published AppImage will not start for anyone but its builder

**Severity:** High (blocks launch for everyone who downloads it) · **Status:** Fixed in `v0.3.2`; `v0.3.0` and `v0.3.1` are affected · **Platform:** Linux, the x86_64 AppImage only

The `v0.3.1` AppImage quits immediately, before any window, for most people who
download it:

```text
/run/firejail/appimage/AppRun: line 12: /run/firejail/appimage/AppRun.wrapped: Permission denied
ERROR: The application exited within 11 seconds instead of showing a window
```

**Cause.** Exactly one of the 300-odd entries in the payload is owner-restricted.
`AppRun.wrapped` is stored `0770 root:root`, so *other* is left with neither the
read nor the execute bit, while every sibling is fine:

```text
-rwxr-xr-x  AppRun              <- the shell wrapper
-rwxrwx---  AppRun.wrapped      <- the binary it execs: 0770
-rw-r--r--  217 files
drwxr-xr-x  54 dirs
```

The mode comes from Tauri, not from anything in this repository. Its AppImage
bundler downloads `AppRun-x86_64` into the tools cache through
`write_and_make_executable`, which calls
`fs::set_permissions(path, from_mode(0o770))`; `fs::copy` then carries that mode
into `AppDir/AppRun`, and linuxdeploy's GTK plugin — which needs an AppRun hook
— renames that file to `AppRun.wrapped` and writes its own `0755` shell script
as `AppRun`. That is why the wrapper is world-executable and the thing it execs
is not. The file inside the shipped bundle is byte-for-byte identical to the
upstream `AppRun-x86_64` (sha256 `f30140a4…73fb4f`), so nothing but the mode is
wrong.

**Why it was invisible here.** The AppImage runtime mounts its squashfs with
squashfuse and without `default_permissions`, so the kernel does not check modes
and every file reads as usable to whoever started it. The AppImage therefore
works on the machine that built it, and on every machine its author tries. It
only breaks for a *different* uid: under firejail, which is what the catalog
test uses; for an extraction performed by root and then run by a user; and for
any mount that does enforce permissions. `--appimage-extract` likewise re-owns
the tree to the extracting user, which hides it again.

**Why no gate caught it.** Every existing gate runs before or during the build —
`cargo`, `pnpm`, and the bundler's own success. None of them looks at the
finished artifact, and no test launches the app as another user. A green release
run said the bundle was produced, not that it could run.

**How it surfaced.** `AppImage/appimage.github.io#9004`, an auto-discovered
catalog entry for this repository, whose test reported the `Permission denied`
above along with `error-not-executable`. Upstream is
`tauri-apps/tauri#16155`, closed 2026-09-28 as fixed "in 2.12" — but
`write_and_make_executable` on `dev` still sets `0o770`, and no commit touching
the AppImage bundler since changes it. So this is worked around here rather than
waited out.

**Fix** — `.github/workflows/release.yml`, Linux job:

- A step before `tauri-action` seeds
  `${XDG_CACHE_HOME:-$HOME/.cache}/tauri/AppRun-x86_64` with the same upstream
  binary, checksum-pinned, at `0755`. The bundler downloads `AppRun` only when
  the cached copy is absent, so seeding it is the entire fix — no repacking of
  the finished AppImage, and no change to its contents.
- A step after it runs `.github/scripts/check-appimage-permissions.sh`, which
  reads the modes stored in the bundle with `unsquashfs -ll` and fails on any
  file that is not other-readable, any owner-executable file that is not
  other-executable, and any directory that is not other-traversable. The check
  has to read the artifact from outside, because the defect cannot be observed
  from the uid that produced it.

Artifacts go to a *draft* release, so a failing gate blocks publication rather
than arriving after the fact.

**Verified** on the published AppImages, downloaded from the release page rather
than taken from a build tree:

| Check | Result |
|---|---|
| `v0.3.1` `AppRun.wrapped` mode as stored | `0770`, uid/gid `0/0` |
| sha256 vs upstream `AppRun-x86_64` | identical — mode is the only defect |
| Gate against `v0.3.1` | fails, naming `AppRun.wrapped` and nothing else |
| `v0.3.2` `AppRun.wrapped` mode as stored | `0755` — seeding works |
| `v0.3.2` stored directory modes | 55 × `0755`, 1 × `0777`; none owner-only |
| Gate against `v0.3.2` | passes, 369 payload entries parsed |
| `v0.3.1` launched normally on the build machine | starts fine — the mount hides the problem |

**Two things made the first version of the gate worthless**, both found by running
it for real rather than by reading it:

1. **It extracted the bundle.** `--appimage-extract` under the runtime `v0.3.2`
   ships creates *every* directory `0700` no matter what the image stores, so the
   gate reported 56 unreadable directories in a payload whose stored modes were
   all `0755` — and it did so on the release run, having passed on `v0.3.1`,
   whose older runtime extracted `0755`. Extracted directory modes describe the
   extractor. Reading the image with `unsquashfs -ll` is the fix.
2. **Its mode regex used `{9}`.** The default awk on Ubuntu is mawk, which does
   not honour interval expressions, so the pattern matched nothing, no offender
   was ever printed, and the gate reported `ok` for the known-broken `v0.3.1` as
   readily as for the fixed `v0.3.2`. Selecting rows by `length($1) == 10`
   avoids intervals entirely, and the entry count is now asserted so that a
   listing which fails to parse fails the gate instead of passing it.

The second is the more instructive: a check that silently matches nothing is
worse than no check, because it manufactures confidence. Both were caught by
running the gate against a bundle known to be bad and requiring it to fail —
which is now how it is tested, in an `ubuntu:22.04` container so the awk is the
same one CI has.

`v0.3.2` was cut for this, because a fix in the workflow does nothing for the
artifact already on the release page and the catalog only ever tests the latest
release. `v0.3.0` and `v0.3.1` stay broken and cannot be repaired in place.

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

The 2026-09-08 re-verification ran on that same configuration (kernel 7.2.2,
Rust 1.94.0, Node 26.8.1 / pnpm 11.21.0, WebKitGTK 2.52.6, `gdk-pixbuf2`
2.44.7-1). Results: `cargo fmt --check`, `cargo clippy --workspace
--all-targets` (one pre-existing `module_inception` warning, no errors) and
`cargo test --workspace` (212 passed) clean; `pnpm run check` (0 errors, the 7
PI-003 warnings), `pnpm test` (99 passed) and `pnpm build` clean; `pnpm tauri
build` produced the binary, `.deb` and `.rpm`, then failed on the AppImage
exactly as PI-006 describes.

Note that `pnpm tauri build`'s failure is easy to miss when its output is piped:
`pnpm ... | tail` reports the exit status of `tail`, so the run looks like it
succeeded. Check `${PIPESTATUS[0]}` (`${pipestatus[1]}` in zsh), or don't pipe.
