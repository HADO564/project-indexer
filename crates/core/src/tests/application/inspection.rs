//! Tests for [`crate::application::inspection`].

use crate::application::inspection::*;
use crate::detectors::{Detection, DetectorOutcome};
use crate::domain::git::GitInfo;
use crate::domain::tracker::Tracker;
use crate::error::DetectorError;

fn sample_git_tracker() -> Tracker {
    Tracker::Git(GitInfo {
        repo_root: "/tmp/x".to_string(),
        dirty: false,
        detached_head: false,
        repo_url: None,
        web_url: None,
        contributors: Vec::new(),
        curr_branch: Some("main".to_string()),
        branches: None,
        commit_hash: None,
    })
}

#[test]
fn results_from_maps_every_outcome_variant() {
    let detection = Detection {
        outcomes: vec![
            DetectorOutcome::Detected {
                kind: "git",
                tracker: sample_git_tracker(),
            },
            DetectorOutcome::NotDetected { kind: "unreal" },
            DetectorOutcome::Failed {
                kind: "unity",
                error: DetectorError::Other("boom".into()),
            },
        ],
    };

    let results = results_from(detection);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].kind, "git");
    assert!(matches!(results[0].status, DetectorStatus::Detected));
    assert!(results[0].tracker.is_some());
    assert!(matches!(results[1].status, DetectorStatus::NotDetected));
    assert!(results[1].tracker.is_none());
    assert!(matches!(results[2].status, DetectorStatus::Failed));
    assert_eq!(results[2].error.as_deref(), Some("boom"));
}
