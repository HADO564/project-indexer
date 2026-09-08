//! Tests for [`crate::platform::app_discovery`].

use crate::platform::app_discovery::*;

#[test]
fn missing_absolute_path_is_unavailable() {
    assert!(!open_with_app_available("/definitely/not/a/real/app-xyz"));
}

#[test]
fn blank_open_with_is_unavailable() {
    assert!(!open_with_app_available(""));
    assert!(!open_with_app_available("   "));
}

#[cfg(windows)]
#[test]
fn a_bare_command_on_path_is_available() {
    // cmd.exe lives in System32, which is always on PATH on Windows.
    assert!(open_with_app_available("cmd.exe"));
    assert!(open_with_app_available("cmd"));
}

#[cfg(windows)]
#[test]
fn a_missing_absolute_exe_is_unavailable() {
    assert!(!open_with_app_available(
        r"C:\definitely\not\a\real\app-xyz.exe"
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn a_bare_command_on_path_is_available() {
    // `ls` is safe to assume present on any Linux box running these tests.
    assert!(open_with_app_available("ls"));
}

#[cfg(target_os = "linux")]
#[test]
fn a_full_command_line_checks_only_the_program() {
    assert!(open_with_app_available("ls -la /tmp"));
    assert!(!open_with_app_available(
        "/definitely/not/a/real/app-xyz -la"
    ));
}

/// Mirrors the nesting in the source: these cover `linux_impl`, which is
/// `#[cfg(target_os = "linux")]`, so they compile and run only there. They are
/// the reason the Linux lib-test count is higher than the Windows one.
#[cfg(target_os = "linux")]
mod linux_impl {
    use crate::platform::app_discovery::linux_impl::*;

    use super::*;

    fn launch(exec: &str, dir: &str) -> (String, Vec<String>) {
        let stored = exec_command(exec).expect("exec should parse");
        build_launch_args(&stored, dir).expect("stored command should launch")
    }

    #[test]
    fn keeps_arguments_of_wrapper_commands() {
        let (program, args) = launch(
            "/usr/bin/flatpak run --branch=stable --command=bottles \
             --file-forwarding com.usebottles.bottles @@u %u @@",
            "/home/me/proj",
        );
        assert_eq!(program, "/usr/bin/flatpak");
        // The directory replaces %u between the file-forwarding markers
        // rather than being appended after them.
        assert_eq!(
            args,
            [
                "run",
                "--branch=stable",
                "--command=bottles",
                "--file-forwarding",
                "com.usebottles.bottles",
                "@@u",
                "/home/me/proj",
                "@@",
            ]
        );
    }

    #[test]
    fn preserves_quoted_arguments_through_storage() {
        let (program, args) = launch(
            r#"env WINEPREFIX="/home/me/my wine/.wine" wine start /ProgIDOpen txtfile %f"#,
            "/home/me/proj",
        );
        assert_eq!(program, "env");
        assert_eq!(
            args,
            [
                "WINEPREFIX=/home/me/my wine/.wine",
                "wine",
                "start",
                "/ProgIDOpen",
                "txtfile",
                "/home/me/proj",
            ]
        );
    }

    #[test]
    fn appends_directory_when_entry_has_no_placeholder() {
        let (program, args) = launch("systemctl --user start warp-taskbar", "/home/me/proj");
        assert_eq!(program, "systemctl");
        assert_eq!(args, ["--user", "start", "warp-taskbar", "/home/me/proj"]);
    }

    #[test]
    fn drops_non_path_field_codes() {
        let (program, args) = launch("gedit %i %c --new-window %U", "/home/me/proj");
        assert_eq!(program, "gedit");
        assert_eq!(args, ["--new-window", "/home/me/proj"]);
    }

    #[test]
    fn handles_a_bare_hand_typed_program() {
        let (program, args) = build_launch_args("code", "/home/me/proj").expect("should launch");
        assert_eq!(program, "code");
        assert_eq!(args, ["/home/me/proj"]);
    }

    #[test]
    fn unescapes_literal_percent_in_exec() {
        let stored = exec_command("printit 100%% %f").expect("exec should parse");
        let (program, args) = build_launch_args(&stored, "/home/me/proj").unwrap();
        assert_eq!(program, "printit");
        assert_eq!(args, ["100%", "/home/me/proj"]);
    }

    #[test]
    fn skips_entries_that_are_not_applications() {
        let link = "[Desktop Entry]\nType=Link\nName=Docs\nURL=https://example.com\n";
        assert!(parse_desktop_entry(link).is_none());

        let hidden = "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n";
        assert!(parse_desktop_entry(hidden).is_none());
    }

    #[test]
    fn reads_name_and_exec_from_the_desktop_entry_section() {
        let entry = "[Desktop Entry]\nType=Application\nName=Editor\nExec=editor %F\n\n\
                     [Desktop Action new]\nName=New\nExec=editor --new\n";
        let app = parse_desktop_entry(entry).expect("should parse");
        assert_eq!(app.name, "Editor");
        assert_eq!(app.path, "editor %F");
    }
}
