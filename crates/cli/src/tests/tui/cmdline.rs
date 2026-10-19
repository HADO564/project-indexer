//! Tokenizing the `:` line and trimming a clap error to one line. Running a
//! command is exercised through the real binary, not here — it needs a
//! database — so these cover the pure parsing helpers.

use crate::tui::cmdline::{first_line, needs_confirm, tokenize};

#[test]
fn tokenize_splits_on_whitespace() {
    assert_eq!(tokenize("open app").unwrap(), ["open", "app"]);
    assert_eq!(
        tokenize("  edit   app  --name  x ").unwrap(),
        ["edit", "app", "--name", "x"]
    );
    assert!(tokenize("   ").unwrap().is_empty());
}

#[test]
fn tokenize_keeps_quoted_values_together() {
    assert_eq!(
        tokenize("open \"build at 12:30\"").unwrap(),
        ["open", "build at 12:30"]
    );
    assert_eq!(
        tokenize("edit app --name 'New Name'").unwrap(),
        ["edit", "app", "--name", "New Name"]
    );
}

#[test]
fn tokenize_rejects_an_unclosed_quote() {
    assert_eq!(tokenize("open \"unterminated"), Err("unclosed quote"));
}

#[test]
fn needs_confirm_flags_only_the_destructive_commands() {
    // Deletes / untrack: gated.
    assert!(needs_confirm("untrack app").is_some());
    assert!(needs_confirm("purge app").is_some());
    assert!(needs_confirm("group delete Work").is_some());
    // Everything else: no gate.
    assert!(needs_confirm("favorite app").is_none());
    assert!(needs_confirm("open app").is_none());
    assert!(needs_confirm("edit app --name x").is_none());
    assert!(needs_confirm("group create Work").is_none());
    assert!(needs_confirm("list").is_none());
    // Unparseable input is not gated (it won't run either).
    assert!(needs_confirm("nonsense").is_none());
}

#[test]
fn first_line_takes_the_error_line() {
    assert_eq!(
        first_line("error: unexpected argument\n\nUsage: dexily open <QUERY>"),
        "error: unexpected argument"
    );
}
