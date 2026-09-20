//! `add [directory]`: its argument, and how a path becomes the string stored.

use std::path::PathBuf;

use clap::Parser;

use crate::commands::add::{absolute, AddArgs};
use crate::commands::Command;
use crate::{Cli, Invocation};

/// The parsed `AddArgs`, or a panic naming what came back instead.
fn parsed(argv: &[&str]) -> AddArgs {
    let cli = Cli::try_parse_from(argv).expect("arguments should parse");
    match cli.invocation {
        Some(Invocation::Command(Command::Add(args))) => args,
        other => panic!("expected `add`, got {other:?}"),
    }
}

#[test]
fn the_directory_is_optional() {
    assert_eq!(parsed(&["indexer", "add"]).directory, None);
    assert_eq!(
        parsed(&["indexer", "add", "/home/me/code/app"]).directory,
        Some(PathBuf::from("/home/me/code/app")),
    );
}

#[test]
fn no_directory_means_the_current_one() {
    // Compared against canonicalising `"."` rather than against `current_dir`,
    // because that is the promise: both spellings of "here" go through the
    // same call, so a symlinked working directory cannot produce two strings.
    let here = std::fs::canonicalize(".").unwrap();
    assert_eq!(absolute(None).unwrap(), here.to_string_lossy());
}

#[test]
fn a_relative_path_is_stored_absolute() {
    let dir = tempfile::tempdir().unwrap();
    let child = dir.path().join("app");
    std::fs::create_dir(&child).unwrap();

    let stored = absolute(Some(child.join("..").join("app"))).unwrap();

    assert_eq!(
        stored,
        std::fs::canonicalize(&child).unwrap().to_string_lossy()
    );
    assert!(
        !stored.contains(".."),
        "the database outlives the shell, so `..` must be resolved: {stored}"
    );
}

#[test]
fn a_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Cargo.toml");
    std::fs::write(&file, "").unwrap();

    let error = absolute(Some(file)).unwrap_err().to_string();

    assert!(
        error.contains("is not a directory"),
        "canonicalising succeeds for a file, so `add` has to reject it: {error}"
    );
}

#[test]
fn a_missing_directory_is_refused() {
    let dir = tempfile::tempdir().unwrap();

    let error = absolute(Some(dir.path().join("nope")))
        .unwrap_err()
        .to_string();

    assert!(
        error.contains("cannot track"),
        "a typo should be reported, not tracked: {error}"
    );
}
