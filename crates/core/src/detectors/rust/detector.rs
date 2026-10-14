use std::fs;
use std::io;
use std::path::Path;

use super::info::RustInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct RustDetector;

impl Detector for RustDetector {
    fn kind(&self) -> &'static str {
        "rust"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let manifest = path.join("Cargo.toml");

        let contents = match fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            // No `Cargo.toml` is the ordinary "not a Rust project" outcome;
            // only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The manifest's presence is the detection signal; the name is a
        // bonus. A workspace root with only `[workspace]` has no `[package]`
        // name, and a malformed manifest parses to none — both leave `name`
        // `None` and still mark the directory a Rust project.
        Ok(Some(Tracker::Rust(RustInfo {
            name: toml_string_in_section(&contents, "package", "name"),
        })))
    }
}

/// The quoted string value of `key` in the `[section]` table of a minimal TOML
/// file, found by scanning lines. This is the one field the detector needs
/// (`[package] name`), so a line scan earns its keep over a TOML dependency,
/// the way the unreal detector line-scans its ini for one `Provider=` key.
/// `None` for a missing key or section, or a value this scan can't make sense
/// of, so a malformed manifest degrades to a detected project with no name.
fn toml_string_in_section(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in contents.lines() {
        let line = line.trim();

        if let Some(header) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            // `trim` catches the `[ package ]` spelling; a nested table like
            // `[package.metadata]` is not the `[package]` wanted.
            in_section = header.trim() == section;
            continue;
        }
        if !in_section {
            continue;
        }

        let Some(rest) = line.strip_prefix(key) else {
            continue;
        };
        // The key must be whole: `name` matches `name =`, never `namespace`.
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        if let Some(value) = toml_string(rest.trim()) {
            return Some(value);
        }
    }
    None
}

/// The contents of the first single- or double-quoted string in `value`, or
/// `None` if it isn't quoted — enough for a plain `name = "x"`, and `None`
/// (never a panic) for anything fancier.
fn toml_string(value: &str) -> Option<String> {
    for quote in ['"', '\''] {
        if let Some(inner) = value.strip_prefix(quote) {
            if let Some(end) = inner.find(quote) {
                return Some(inner[..end].to_string());
            }
        }
    }
    None
}
