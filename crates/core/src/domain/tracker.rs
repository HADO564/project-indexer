use crate::domain::git::GitInfo;
use crate::domain::unreal::UnrealInfo;
use serde::{Deserialize, Serialize};

/// A project type detected on a directory, carrying whatever detail that
/// tracker's detector was able to gather.
///
/// A variant exists only if a detector can produce it — add a new variant
/// together with its detector, never on its own. The frontend
/// (`lib/trackers.ts`) renders variants generically off their serde shape,
/// so a new one needs no frontend change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tracker {
    Git(GitInfo),
    Unreal(UnrealInfo),
}
