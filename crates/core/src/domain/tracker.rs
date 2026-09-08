use serde::{Deserialize, Serialize};

use crate::detectors::git::GitInfo;
use crate::detectors::unreal::UnrealInfo;

/// A project type detected on a directory, carrying whatever detail that
/// tracker's detector was able to gather.
///
/// A variant exists only if a detector can produce it — add a new variant
/// together with its detector, never on its own. The frontend
/// (`lib/trackers.ts`) renders variants generically off their serde shape,
/// so a new one needs no frontend change.
///
/// **This stays an enum on purpose.** It was briefly a generic
/// `{ kind, data }` struct so that adding a detector touched one file instead
/// of two — and that was the wrong trade. A detector is added a handful of
/// times in a project's life; a tracker's contents are read for the life of
/// the project. As an enum, the compiler proves every kind is handled and
/// catches a renamed field at build time. As a string-keyed map it could not,
/// and a rename would have compiled cleanly and silently stopped working.
///
/// The variants reference the info structs where they live, in each
/// detector's own directory. That the reference points from `domain` into
/// `detectors` is deliberate: they are modules of one crate, not layers of a
/// dependency graph, and keeping a detector's model beside its detector is
/// worth more than the tidiness of the arrow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tracker {
    Git(GitInfo),
    Unreal(UnrealInfo),
}

impl Tracker {
    /// The variant's name — the same string [`crate::detectors::Detector::kind`]
    /// reports, and the key this serialises under.
    ///
    /// Lets a caller that only cares *which* tracker this is avoid a `match`
    /// with an unused arm, without giving up exhaustiveness anywhere it does
    /// care about the payload.
    pub fn kind(&self) -> &'static str {
        match self {
            Tracker::Git(_) => "Git",
            Tracker::Unreal(_) => "Unreal",
        }
    }

    /// Case-insensitive kind test, so callers can say `is("git")` and match
    /// what `Detector::kind()` returns rather than the variant's spelling.
    pub fn is(&self, kind: &str) -> bool {
        self.kind().eq_ignore_ascii_case(kind)
    }
}
