//! Tests for [`crate::domain::tracker`].
//!
//! These pin the *stored* shape of a tracker. `Tracker` is what goes into every
//! project record, so its serde form is a compatibility surface: changing it
//! silently orphans the trackers in every existing database. It was briefly a
//! generic `{ kind, data }` struct and is an enum again — these tests are what
//! proved both forms wrote byte-identical JSON, and they stay to keep it that
//! way.

use crate::detectors::git::GitInfo;
use crate::detectors::unreal::UnrealInfo;
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

/// An externally-tagged enum writes the variant name as the only key. The
/// frontend reads exactly this — `Object.keys(tracker)[0]` in `trackers.ts`.
#[test]
fn serializes_as_a_single_key_map() {
    let json = serde_json::to_value(Tracker::Git(git_info())).unwrap();
    let obj = json.as_object().expect("a tracker is a map");

    assert_eq!(obj.len(), 1, "exactly one key, the kind");
    assert!(obj.contains_key("Git"), "the key is the variant name");
    assert_eq!(obj["Git"]["repo_root"], "/tmp/x");
    assert_eq!(obj["Git"]["dirty"], true);
}

/// A record as it appears in a database written by an older build.
#[test]
fn deserializes_a_stored_record() {
    let stored = r#"{"Git":{"repo_root":"/tmp/x","dirty":true,"detached_head":false,
        "repo_url":null,"web_url":null,"contributors":[],"curr_branch":"main",
        "branches":null,"commit_hash":null}}"#;

    let tracker: Tracker = serde_json::from_str(stored).unwrap();

    let Tracker::Git(info) = tracker else {
        panic!("expected Tracker::Git");
    };
    assert_eq!(info.repo_root, "/tmp/x");
    assert_eq!(info.curr_branch.as_deref(), Some("main"));
}

#[test]
fn round_trips_through_json() {
    let original = serde_json::to_string(&Tracker::Git(git_info())).unwrap();
    let parsed: Tracker = serde_json::from_str(&original).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), original);
}

fn unreal_info() -> UnrealInfo {
    UnrealInfo {
        project_root: "/tmp/g".to_string(),
        project_name: "Game".to_string(),
        uproject_path: "/tmp/g/Game.uproject".to_string(),
        engine_association: None,
        category: None,
        description: None,
        modules: Vec::new(),
        plugins: Vec::new(),
        vcs_provider: None,
    }
}

/// `kind()` must agree with the serialised key for *every* variant, since
/// callers use it to talk about a tracker without matching on it — and a new
/// detector's variant is exactly where the two could drift apart.
#[test]
fn kind_matches_the_serialized_key() {
    for tracker in [Tracker::Git(git_info()), Tracker::Unreal(unreal_info())] {
        let json = serde_json::to_value(&tracker).unwrap();
        let key = json.as_object().unwrap().keys().next().unwrap().clone();
        assert_eq!(tracker.kind(), key);
    }
}

/// Callers say `is("git")` to match `Detector::kind()`, which is lowercase,
/// while the variant and the wire key are capitalised.
#[test]
fn is_matches_a_kind_regardless_of_case() {
    let tracker = Tracker::Git(git_info());
    assert!(tracker.is("git"));
    assert!(tracker.is("Git"));
    assert!(!tracker.is("unreal"));
}
