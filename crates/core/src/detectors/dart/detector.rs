use std::fs;
use std::io;
use std::path::Path;

use super::info::DartInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct DartDetector;

impl Detector for DartDetector {
    fn kind(&self) -> &'static str {
        "dart"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let manifest = path.join("pubspec.yaml");

        let contents = match fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            // No `pubspec.yaml` is the ordinary "not a Dart project" outcome;
            // only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The manifest's presence is the detection signal; the fields are a
        // bonus. `name` and `version` are top-level keys in `pubspec.yaml`, so
        // a line scan reads them without a YAML parser — either stays `None`
        // when it is absent or unreadable.
        Ok(Some(Tracker::Dart(DartInfo {
            name: yaml_top_level(&contents, "name"),
            version: yaml_top_level(&contents, "version"),
        })))
    }
}

/// The value of a top-level `key:` entry in a minimal YAML file, found by
/// scanning lines. Only column-zero keys are considered, so an indented `name:`
/// nested under another mapping is not mistaken for the top-level one. A
/// trailing `# comment` and surrounding quotes are stripped. `None` for a
/// missing key or an empty value, so a malformed manifest degrades to empty
/// fields rather than erroring.
fn yaml_top_level(contents: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in contents.lines() {
        // Top-level keys sit at column zero; a leading space marks a nested
        // entry this scan deliberately skips.
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let value = rest.split(" #").next().unwrap_or("").trim();
        let value = unquote(value);
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

/// `value` without a single matching pair of surrounding quotes, or unchanged
/// when it carries none.
fn unquote(value: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = value.strip_prefix(quote) {
            if let Some(stripped) = inner.strip_suffix(quote) {
                return stripped;
            }
        }
    }
    value
}
