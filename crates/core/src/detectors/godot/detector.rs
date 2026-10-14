use std::fs;
use std::io;
use std::path::Path;

use super::info::GodotInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct GodotDetector;

impl Detector for GodotDetector {
    fn kind(&self) -> &'static str {
        "godot"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let project_file = path.join("project.godot");

        let contents = match fs::read_to_string(&project_file) {
            Ok(contents) => contents,
            // No `project.godot` is the ordinary "not a Godot project"
            // outcome; only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The file's presence is the detection signal; the name is a bonus
        // that stays `None` when the file declares none.
        Ok(Some(Tracker::Godot(GodotInfo {
            project_name: project_name(&contents),
        })))
    }
}

/// The `config/name="..."` value from a `project.godot`'s `[application]`
/// section. The file is an ini-like format Godot writes itself, so the one key
/// we want is a line scan, as the unreal detector does for its ini.
fn project_name(contents: &str) -> Option<String> {
    let mut in_application = false;
    for line in contents.lines() {
        let line = line.trim();

        if let Some(header) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_application = header.trim() == "application";
            continue;
        }
        if !in_application {
            continue;
        }

        let Some(rest) = line.strip_prefix("config/name") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        // Godot writes the name as a double-quoted string.
        let value = rest.trim();
        if let Some(inner) = value.strip_prefix('"') {
            if let Some(end) = inner.find('"') {
                let name = &inner[..end];
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}
