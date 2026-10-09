//! Phase 1 on argv alone: which commands dexily knows, and the folders each
//! should leave behind. No processes, no disk.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::observe::recognizers::{expect, Expectation};

fn cwd() -> &'static Path {
    Path::new("/home/me/code")
}

fn expected(words: &[&str]) -> Option<Vec<PathBuf>> {
    let argv: Vec<OsString> = words.iter().map(OsString::from).collect();
    expect(&argv, cwd()).map(|Expectation { folders }| folders)
}

fn under_cwd(names: &[&str]) -> Option<Vec<PathBuf>> {
    Some(names.iter().map(|name| cwd().join(name)).collect())
}

#[test]
fn mkdir_expects_the_folder_it_names_under_the_working_folder() {
    assert_eq!(expected(&["mkdir", "thing"]), under_cwd(&["thing"]));
}

#[test]
fn mkdir_expects_every_folder_it_names() {
    assert_eq!(
        expected(&["mkdir", "a", "b", "c"]),
        under_cwd(&["a", "b", "c"])
    );
}

#[test]
fn with_p_only_the_named_path_is_meant_not_its_parents() {
    assert_eq!(
        expected(&["mkdir", "-p", "work/clients/acme"]),
        under_cwd(&["work/clients/acme"])
    );
}

#[test]
fn a_mode_is_never_taken_for_a_folder() {
    for argv in [
        &["mkdir", "-m", "755", "thing"][..],
        &["mkdir", "-pm", "755", "thing"],
        &["mkdir", "-m755", "thing"],
        &["mkdir", "--mode", "755", "thing"],
        &["mkdir", "--mode=755", "thing"],
    ] {
        assert_eq!(expected(argv), under_cwd(&["thing"]), "{argv:?}");
    }
}

#[test]
fn other_options_are_skipped() {
    assert_eq!(
        expected(&["mkdir", "-v", "--parents", "thing"]),
        under_cwd(&["thing"])
    );
}

#[test]
fn after_a_double_dash_a_name_may_start_with_a_dash() {
    assert_eq!(expected(&["mkdir", "--", "-odd"]), under_cwd(&["-odd"]));
}

#[test]
fn an_absolute_path_stays_as_it_is() {
    assert_eq!(
        expected(&["mkdir", "/srv/projects/new"]),
        Some(vec![PathBuf::from("/srv/projects/new")])
    );
}

#[test]
fn mkdir_naming_nothing_expects_nothing() {
    assert_eq!(expected(&["mkdir"]), None);
    assert_eq!(expected(&["mkdir", "-p"]), None);
}

#[test]
fn a_command_without_a_recognizer_is_not_known() {
    for argv in [
        &["ls", "-la"][..],
        &["rm", "-rf", "thing"],
        &["git", "push"],
    ] {
        assert_eq!(expected(argv), None, "{argv:?}");
    }
}

#[test]
fn only_the_bare_name_is_known_never_a_path_to_another_program() {
    // `/tmp/x/mkdir` could be anything that happens to share the name.
    assert_eq!(expected(&["/tmp/x/mkdir", "thing"]), None);
    assert_eq!(expected(&["./mkdir", "thing"]), None);
}

#[test]
fn nothing_at_all_is_not_known() {
    assert_eq!(expected(&[]), None);
}
