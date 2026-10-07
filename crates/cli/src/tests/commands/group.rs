use indexer_core::Group;

use crate::commands::group::find;

fn groups(names: &[&str]) -> Vec<Group> {
    names
        .iter()
        .enumerate()
        .map(|(i, name)| {
            Group::new(name.to_string(), "cyan".into(), "folder".into(), i as i64).unwrap()
        })
        .collect()
}

#[test]
fn finds_a_group_ignoring_case_and_surrounding_spaces() {
    let groups = groups(&["Work", "Clients"]);
    assert_eq!(find(&groups, "  clients ").unwrap().name, "Clients");
}

#[test]
fn part_of_a_name_is_not_a_match() {
    let groups = groups(&["Work"]);
    assert!(
        find(&groups, "or").is_err(),
        "--group or must not find Work"
    );
}

#[test]
fn a_miss_lists_the_groups() {
    let groups = groups(&["Work", "Clients"]);
    let err = find(&groups, "Wrok").unwrap_err().to_string();
    assert_eq!(err, "no group named \"Wrok\". Groups: Work, Clients");
}

#[test]
fn a_miss_with_no_groups_says_so() {
    let err = find(&[], " Work ").unwrap_err().to_string();
    assert_eq!(err, "no group named \"Work\": there are no groups yet");
}

// The commands, against an in-memory database.

use super::support::{context, never_asked, run, Scripted};
use crate::commands::Outcome;

fn ctx(consent: bool) -> crate::context::Context {
    context(consent, Scripted(never_asked))
}

fn listed(ctx: &crate::context::Context) -> Vec<(String, String, String, usize)> {
    let Outcome::Groups { groups } = run(ctx, &["indexer", "group", "list"]).unwrap() else {
        panic!("group list lists groups");
    };
    groups
        .into_iter()
        .map(|(g, count)| (g.name, g.color, g.icon, count))
        .collect()
}

#[test]
fn create_takes_the_app_defaults_and_lists_last() {
    let ctx = ctx(true);
    run(&ctx, &["indexer", "group", "create", "Work"]).unwrap();
    run(
        &ctx,
        &[
            "indexer", "group", "create", "Clients", "--color", "#FF8800", "--icon", "Star",
        ],
    )
    .unwrap();
    assert_eq!(
        listed(&ctx),
        vec![
            ("Work".into(), "cyan".into(), "briefcase".into(), 0),
            ("Clients".into(), "#ff8800".into(), "star".into(), 0),
        ]
    );
}

#[test]
fn a_group_icon_is_a_bundled_one_and_a_colour_is_required() {
    let ctx = ctx(true);
    let custom = run(
        &ctx,
        &[
            "indexer",
            "group",
            "create",
            "Work",
            "--icon",
            "custom:logo",
        ],
    );
    assert!(custom
        .unwrap_err()
        .to_string()
        .starts_with("a group's icon is a bundled one"));
    let blank = run(
        &ctx,
        &["indexer", "group", "create", "Work", "--color", " "],
    );
    assert_eq!(blank.unwrap_err().to_string(), "a group needs a colour");
    let typo = run(
        &ctx,
        &["indexer", "group", "create", "Work", "--icon", "rockt"],
    );
    assert!(typo
        .unwrap_err()
        .to_string()
        .starts_with("not an icon: \"rockt\""));
    assert!(listed(&ctx).is_empty(), "nothing was created");
}

#[test]
fn a_name_another_group_has_is_refused_ignoring_case() {
    let ctx = ctx(true);
    run(&ctx, &["indexer", "group", "create", "Work"]).unwrap();
    assert!(run(&ctx, &["indexer", "group", "create", "work"]).is_err());
    assert_eq!(listed(&ctx).len(), 1);
}

#[test]
fn edit_changes_only_what_it_is_given() {
    let ctx = ctx(true);
    run(
        &ctx,
        &["indexer", "group", "create", "Work", "--icon", "code"],
    )
    .unwrap();
    run(
        &ctx,
        &["indexer", "group", "edit", "work", "--color", "Violet"],
    )
    .unwrap();
    run(
        &ctx,
        &["indexer", "group", "edit", "Work", "--name", "Office"],
    )
    .unwrap();
    assert_eq!(
        listed(&ctx),
        vec![("Office".into(), "violet".into(), "code".into(), 0)]
    );
}

#[test]
fn edit_needs_something_to_change() {
    use clap::Parser;
    let err = crate::Cli::try_parse_from(["indexer", "group", "edit", "Work"])
        .expect_err("a bare group edit changes nothing");
    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
}

#[test]
fn list_counts_each_group_s_live_projects() {
    let ctx = ctx(true);
    run(&ctx, &["indexer", "group", "create", "Work"]).unwrap();
    run(&ctx, &["indexer", "group", "create", "Empty"]).unwrap();
    run(&ctx, &["indexer", "edit", "app", "--group", "Work"]).unwrap();
    let counts: Vec<usize> = listed(&ctx).into_iter().map(|row| row.3).collect();
    assert_eq!(counts, vec![1, 0]);
}

#[test]
fn delete_asks_then_leaves_its_projects_ungrouped() {
    let ctx = ctx(true);
    run(&ctx, &["indexer", "group", "create", "Work"]).unwrap();
    run(&ctx, &["indexer", "edit", "app", "--group", "Work"]).unwrap();
    let outcome = run(&ctx, &["indexer", "group", "delete", "work"]).unwrap();
    assert!(matches!(outcome, Outcome::GroupDeleted { members: 1, .. }));
    assert!(listed(&ctx).is_empty());
    let app = ctx.projects.get("app-0000-4000-8000-000000000000").unwrap();
    assert_eq!(app.group_id, None, "the project stays, ungrouped");
}

#[test]
fn answering_no_keeps_the_group() {
    let ctx = ctx(false);
    run(&ctx, &["indexer", "group", "create", "Work"]).unwrap();
    let outcome = run(&ctx, &["indexer", "group", "delete", "Work"]).unwrap();
    assert!(matches!(outcome, Outcome::Cancelled));
    assert_eq!(listed(&ctx).len(), 1);
}

#[test]
fn an_unknown_group_is_an_error_naming_the_groups() {
    let ctx = ctx(true);
    run(&ctx, &["indexer", "group", "create", "Work"]).unwrap();
    let err = run(
        &ctx,
        &["indexer", "group", "edit", "Wrok", "--color", "pink"],
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "no group named \"Wrok\". Groups: Work");
}

#[test]
fn delete_counts_binned_members_too_as_core_ungroups_them() {
    let ctx = ctx(true);
    run(&ctx, &["indexer", "group", "create", "Work"]).unwrap();
    let work = ctx.groups.list().unwrap().remove(0).id;
    ctx.projects
        .update(
            "old-0000-4000-8000-000000000000",
            indexer_core::UpdateProject {
                group_id: Some(Some(work)),
                ..Default::default()
            },
        )
        .unwrap();
    let outcome = run(&ctx, &["indexer", "group", "delete", "Work"]).unwrap();
    assert!(matches!(outcome, Outcome::GroupDeleted { members: 1, .. }));
}
