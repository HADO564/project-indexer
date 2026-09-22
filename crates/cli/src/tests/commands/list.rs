//! `list`'s arguments, and the sort they map onto.

use clap::Parser;
use indexer_core::domain::sorting::{SortBy, SortDirection};

use crate::commands::list::{ListArgs, SortByKind};
use crate::commands::{Command, TrackerKind};
use crate::{Cli, Invocation};

/// The parsed `ListArgs`, or a panic naming what came back instead.
fn parsed(argv: &[&str]) -> ListArgs {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    match cli.invocation {
        Some(Invocation::Command(Command::List(args))) => args,
        other => panic!("expected `list`, got {other:?}"),
    }
}

#[test]
fn the_query_is_positional_and_the_rest_are_flags() {
    let args = parsed(&["indexer", "list"]);
    assert_eq!(args.query, None);
    assert_eq!(
        args.sort,
        SortByKind::Name,
        "names A to Z is the default view"
    );
    assert!(!args.reverse);
    assert!(args.tracker.is_empty());

    let args = parsed(&["indexer", "list", "app", "-s", "last-opened", "-r"]);
    assert_eq!(args.query.as_deref(), Some("app"));
    assert_eq!(args.sort, SortByKind::LastOpened);
    assert!(args.reverse);
}

#[test]
fn an_unknown_sort_field_is_rejected() {
    // `value_enum` is what makes this a usage error listing the valid values,
    // rather than a silent fallback to the default.
    assert!(Cli::try_parse_from(["indexer", "list", "--sort", "size"]).is_err());
}

#[test]
fn the_spelling_display_prints_is_the_one_clap_parses() {
    // `--help` shows `[default: name]` by rendering the value through
    // `Display`. If that disagreed with `ValueEnum`, the help would advertise
    // a value the command refuses.
    for kind in [SortByKind::Name, SortByKind::LastOpened] {
        let spelled = kind.to_string();
        let args = parsed(&["indexer", "list", "--sort", &spelled]);
        assert_eq!(args.sort, kind, "`{spelled}` should parse back to {kind:?}");
    }
}

#[test]
fn each_field_maps_to_its_core_counterpart() {
    assert_eq!(SortBy::from(SortByKind::Name), SortBy::Alphabetical);
    assert_eq!(SortBy::from(SortByKind::LastOpened), SortBy::LastOpened);
}

#[test]
fn reverse_flips_each_field_from_its_own_natural_order() {
    // The point of `direction`: the two fields disagree about which way is
    // natural, so `--reverse` cannot simply mean `Descending`. Plain
    // `--sort last-opened` has to answer most-recent-first, which core spells
    // `Descending` — the opposite of the default this would otherwise take.
    assert_eq!(
        SortByKind::Name.direction(false),
        SortDirection::Ascending,
        "names read A to Z"
    );
    assert_eq!(SortByKind::Name.direction(true), SortDirection::Descending);

    assert_eq!(
        SortByKind::LastOpened.direction(false),
        SortDirection::Descending,
        "last opened is only useful most recent first"
    );
    assert_eq!(
        SortByKind::LastOpened.direction(true),
        SortDirection::Ascending
    );
}

#[test]
fn tracker_still_parses_alongside_the_sort_flags() {
    let args = parsed(&["indexer", "list", "-t", "git,unreal", "-s", "last-opened"]);

    assert_eq!(args.tracker, vec![TrackerKind::Git, TrackerKind::Unreal]);
    assert_eq!(args.sort, SortByKind::LastOpened);
}
