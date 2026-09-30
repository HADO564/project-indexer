#!/usr/bin/env bash
#
# Fail the release if anything inside the AppImage is unreadable or
# unexecutable for a user other than the one that built it.
#
# Tauri's AppImage bundler writes its cached AppRun with mode 0770
# (tauri-apps/tauri#16155) and copies it into the AppDir, where linuxdeploy's
# GTK plugin renames it to AppRun.wrapped. "Other" is then left with no execute
# bit. The AppImage runtime's FUSE mount presents every file as owned by
# whoever started it, so an owner-only binary still launches for the person who
# built it — the defect cannot be seen from the inside, and only appears for a
# different uid: firejail, a root-owned extraction, the appimage.github.io
# catalog test. That is why this gate inspects the finished artifact from
# outside rather than trusting the build to have gone well.
#
# See docs/app/KNOWN-ISSUES.md → PI-008.
set -euo pipefail

shopt -s nullglob
bundles=(target/release/bundle/appimage/*.AppImage)

if [ ${#bundles[@]} -eq 0 ]; then
  echo "error: no AppImage found under target/release/bundle/appimage/" >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

status=0

for appimage in "${bundles[@]}"; do
  echo "Checking $appimage"

  # Extract rather than mount: --appimage-extract needs no FUSE, and it
  # preserves the stored modes (only ownership becomes ours), which is exactly
  # what is under test.
  rm -rf "$work/squashfs-root"
  cp "$appimage" "$work/app.AppImage"
  chmod +x "$work/app.AppImage"
  (cd "$work" && ./app.AppImage --appimage-extract >/dev/null)

  root="$work/squashfs-root"

  # Three ways a payload can be unusable to another uid. Symlinks are skipped:
  # their own mode is not consulted, the target's is.
  offenders="$(
    find "$root" -type f ! -perm -o=r -printf 'not readable by others: %M %P\n'
    find "$root" -type f -perm -u=x ! -perm -o=x -printf 'executable only for its owner: %M %P\n'
    find "$root" -type d \( ! -perm -o=r -o ! -perm -o=x \) -printf 'not traversable by others: %M %P\n'
  )"

  if [ -n "$offenders" ]; then
    echo "$offenders" >&2
    echo >&2
    echo "error: $(basename "$appimage") would fail to start for any user whose" >&2
    echo "uid does not match the builder's. See docs/app/KNOWN-ISSUES.md → PI-008." >&2
    status=1
  else
    echo "ok: every file is readable and every executable runnable for other users"
  fi
done

exit $status
