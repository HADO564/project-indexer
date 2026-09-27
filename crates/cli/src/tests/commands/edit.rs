//! `edit`: its tag and property flags, and how the new tag list and property
//! map are built.

use std::collections::BTreeMap;

use clap::Parser;

use crate::commands::edit::{edited_properties, edited_tags, EditArgs};
use crate::commands::Command;
use crate::{Cli, Invocation};

/// The parsed `EditArgs`, or a panic naming what came back instead.
fn parsed(argv: &[&str]) -> EditArgs {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    match cli.invocation {
        Some(Invocation::Command(Command::Edit(args))) => args,
        other => panic!("expected `edit`, got {other:?}"),
    }
}

fn tags(list: &[&str]) -> Vec<String> {
    list.iter().map(|t| t.to_string()).collect()
}

#[test]
fn tag_flags_repeat_and_split_on_commas() {
    let args = parsed(&[
        "indexer",
        "edit",
        "app",
        "--add-tag",
        "rust,web",
        "--add-tag",
        "cli",
        "--remove-tag",
        "old",
    ]);
    assert_eq!(args.add_tag, tags(&["rust", "web", "cli"]));
    assert_eq!(args.remove_tag, tags(&["old"]));
    assert_eq!(args.description, None);
}

#[test]
fn a_tag_flag_alone_counts_as_a_change() {
    // Each flag belongs to the `change` group, so either one satisfies the
    // "at least one change" rule without `--description`.
    assert_eq!(
        parsed(&["indexer", "edit", "app", "--add-tag", "rust"]).add_tag,
        tags(&["rust"])
    );
    assert_eq!(
        parsed(&["indexer", "edit", "app", "--remove-tag", "rust"]).remove_tag,
        tags(&["rust"])
    );
}

#[test]
fn a_removal_matches_a_stored_tag_whatever_its_case() {
    // Stored tags are normalized (`Rust`); what the user types is not. Without
    // normalizing the removal first, this would remove nothing and succeed.
    assert_eq!(
        edited_tags(&tags(&["Rust", "Web"]), Vec::new(), &tags(&["rust"])),
        tags(&["Web"])
    );
    assert_eq!(
        edited_tags(&tags(&["Rust", "Web"]), Vec::new(), &tags(&[" RUST "])),
        tags(&["Web"])
    );
}

#[test]
fn removing_a_tag_the_project_lacks_changes_nothing() {
    assert_eq!(
        edited_tags(&tags(&["Rust"]), Vec::new(), &tags(&["web"])),
        tags(&["Rust"])
    );
}

#[test]
fn additions_are_appended_after_removals() {
    // Removals first, so a tag both removed and added ends up present — the
    // order `--help` promises. Additions go in as typed; core's `update`
    // title-cases them and drops duplicates.
    assert_eq!(
        edited_tags(&tags(&["Rust", "Web"]), tags(&["web"]), &tags(&["web"])),
        tags(&["Rust", "web"])
    );
    assert_eq!(
        edited_tags(&tags(&["Rust"]), tags(&["cli"]), &[]),
        tags(&["Rust", "cli"])
    );
}

fn props(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn set_splits_at_the_first_equals_and_keeps_commas() {
    let args = parsed(&[
        "indexer",
        "edit",
        "app",
        "--set",
        "url=https://x.com/?a=b",
        "--set",
        "note=a,b",
        "--unset",
        "old",
    ]);
    assert_eq!(
        args.set,
        pairs(&[("url", "https://x.com/?a=b"), ("note", "a,b")])
    );
    assert_eq!(args.unset, tags(&["old"]));
}

#[test]
fn set_without_an_equals_is_a_usage_error() {
    let err = Cli::try_parse_from(["indexer", "edit", "app", "--set", "client"])
        .expect_err("a value with no `=` should be refused");
    assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation);
}

#[test]
fn unset_matches_a_stored_name_whatever_its_case() {
    // The GUI stores names as typed; the search matches them ignoring case,
    // so `--unset` must too, or it would miss `Client` and still succeed.
    let current = props(&[("Client", "acme"), ("Owner", "me")]);
    assert_eq!(
        edited_properties(&current, Vec::new(), &tags(&["client"])),
        props(&[("Owner", "me")])
    );
}

#[test]
fn set_replaces_a_name_equal_ignoring_case_and_keeps_the_new_spelling() {
    let current = props(&[("Client", "acme")]);
    assert_eq!(
        edited_properties(&current, pairs(&[("client", "globex")]), &[]),
        props(&[("client", "globex")])
    );
}

#[test]
fn a_value_is_kept_exactly_as_typed() {
    // Core's rule: a value may contain anything, spaces at either end included.
    assert_eq!(
        edited_properties(&BTreeMap::new(), pairs(&[("pad", "  spaced  ")]), &[]),
        props(&[("pad", "  spaced  ")])
    );
}

#[test]
fn unsets_apply_before_sets_and_a_missing_name_changes_nothing() {
    let current = props(&[("Pad", "old")]);
    assert_eq!(
        edited_properties(&current, pairs(&[("Pad", "back")]), &tags(&["PAD"])),
        props(&[("Pad", "back")])
    );
    assert_eq!(
        edited_properties(&current, Vec::new(), &tags(&["nothere"])),
        current
    );
}
