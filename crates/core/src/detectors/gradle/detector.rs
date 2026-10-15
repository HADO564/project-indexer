use std::fs;
use std::io;
use std::path::Path;

use super::info::GradleInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct GradleDetector;

/// The build scripts that mark a Gradle project: the Groovy DSL first, then the
/// Kotlin DSL. Either proves the directory a Gradle build.
const MARKERS: [&str; 2] = ["build.gradle", "build.gradle.kts"];

impl Detector for GradleDetector {
    fn kind(&self) -> &'static str {
        "gradle"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let Some(marker) = first_marker(path)? else {
            return Ok(None);
        };

        Ok(Some(Tracker::Gradle(GradleInfo { marker })))
    }
}

/// The first of [`MARKERS`] present in `path`, or `None` if none is. A missing
/// file is an ordinary "not this one"; any other read error propagates.
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
