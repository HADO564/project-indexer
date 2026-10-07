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
