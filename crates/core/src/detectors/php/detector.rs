use serde::Deserialize;
use std::fs;
use std::io;
use std::path::Path;

use super::info::PhpInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct PhpDetector;

impl Detector for PhpDetector {
    fn kind(&self) -> &'static str {
        "php"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let manifest = path.join("composer.json");

        let contents = match fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            // No `composer.json` is the ordinary "not a Composer project"
            // outcome; only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The manifest's presence is the detection signal; the name is a bonus.
        // A malformed `composer.json` still marks the directory a PHP project —
        // with no name — rather than erroring. `composer.json` is JSON, so this
        // uses the `serde_json` the crate already carries rather than a scan.
        let parsed: ComposerJson = serde_json::from_str(&contents).unwrap_or_default();

        Ok(Some(Tracker::Php(PhpInfo {
            name: non_empty(parsed.name),
        })))
    }
}

/// The one `composer.json` field this detector reports. The real file carries
/// far more (`require`, `autoload`, ...) that we neither need nor read — serde
/// skips unrecognized keys by default.
#[derive(Debug, Default, Deserialize)]
struct ComposerJson {
    #[serde(default)]
    name: String,
}

/// Collapses an empty string to `None`, as the node detector does for a field
/// that is present but blank.
fn non_empty(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}
