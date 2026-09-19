//! `scan <dir>`: its arguments, and the choices `run` makes from them.

use clap::Parser;
use indexer_core::domain::scan::{Candidate, ScanMode, ScanReport};

use crate::commands::scan::{detectors, mode, selections, ScanArgs};
use crate::commands::{Command, TrackerKind};
use crate::{Cli, Invocation};

/// The kinds a build with both detectors reports, in their own spelling.
fn available() -> Vec<String> {
    vec!["Git".to_string(), "Unreal".to_string()]
}

fn candidate(directory: &str, name: &str) -> Candidate {
    Candidate {
        directory: directory.to_string(),
        suggested_name: name.to_string(),
        matched_kinds: vec!["git".to_string()],
        already_tracked: false,
        disambiguated: false,
    }
}

/// The parsed `ScanArgs`, or a panic naming what came back instead.
fn parsed(argv: &[&str]) -> ScanArgs {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    match cli.invocation {
        Some(Invocation::Command(Command::Scan(args))) => args,
        other => panic!("expected `scan`, got {other:?}"),
    }
}

#[test]
fn the_directory_is_positional_and_everything_else_is_a_flag() {
    let args = parsed(&["indexer", "scan", "/home/me/code"]);

    assert_eq!(args.directory.to_string_lossy(), "/home/me/code");
    assert_eq!(args.depth, 1, "a plain scan visits only the children");
    assert!(!args.include_ignored);
    assert!(!args.import);
    assert!(args.tracker.is_empty());
}

#[test]
fn the_flags_parse() {
    let args = parsed(&[
        "indexer",
        "scan",
        "/home/me/code",
        "--depth",
        "3",
        "--include-ignored",
        "--import",
        "-t",
        "git,unreal",
    ]);

    assert_eq!(args.depth, 3);
    assert!(args.include_ignored);
    assert!(args.import);
    assert_eq!(args.tracker, [TrackerKind::Git, TrackerKind::Unreal]);
}

#[test]
fn depth_one_is_the_walkers_quick_mode() {
    assert_eq!(mode(1), ScanMode::Quick);
    assert_eq!(mode(0), ScanMode::Quick, "a floor, not an error");
    assert_eq!(mode(3), ScanMode::Deep { depth: 3 });
}

#[test]
fn no_tracker_flag_scans_for_every_kind_the_build_has() {
    assert_eq!(detectors(available(), &[]), ["Git", "Unreal"]);
}

#[test]
fn a_requested_kind_matches_the_detectors_own_spelling() {
    // `--tracker git` is lowercase; the detector calls itself `Git`, and the
    // scan compares kinds exactly.
    assert_eq!(detectors(available(), &[TrackerKind::Git]), ["Git"]);
}

#[test]
fn a_kind_this_build_does_not_have_scans_for_nothing() {
    // Rather than falling back to every kind, which would silently scan for
    // more than was asked for.
    assert!(detectors(vec!["Unreal".to_string()], &[TrackerKind::Git]).is_empty());
}

#[test]
fn every_candidate_is_imported_under_the_name_the_scan_settled_on() {
    let report = ScanReport {
        candidates: vec![
            candidate("/home/me/code/api", "api"),
            candidate("/home/me/work/api", "work/api"),
        ],
        visited: 9,
        stopped_early: false,
    };

    let selections = selections(&report);

    assert_eq!(selections.len(), 2);
    assert_eq!(selections[0].directory, "/home/me/code/api");
    assert_eq!(selections[1].name, "work/api", "the disambiguated name");
}
