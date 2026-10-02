//! `edit`: its tag and property flags, and how the new tag list and property
//! map are built.

use std::collections::BTreeMap;

use clap::Parser;

use indexer_core::UpdateProject;

use super::support::{context, never_asked, run, Scripted};
use crate::commands::edit::{edited_properties, edited_tags, EditArgs};
use crate::commands::{Command, Failure, Outcome};
use crate::editor::{ProjectEditor, TerminalEditor};
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

#[test]
fn a_bare_edit_saves_what_the_editor_returns() {
    // The form's changes go through the same `update` the flags use.
    let ctx = context(
        true,
        Scripted(|_| {
            Some(UpdateProject {
                description: Some("From the form".into()),
                ..Default::default()
            })
        }),
    );
    let Outcome::Edited { project } = run(&ctx, &["indexer", "edit", "app"]).unwrap() else {
        panic!("expected `Edited`");
    };
    assert_eq!(project.description, "From the form");
    assert_eq!(
        ctx.projects.get(&project.id).unwrap().description,
        "From the form"
    );
}

#[test]
fn cancelling_the_editor_changes_nothing() {
    let ctx = context(true, Scripted(|_| None));
    assert!(matches!(
        run(&ctx, &["indexer", "edit", "app"]).unwrap(),
        Outcome::Cancelled
    ));
    assert_eq!(
        ctx.projects
            .get("app-0000-4000-8000-000000000000")
            .unwrap()
            .description,
        ""
    );
}

#[test]
fn a_field_flag_never_opens_the_editor() {
    // `never_asked` panics if it is called.
    let ctx = context(true, Scripted(never_asked));
    assert!(run(&ctx, &["indexer", "edit", "app", "--add-tag", "rust"]).is_ok());
}

#[test]
fn the_terminal_editor_refuses_under_json_with_a_usage_error() {
    // `--json` is checked before the terminal, so this holds wherever the
    // tests run. `main` turns a `Failure::Usage` into prose and exit code 2.
    let project = super::support::project("app", false);
    let err = TerminalEditor::new(true)
        .edit(&project)
        .expect_err("no form under --json");
    assert!(matches!(
        err.downcast_ref::<Failure>(),
        Some(Failure::Usage { .. })
    ));
}

#[test]
fn an_editor_saved_untouched_writes_nothing() {
    let ctx = context(true, Scripted(|_| Some(UpdateProject::default())));
    let before = ctx
        .projects
        .get("app-0000-4000-8000-000000000000")
        .unwrap()
        .updated_at;
    let Outcome::Unchanged { project } = run(&ctx, &["indexer", "edit", "app"]).unwrap() else {
        panic!("expected `Unchanged`");
    };
    assert_eq!(project.name, "app");
    assert_eq!(
        ctx.projects.get(&project.id).unwrap().updated_at,
        before,
        "no write, so `updated_at` does not move"
    );
}
