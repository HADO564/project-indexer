use std::fs;
use std::io;
use std::path::Path;

use super::info::UnityInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct UnityDetector;

impl Detector for UnityDetector {
    fn kind(&self) -> &'static str {
        "unity"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        // Unity marks a project by this one generated file, not by a top-level
        // marker: the folder holds `Assets/`, `Library/`, ... but only
        // `ProjectSettings/ProjectVersion.txt` is a reliable, cheap signal.
        let version_file = path.join("ProjectSettings").join("ProjectVersion.txt");

        let contents = match fs::read_to_string(&version_file) {
            Ok(contents) => contents,
            // A missing file — or a missing `ProjectSettings` directory, which
            // also surfaces as `NotFound` here — is the ordinary "not a Unity
            // project" outcome; only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        Ok(Some(Tracker::Unity(UnityInfo {
            editor_version: editor_version(&contents),
        })))
    }
}

/// The `m_EditorVersion:` value from a `ProjectVersion.txt`, e.g. `2021.3.0f1`.
/// The file is a tiny YAML-ish list of `key: value` lines, so the one key we
/// want is a line scan rather than a reason to add a YAML parser.
fn editor_version(contents: &str) -> Option<String> {
    for line in contents.lines() {
        // `m_EditorVersion:` only — the file also carries
        // `m_EditorVersionWithRevision:`, which this prefix deliberately
        // excludes because the next char there is `W`, not `:`.
        if let Some(rest) = line.trim().strip_prefix("m_EditorVersion:") {
            let version = rest.trim();
            if !version.is_empty() {
                return Some(version.to_string());
            }
        }
    }
    None
}
