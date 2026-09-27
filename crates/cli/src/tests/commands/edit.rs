//! `edit`: its tag flags, and how the new tag list is built.

use clap::Parser;

use crate::commands::edit::{edited_tags, EditArgs};
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
