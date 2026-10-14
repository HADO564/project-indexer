use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;

use super::info::NodeInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct NodeDetector;

impl Detector for NodeDetector {
    fn kind(&self) -> &'static str {
        "node"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let manifest = path.join("package.json");

        let contents = match fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            // No `package.json` is the ordinary "not a Node project" outcome,
            // not a failure. Only a genuine read error — a permission wall, a
            // bad mount — is worth surfacing as an `Err`.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The manifest's presence is the detection signal; its contents are a
        // bonus. A half-written or malformed `package.json` still marks the
        // directory a Node project — with the fields it couldn't parse left
        // empty — rather than erroring, so a broken manifest never hides the
        // project it sits in.
        let parsed: PackageJson = serde_json::from_str(&contents).unwrap_or_default();

        Ok(Some(Tracker::Node(NodeInfo {
            manifest_path: manifest.display().to_string(),
            name: non_empty(parsed.name),
            version: non_empty(parsed.version),
        })))
    }
}

/// The handful of `package.json` fields this detector reports. The real file
/// carries far more (scripts, dependencies, ...) that we neither need nor read
/// — `serde` skips unrecognized keys by default.
#[derive(Debug, Default, Deserialize)]
struct PackageJson {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
}

/// Collapses an empty string to `None`, the way the git and unreal detectors
/// treat a field that is present but blank as "not set" rather than a value.
fn non_empty(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}
