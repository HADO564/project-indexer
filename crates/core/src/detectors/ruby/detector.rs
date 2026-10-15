use std::fs;
use std::io;
use std::path::Path;

use super::info::RubyInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct RubyDetector;

impl Detector for RubyDetector {
    fn kind(&self) -> &'static str {
        "ruby"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let gemfile = path.join("Gemfile");

        let contents = match fs::read_to_string(&gemfile) {
            Ok(contents) => contents,
            // No `Gemfile` is the ordinary "not a Bundler project" outcome;
            // only a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The file's presence is the detection signal; the pinned Ruby version
        // is a bonus that stays `None` when no `ruby` directive is declared.
        Ok(Some(Tracker::Ruby(RubyInfo {
            ruby_version: ruby_version(&contents),
        })))
    }
}

/// The version string from a `Gemfile`'s `ruby "x.y.z"` directive. The Gemfile
/// is Ruby, but the one directive we want is a leading `ruby` followed by a
/// quoted string, so a line scan earns its keep over an interpreter. `None`
/// when no such line is present, so a Gemfile that omits it degrades to a
/// detected project with no version.
fn ruby_version(contents: &str) -> Option<String> {
    for line in contents.lines() {
        let line = line.trim();
        // The directive is `ruby`; a dependency line such as `gem "ruby-lsp"`
        // starts with `gem`, not `ruby`, so it is never mistaken for one.
        let Some(rest) = line.strip_prefix("ruby") else {
            continue;
        };
        // A space or a quote must follow, or this is a word that merely starts
        // with "ruby".
        if !rest.starts_with([' ', '\t', '"', '\'']) {
            continue;
        }
        if let Some(version) = first_quoted(rest) {
            return Some(version);
        }
    }
    None
}

/// The contents of the first single- or double-quoted string in `s`, or `None`
/// if there isn't one.
fn first_quoted(s: &str) -> Option<String> {
    for quote in ['"', '\''] {
        if let Some(open) = s.find(quote) {
            let rest = &s[open + 1..];
            if let Some(end) = rest.find(quote) {
                let value = &rest[..end];
                if !value.is_empty() {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}
