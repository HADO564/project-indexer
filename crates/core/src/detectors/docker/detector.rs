use std::fs;
use std::io;
use std::path::Path;

use super::info::DockerInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct DockerDetector;

/// The files that mark a Docker project, the image recipe first and then the
/// Compose files, newest spelling first. Any one of them proves the directory.
const MARKERS: [&str; 5] = [
    "Dockerfile",
    "compose.yaml",
    "compose.yml",
    "docker-compose.yml",
    "docker-compose.yaml",
];

impl Detector for DockerDetector {
    fn kind(&self) -> &'static str {
        "docker"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let Some(marker) = first_marker(path)? else {
            return Ok(None);
        };

        Ok(Some(Tracker::Docker(DockerInfo { marker })))
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
