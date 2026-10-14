use std::fs;
use std::io;
use std::path::Path;

use super::info::GoInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct GoDetector;

impl Detector for GoDetector {
    fn kind(&self) -> &'static str {
        "go"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let go_mod = path.join("go.mod");

        let contents = match fs::read_to_string(&go_mod) {
            Ok(contents) => contents,
            // No `go.mod` is the ordinary "not a Go module" outcome; only a
            // real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The file's presence is the detection signal; the module path is a
        // bonus that stays `None` if the directive is missing or malformed.
        Ok(Some(Tracker::Go(GoInfo {
            module: module_path(&contents),
        })))
    }
}

/// The module path from a `go.mod`'s `module` directive, e.g.
/// `github.com/user/thing`. `go.mod` is line-oriented, so the one directive we
/// want is a cheap scan rather than a reason to add a parser.
fn module_path(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let Some(rest) = line.trim().strip_prefix("module") else {
            continue;
        };
        // A space must follow the keyword, or this is some other line that
        // merely starts with the letters "module".
        let Some(rest) = rest.strip_prefix([' ', '\t']) else {
            continue;
        };
        // Take the first token, ignoring a trailing line comment, and strip the
        // surrounding quotes a quoted module path may carry.
        let module = rest
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches('"');
        if !module.is_empty() {
            return Some(module.to_string());
        }
    }
    None
}
