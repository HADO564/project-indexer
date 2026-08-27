use crate::models::git::GitInfo;
use crate::models::unreal::UnrealInfo;
use serde::{Deserialize, Serialize};

// The type of trackers a project could have, carrying whatever detail that
// tracker's detector was able to gather.
// TODO: ADD NEW trackers can be added here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tracker {
    Git(GitInfo),
    Unreal(UnrealInfo),
    Unity,
    Blender,
}
