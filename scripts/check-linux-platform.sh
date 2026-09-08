#!/usr/bin/env bash
# Cross-checks the Linux-gated platform code against the Linux target.
#
# The full crate cannot be cross-checked from Windows: git2/rusqlite build
# scripts need x86_64-linux-gnu-gcc. But the platform module is std-only plus
# InstalledApp, so it can be checked in isolation with a stub. Compiling FOR
# the Linux target makes cfg(target_os = "linux") true, so the gated code is
# what actually gets compiled.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-${TMPDIR:-/tmp}/pi-linux-check}"
rm -rf "$OUT"; mkdir -p "$OUT/src/platform" "$OUT/src/tests/platform"

cat > "$OUT/Cargo.toml" <<'EOF'
[package]
name = "linuxcheck"
version = "0.0.0"
edition = "2021"
[dependencies]
EOF

cat > "$OUT/src/lib.rs" <<'EOF'
#![allow(dead_code, unused_imports)]
pub mod domain {
    #[derive(Debug, Clone)]
    pub struct InstalledApp { pub name: String, pub path: String }
}
pub mod platform { pub mod app_discovery; pub mod app_launching; pub mod desktop_entry; }
#[cfg(test)]
mod tests { pub mod platform { pub mod app_launching; pub mod desktop_entry; } }
EOF

# Copy whichever layout exists: a single file, or a module directory.
copy() { # $1 = relative source path under crates/core/src, $2 = dest dir
  if [ -d "$ROOT/crates/core/src/$1" ]; then cp -r "$ROOT/crates/core/src/$1" "$2/";
  else cp "$ROOT/crates/core/src/$1.rs" "$2/"; fi
}
copy platform/app_discovery       "$OUT/src/platform"
copy platform/app_launching       "$OUT/src/platform"
copy platform/desktop_entry       "$OUT/src/platform"
copy tests/platform/app_launching "$OUT/src/tests/platform"
copy tests/platform/desktop_entry "$OUT/src/tests/platform"

cd "$OUT" && cargo check --target x86_64-unknown-linux-gnu --all-targets 2>&1 | tail -25
