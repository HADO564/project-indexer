use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;

use super::info::DotnetInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct DotnetDetector;

impl Detector for DotnetDetector {
    fn kind(&self) -> &'static str {
        "dotnet"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        // Unlike the other detectors, the marker is a glob — any `*.csproj` or
        // `*.sln` in the folder — so this reads the directory entries and
        // matches by extension rather than joining a fixed filename.
        let entries = match fs::read_dir(path) {
            Ok(entries) => entries,
            // A missing directory is the ordinary "nothing to detect" outcome;
            // only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut solution = None;
        let mut project = None;
        let mut global = false;
        // `flatten` skips an entry that fails to read: the listing itself
        // succeeded, so one unreadable entry shouldn't hide the project.
        for entry in entries.flatten() {
            let name = entry.file_name();
            if has_extension(&name, "sln") {
                solution.get_or_insert_with(|| name.to_string_lossy().into_owned());
            } else if has_extension(&name, "csproj") {
                project.get_or_insert_with(|| name.to_string_lossy().into_owned());
            } else if name.to_str() == Some("global.json") {
                global = true;
            }
        }

        // A solution names the whole thing, so it wins over a single project,
        // and either is a stronger signal than a bare `global.json`.
        let marker = solution
            .or(project)
            .or_else(|| global.then(|| "global.json".to_string()));

        Ok(marker.map(|marker| Tracker::Dotnet(DotnetInfo { marker })))
    }
}

/// Whether `name` ends in `.<ext>`, compared case-insensitively — `.csproj`
/// and `.CSPROJ` are the same marker.
fn has_extension(name: &OsStr, ext: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
}
