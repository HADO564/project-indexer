//! Tests for [`crate::platform::desktop_entry`] — parsing `.desktop` files
//! and the `Exec` line grammar. Linux only, as the module is.

#![cfg(target_os = "linux")]

use crate::platform::desktop_entry::*;

#[test]
fn reads_name_and_exec_from_the_desktop_entry_section() {
    let entry = "[Desktop Entry]\nType=Application\nName=Editor\nExec=editor %F\n\n\
                 [Desktop Action new]\nName=New\nExec=editor --new\n";
    let app = parse_desktop_entry(entry).expect("should parse");
    assert_eq!(app.name, "Editor");
    assert_eq!(app.path, "editor %F");
}

#[test]
fn skips_entries_that_are_not_applications() {
    let link = "[Desktop Entry]\nType=Link\nName=Docs\nURL=https://example.com\n";
    assert!(parse_desktop_entry(link).is_none());

    let hidden = "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n";
    assert!(parse_desktop_entry(hidden).is_none());
}
