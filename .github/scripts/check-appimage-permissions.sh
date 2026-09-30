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
# catalog test. That is why this gate exists at all.
#
# It reads the modes *stored in the squashfs* with `unsquashfs -ll`, and does
# not extract. --appimage-extract is not usable for this: the current runtime
# creates every directory 0700 regardless of what the image says, so extracted
# directory modes describe the extractor rather than the artifact.
#
# Nothing here may use awk interval expressions (`{n}`): the default awk on
# Ubuntu is mawk, which does not honour them, and a regex that quietly matches
# nothing turns this gate into a rubber stamp. That is also why the parsed entry
# count is asserted below — an unreadable listing has to fail, not pass.
#
# See docs/app/KNOWN-ISSUES.md → PI-008.
set -euo pipefail

if ! command -v unsquashfs >/dev/null; then
  echo "error: unsquashfs not found — install squashfs-tools" >&2
  exit 1
fi

shopt -s nullglob
bundles=(target/release/bundle/appimage/*.AppImage)

if [ ${#bundles[@]} -eq 0 ]; then
  echo "error: no AppImage found under target/release/bundle/appimage/" >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

# `unsquashfs -ll` prints: mode owner/group size date time path, with a trailing
# " -> target" on symlinks. Selecting rows by a 10-character mode in the first
# field keeps this free of interval expressions.
select_rows='length($1) == 10 && $1 ~ /^[-dlbcsp]/'

status=0

for appimage in "${bundles[@]}"; do
  echo "Checking $appimage"

  chmod +x "$appimage"
  offset="$("$appimage" --appimage-offset)"

  listing="$work/listing.txt"
  unsquashfs -o "$offset" -ll "$appimage" > "$listing"

  # A listing that does not parse must fail. Silence would otherwise be
  # indistinguishable from a clean payload.
  entries="$(awk "$select_rows {n++} END {print n + 0}" "$listing")"
  if [ "$entries" -lt 10 ]; then
    echo "error: could not read the payload of $(basename "$appimage") —" >&2
    echo "parsed $entries entries from unsquashfs, which cannot be right." >&2
    status=1
    continue
  fi
  echo "  $entries entries in the payload"

  offenders="$(
    awk "$select_rows {
      mode = \$1
      path = \$6
      for (i = 7; i <= NF; i++) path = path \" \" \$i
      arrow = index(path, \" -> \")
      if (arrow > 0) path = substr(path, 1, arrow - 1)
      sub(/^squashfs-root\/?/, \"\", path)
      if (path == \"\") path = \"(payload root)\"

      type       = substr(mode, 1, 1)
      owner_exec = substr(mode, 4, 1) == \"x\" || substr(mode, 4, 1) == \"s\"
      other_read = substr(mode, 8, 1) == \"r\"
      other_exec = substr(mode, 10, 1) == \"x\" || substr(mode, 10, 1) == \"t\"

      if (type == \"-\") {
        if (!other_read)               print \"not readable by others: \" mode \" \" path
        if (owner_exec && !other_exec) print \"executable only for its owner: \" mode \" \" path
      } else if (type == \"d\") {
        if (!other_read || !other_exec) print \"not traversable by others: \" mode \" \" path
      }
    }" "$listing"
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
