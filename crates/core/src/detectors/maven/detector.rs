use std::fs;
use std::io;
use std::path::Path;

use super::info::MavenInfo;
use crate::detectors::detector::Detector;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

pub struct MavenDetector;

impl Detector for MavenDetector {
    fn kind(&self) -> &'static str {
        "maven"
    }

    fn detect(&self, path: &Path) -> Result<Option<Tracker>, DetectorError> {
        let manifest = path.join("pom.xml");

        let contents = match fs::read_to_string(&manifest) {
            Ok(contents) => contents,
            // No `pom.xml` is the ordinary "not a Maven project" outcome; only
            // a real read error is worth surfacing.
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        // The manifest's presence is the detection signal; the two fields are a
        // bonus. We read the first `<artifactId>`/`<version>`, which is the
        // project's own in a plain pom — a `<parent>` block can shadow it, a
        // known trade for not pulling in an XML parser for two tags. Either
        // field stays `None` when it is absent or unreadable.
        Ok(Some(Tracker::Maven(MavenInfo {
            artifact_id: xml_tag_value(&contents, "artifactId"),
            version: xml_tag_value(&contents, "version"),
        })))
    }
}

/// The trimmed text of the first `<tag>...</tag>` pair in `contents`, or `None`
/// when the tag is absent, unclosed or empty. A deliberately small scan: it
/// ignores namespaces and attributes, which plain `pom.xml` fields don't carry,
/// so a malformed file degrades to empty fields rather than a parse error.
fn xml_tag_value(contents: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = contents.find(&open)? + open.len();
    let rest = &contents[start..];
    let end = rest.find(&close)?;
    let value = rest[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}
