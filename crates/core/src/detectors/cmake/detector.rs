use std::fs;
use std::io;
use std::path::Path;

use super::info::CmakeInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct CmakeDetector;

impl Detector for CmakeDetector {
    fn kind(&self) -> &'static str {
        "cmake"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let listfile = path.join("CMakeLists.txt");

        let contents = match fs::read_to_string(&listfile) {
            Ok(contents) => contents,
            // No `CMakeLists.txt` is the ordinary "not a CMake project"
            // outcome; only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The file's presence is the detection signal; the name is a bonus
        // that stays `None` when no `project()` command can be read.
        Ok(Some(Tracker::Cmake(CmakeInfo {
            project_name: project_name(&contents),
        })))
    }
}

/// The first token of a `project(<name> ...)` command, e.g. `project(MyApp CXX)`
/// → `MyApp`. CMake is a command language, but the one name we want sits right
/// after the `project(` opener, so a line scan earns its keep over a parser,
/// the way the godot detector line-scans its ini for one key. `None` for a
/// listfile with no readable `project()`, so a file we can't parse degrades to
/// a detected project with no name.
fn project_name(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim();
        // Comments start with `#`; a commented-out `project()` is not one.
        if line.starts_with('#') {
            continue;
        }
        // The command name is case-insensitive in CMake, but must be the whole
        // leading word: `my_project(` is a different command. `get(..7)` is
        // `None` for a shorter line, and the seven bytes of "project" are ASCII
        // so slicing at 7 lands on a char boundary.
        let Some(rest) = line
            .get(..7)
            .filter(|head| head.eq_ignore_ascii_case("project"))
            .map(|_| &line[7..])
        else {
            continue;
        };
        // Whitespace is allowed between the command and its `(`.
        let Some(inner) = rest.trim_start().strip_prefix('(') else {
            continue;
        };
        // The first token, past an optional opening quote, up to whitespace,
        // the closing paren or a quote.
        let inner = inner.trim_start();
        let inner = inner.strip_prefix('"').unwrap_or(inner);
        let end = inner
            .find(|c: char| c.is_whitespace() || c == ')' || c == '"')
            .unwrap_or(inner.len());
        let name = &inner[..end];
        if !name.is_empty() {
            return Some(name.to_string());
        }
    }
    None
}
