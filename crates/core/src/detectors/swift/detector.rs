use std::fs;
use std::io;
use std::path::Path;

use super::info::SwiftInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct SwiftDetector;

impl Detector for SwiftDetector {
    fn kind(&self) -> &'static str {
        "swift"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let manifest = path.join("Package.swift");

        let contents = match fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            // No `Package.swift` is the ordinary "not a Swift package" outcome;
            // only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The manifest's presence is the detection signal; the name is a bonus
        // that stays `None` when the `name:` argument can't be read.
        Ok(Some(Tracker::Swift(SwiftInfo {
            name: package_name(&contents),
        })))
    }
}

/// The package name from a `Package.swift`'s `Package(name: "...")`
/// initializer. The manifest is Swift source, but the one argument we want is
/// the first `name:` followed by a quoted string — the package's own, since the
/// initializer leads the file — so a scan earns its keep over compiling Swift.
/// `None` when no such argument can be read.
fn package_name(contents: &str) -> Option<String> {
    let after = &contents[contents.find("name:")? + "name:".len()..];
    let start = after.find('"')? + 1;
    let rest = &after[start..];
    let end = rest.find('"')?;
    let name = &rest[..end];
    (!name.is_empty()).then(|| name.to_string())
}
