use serde::{Deserialize, Serialize};


// The type of trackers a project could have.
// TODO: ADD. New trackers can be added here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tracker {
    Git,
    Unreal,
    Unity,
    Blender,
}
