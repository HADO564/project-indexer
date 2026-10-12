//! Phase 1 on argv alone: which commands dexily knows, and the folders each
//! should leave behind. No processes, no disk.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::observe::recognizers::{confirm, expect, Expectation};

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
fn mkdir_naming_nothing_is_still_known_and_expects_no_folders() {
    // It runs — mkdir prints its own usage, `--help` works — and there is
    // nothing to track. `None` would call mkdir an unknown command.
    for argv in [&["mkdir"][..], &["mkdir", "-p"], &["mkdir", "--help"]] {
        assert_eq!(expected(argv), Some(Vec::new()), "{argv:?}");
    }
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

// Phase 3: the evidence. Real folders in a temp dir, made-up exit codes.

/// An expectation of `names` under `root`, with each folder's "was it there
/// before" taken now, as the observer takes it before running the command.
fn before(root: &Path, names: &[&str]) -> (Expectation, Vec<bool>) {
    let folders: Vec<PathBuf> = names.iter().map(|name| root.join(name)).collect();
    let existed = folders.iter().map(|f| f.exists()).collect();
    (Expectation { folders }, existed)
}

#[test]
fn a_folder_made_by_a_command_that_succeeded_is_confirmed() {
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    std::fs::create_dir(root.path().join("thing")).unwrap();
    assert_eq!(
        confirm(&expectation, &existed, 0),
        vec![root.path().join("thing")]
    );
}

#[test]
fn a_failed_command_confirms_nothing_whatever_it_left_behind() {
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    std::fs::create_dir(root.path().join("thing")).unwrap();
    assert_eq!(confirm(&expectation, &existed, 1), Vec::<PathBuf>::new());
}

#[test]
fn a_folder_that_was_already_there_is_not_this_command_s() {
    // `mkdir -p old new`: exit 0, both exist, only `new` was made.
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("old")).unwrap();
    let (expectation, existed) = before(root.path(), &["old", "new"]);
    std::fs::create_dir(root.path().join("new")).unwrap();
    assert_eq!(
        confirm(&expectation, &existed, 0),
        vec![root.path().join("new")]
    );
}

#[test]
fn a_folder_expected_but_never_made_is_not_confirmed() {
    // Exit 0 without the folder: the prediction was wrong, so trust the disk.
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    assert_eq!(confirm(&expectation, &existed, 0), Vec::<PathBuf>::new());
}

#[test]
fn a_file_where_the_folder_was_expected_is_not_a_project() {
    let root = tempfile::tempdir().unwrap();
    let (expectation, existed) = before(root.path(), &["thing"]);
    std::fs::write(root.path().join("thing"), "").unwrap();
    assert_eq!(confirm(&expectation, &existed, 0), Vec::<PathBuf>::new());
}
