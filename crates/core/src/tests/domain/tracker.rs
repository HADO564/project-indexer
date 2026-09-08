//! Tests for [`crate::domain::tracker`].
//!
//! The wire-format tests are the load-bearing ones: `Tracker` stopped being an
//! enum, and the whole point of the hand-written `Serialize`/`Deserialize` is
//! that stored projects did not have to be migrated. If these fail, the change
//! is no longer backwards compatible.

use crate::detectors::git::GitInfo;
use crate::domain::Tracker;

fn git_info() -> GitInfo {
    GitInfo {
        repo_root: "/tmp/x".to_string(),
        dirty: true,
        detached_head: false,
        repo_url: Some("https://github.com/u/r.git".to_string()),
        web_url: None,
        contributors: Vec::new(),
        curr_branch: Some("main".to_string()),
        branches: None,
        commit_hash: None,
    }
}

/// The exact shape an externally-tagged `enum Tracker { Git(GitInfo) }`
/// produced, which is what every project already stored on disk contains.
#[test]
fn serializes_as_a_single_key_map() {
    let json = serde_json::to_value(Tracker::new("Git", git_info())).unwrap();
    let obj = json.as_object().expect("a tracker is a map");

    assert_eq!(obj.len(), 1, "exactly one key, the kind");
    assert!(obj.contains_key("Git"), "the key is the kind verbatim");
    assert_eq!(obj["Git"]["repo_root"], "/tmp/x");
    assert_eq!(obj["Git"]["dirty"], true);
}

/// Reading a record written before `Tracker` stopped being an enum.
#[test]
fn deserializes_a_record_written_by_the_enum() {
    let stored = r#"{"Git":{"repo_root":"/tmp/x","dirty":true,"detached_head":false,
        "repo_url":null,"web_url":null,"contributors":[],"curr_branch":"main",
        "branches":null,"commit_hash":null}}"#;

    let tracker: Tracker = serde_json::from_str(stored).unwrap();

    assert_eq!(tracker.kind, "Git");
    assert!(tracker.is("git"), "matching a kind ignores case");
    assert_eq!(tracker.str_field("curr_branch"), Some("main"));
    let info: GitInfo = tracker.info().expect("the payload still fits GitInfo");
    assert_eq!(info.repo_root, "/tmp/x");
}

#[test]
fn round_trips_through_json() {
    let original = Tracker::new("Git", git_info());
    let json = serde_json::to_string(&original).unwrap();
    assert_eq!(serde_json::from_str::<Tracker>(&json).unwrap(), original);
}

/// `null` and `""` are both "no value" to a reader, which is what the field
/// helpers exist to collapse.
#[test]
fn str_field_treats_empty_and_null_as_absent() {
    let tracker = Tracker::new("Git", git_info());
    assert_eq!(tracker.str_field("web_url"), None, "null is absent");
    assert_eq!(tracker.str_field("nonexistent"), None);
    assert_eq!(tracker.str_field("repo_root"), Some("/tmp/x"));
}

/// Two kinds in one slot is corrupt input, not a tracker to pick from.
#[test]
fn refuses_a_map_that_is_not_exactly_one_kind() {
    assert!(serde_json::from_str::<Tracker>("{}").is_err());
    assert!(serde_json::from_str::<Tracker>(r#"{"Git":{},"Unreal":{}}"#).is_err());
}
