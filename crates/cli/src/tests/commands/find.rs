//! `find <query>`: its arguments, and the search it runs — the app's, not
//! `list`'s name-or-path match.

use clap::Parser;

use super::support::{context, never_asked, run, Scripted};
use crate::commands::find::FindArgs;
use crate::commands::list::SortByKind;
use crate::commands::{Command, Outcome, TrackerKind, View};
use crate::context::Context;
use crate::{Cli, Invocation};

/// The parsed `FindArgs`, or a panic naming what came back instead.
fn parsed(argv: &[&str]) -> FindArgs {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    match cli.invocation {
        Some(Invocation::Command(Command::Find(args))) => args,
        other => panic!("expected `find`, got {other:?}"),
    }
}

/// The support context, with `app` given a tag and a property to search for.
/// `old` stays as it is, binned.
fn searchable() -> Context {
    let ctx = context(true, Scripted(never_asked));
    let edit = [
        "dexily",
        "edit",
        "app",
        "--add-tag",
        "rust",
        "--set",
        "client=acme",
    ];
    run(&ctx, &edit).expect("the edit should save");
    ctx
}

/// The names `argv` came back with, in order.
fn found(ctx: &Context, argv: &[&str]) -> Vec<String> {
    match run(ctx, argv).expect("find should run") {
        Outcome::Projects { projects, .. } => projects.into_iter().map(|p| p.name).collect(),
        other => panic!("expected projects, got {other:?}"),
    }
}

#[test]
fn the_query_is_required_and_the_flags_are_the_ones_list_takes() {
    let args = parsed(&[
        "dexily",
        "find",
        "client: acme",
        "-s",
        "last-opened",
        "-r",
        "-t",
        "git",
        "--view",
        "favorites",
    ]);

    assert_eq!(args.query, "client: acme");
    assert_eq!(args.sort, SortByKind::LastOpened);
    assert!(args.reverse);
    assert_eq!(args.tracker, [TrackerKind::Git]);
    assert_eq!(args.view, View::Favorites);

    let err = Cli::try_parse_from(["dexily", "find"]).expect_err("a bare find should not parse");
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
}

#[test]
fn the_flags_default_as_they_do_for_list() {
    let args = parsed(&["dexily", "find", "app"]);

    assert_eq!(args.sort, SortByKind::Name);
    assert!(!args.reverse);
    assert!(args.tracker.is_empty());
    assert_eq!(args.view, View::All);
}

#[test]
fn free_text_matches_a_tag() {
    let ctx = searchable();

    assert_eq!(found(&ctx, &["dexily", "find", "RUST"]), ["app"]);
}

#[test]
fn free_text_matches_any_part_of_the_path_where_list_does_not() {
    let ctx = searchable();

    // `/home/me/work/app`: `work` is a folder in the middle of the path.
    assert_eq!(found(&ctx, &["dexily", "find", "work"]), ["app"]);
    assert!(found(&ctx, &["dexily", "list", "work"]).is_empty());
}

#[test]
fn free_text_matches_a_property_value_but_not_its_name() {
    let ctx = searchable();

    assert_eq!(found(&ctx, &["dexily", "find", "acme"]), ["app"]);
    assert!(found(&ctx, &["dexily", "find", "client"]).is_empty());
}

#[test]
fn name_colon_value_asks_about_that_property_alone() {
    let ctx = searchable();

    assert_eq!(found(&ctx, &["dexily", "find", "client: acme"]), ["app"]);
    assert!(found(&ctx, &["dexily", "find", "client: other"]).is_empty());
    // `app` is the name, but a property query looks at nothing else.
    assert!(found(&ctx, &["dexily", "find", "client: app"]).is_empty());
}

#[test]
fn the_view_picks_the_set_searched() {
    let ctx = searchable();

    assert!(found(&ctx, &["dexily", "find", "old"]).is_empty());
    assert_eq!(
        found(&ctx, &["dexily", "find", "old", "--view", "binned"]),
        ["old"]
    );
    assert!(found(&ctx, &["dexily", "find", "app", "--view", "favorites"]).is_empty());
}

#[test]
fn no_match_is_an_empty_list_that_names_the_query() {
    let ctx = searchable();

    match run(&ctx, &["dexily", "find", "nothing-here"]).unwrap() {
        Outcome::Projects {
            projects, query, ..
        } => {
            assert!(projects.is_empty());
            assert_eq!(query.as_deref(), Some("nothing-here"));
        }
        other => panic!("expected projects, got {other:?}"),
    }
}
