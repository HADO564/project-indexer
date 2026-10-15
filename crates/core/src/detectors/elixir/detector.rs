use std::fs;
use std::io;
use std::path::Path;

use super::info::ElixirInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct ElixirDetector;

impl Detector for ElixirDetector {
    fn kind(&self) -> &'static str {
        "elixir"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let manifest = path.join("mix.exs");

        let contents = match fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            // No `mix.exs` is the ordinary "not a Mix project" outcome; only a
            // real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The manifest's presence is the detection signal; the app name is a
        // bonus that stays `None` when the `app:` key can't be read.
        Ok(Some(Tracker::Elixir(ElixirInfo {
            app: app_name(&contents),
        })))
    }
}

/// The application name from a `mix.exs`'s `app: :name` key. The manifest is
/// Elixir source, but the one key we want is the first `app:` followed by an
/// atom — the project's own, since `project/0` leads the module — so a scan
/// earns its keep over evaluating Elixir. `None` when no such atom can be read.
fn app_name(contents: &str) -> Option<String> {
    let after = contents[contents.find("app:")? + "app:".len()..].trim_start();
    let atom = after.strip_prefix(':')?;
    // An atom is a run of word characters; stop at the first thing that isn't.
    let name: String = atom
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}
