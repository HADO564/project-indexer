use std::fs;
use std::io;
use std::path::Path;

use super::info::PythonInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct PythonDetector;

/// The files that mark a Python project, most informative first. A
/// `pyproject.toml` carries a parseable name; `setup.py` and `requirements.txt`
/// only prove the directory is one, which is still worth tracking.
const MARKERS: [&str; 3] = ["pyproject.toml", "setup.py", "requirements.txt"];

impl Detector for PythonDetector {
    fn kind(&self) -> &'static str {
        "python"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let Some(marker) = first_marker(path)? else {
            return Ok(None);
        };

        // Only `pyproject.toml` puts the name in a fixed place. For the other
        // markers, and for a `pyproject.toml` we can't read or parse, the name
        // stays `None` — which the renderers already handle — rather than the
        // detection failing.
        let name = (marker == "pyproject.toml")
            .then(|| fs::read_to_string(path.join(&marker)).ok())
            .flatten()
            .and_then(|contents| toml_string_in_section(&contents, "project", "name"));

        Ok(Some(Tracker::Python(PythonInfo { marker, name })))
    }
}

/// The first of [`MARKERS`] present in `path`, or `None` if none is. A missing
/// file is an ordinary "not this one"; any other read error is a genuine
/// problem and propagates.
fn first_marker(path: &Path) -> Result<Option<String>, DetectorError> {
    for marker in MARKERS {
        match fs::metadata(path.join(marker)) {
            Ok(_) => return Ok(Some(marker.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(None)
}

/// The quoted string value of `key` in the `[section]` table of a minimal TOML
/// file, found by scanning lines. This is the one field the detector needs
/// (`[project] name`), so a line scan earns its keep over a TOML dependency,
/// the way the unreal detector line-scans its ini for one `Provider=` key.
/// `None` for a missing key or section, or a value this scan can't make sense
/// of, so a malformed manifest degrades to a detected project with no name.
fn toml_string_in_section(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in contents.lines() {
        let line = line.trim();

        if let Some(header) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            // `trim` catches the `[ project ]` spelling; a nested table like
            // `[project.optional-dependencies]` is not the `[project]` wanted.
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
