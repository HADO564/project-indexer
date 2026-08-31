use crate::models::InstalledApp;

/// Recursively deletes a directory from disk, treating an already-missing
/// directory as success (deleting is idempotent: the goal state — the
/// directory being gone — is already reached). `std::fs::remove_dir_all`
/// works identically on Windows and Linux, so no OS-specific handling is
/// needed here.
pub fn remove_directory(path: &str) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Failed to delete directory: {}", e)),
    }
}

/// Scans platform-specific sources for installed applications, used by
/// the "open with" app picker: Start Menu shortcuts and registry App
/// Paths on Windows, `.desktop` files on Linux. macOS isn't covered yet,
/// so it gets an empty list.
#[tauri::command]
pub fn list_installed_apps() -> Result<Vec<InstalledApp>, String> {
    #[cfg(windows)]
    {
        Ok(windows_impl::list_installed_apps())
    }
    #[cfg(target_os = "linux")]
    {
        Ok(linux_impl::list_installed_apps())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        Ok(Vec::new())
    }
}

/// Opens `directory`, either with the application in `open_with` or with
/// the system default when that's `None`.
///
/// Windows and macOS hand the whole thing to the opener plugin, which
/// resolves an `.exe` path or an app name the way each platform expects.
/// Linux takes its own path because the picker stores a full `.desktop`
/// command line, which the plugin would treat as one long program name.
pub fn open_in_app(directory: &str, open_with: Option<&str>) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        if let Some(command) = open_with.map(str::trim).filter(|c| !c.is_empty()) {
            return linux_impl::open_with_command(directory, command);
        }
    }

    tauri_plugin_opener::open_path(directory, open_with).map_err(|e| e.to_string())
}

/// Checks whether the app an `open_with` value points at can still be
/// launched, so a project configured to open with an app that's since been
/// uninstalled or moved can be caught before actually trying — and reported
/// as a specific "app is missing" error rather than a generic launch
/// failure.
///
/// On Linux `open_with` is a full command line (as stored by the picker, or
/// hand-typed), so only the program — the first token — is resolved. On
/// Windows/macOS it's just a path or bare command name.
pub fn open_with_app_available(open_with: &str) -> bool {
    let program = program_from_open_with(open_with);
    let program = program.trim();
    if program.is_empty() {
        return false;
    }
    command_exists(program)
}

#[cfg(target_os = "linux")]
fn program_from_open_with(open_with: &str) -> String {
    linux_impl::split_command(open_with)
        .into_iter()
        .next()
        .unwrap_or_default()
}

#[cfg(not(target_os = "linux"))]
fn program_from_open_with(open_with: &str) -> String {
    open_with.to_string()
}

/// Checks whether `program` exists: as a path on its own when it looks like
/// one (absolute, or containing a separator), otherwise by searching `PATH`
/// the way the OS would when launching a bare command name.
fn command_exists(program: &str) -> bool {
    use std::path::Path;

    let path = Path::new(program);
    if path.is_absolute() || program.contains(std::path::MAIN_SEPARATOR) {
        return path.is_file();
    }

    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    let extensions = windows_path_extensions();

    std::env::split_paths(&path_var).any(|dir| {
        dir.join(program).is_file()
            || extensions
                .iter()
                .any(|ext| dir.join(format!("{program}{ext}")).is_file())
    })
}

/// `PATHEXT` suffixes (`.EXE`, `.CMD`, …) that Windows tries in turn against
/// a bare command name when resolving it through `PATH`. Empty on other
/// platforms, where a bare name must match a `PATH` entry exactly.
#[cfg(windows)]
fn windows_path_extensions() -> Vec<String> {
    std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
        .split(';')
        .filter(|ext| !ext.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(not(windows))]
fn windows_path_extensions() -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
mod windows_impl {
    use crate::models::InstalledApp;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use winreg::{HKCU, HKLM};

    /// Keyed by lowercased resolved exe path so Start Menu and App Paths
    /// entries for the same app collapse into one, with Start Menu's
    /// friendlier name winning since it's scanned first.
    pub fn list_installed_apps() -> Vec<InstalledApp> {
        let mut apps: HashMap<String, InstalledApp> = HashMap::new();

        for dir in start_menu_dirs() {
            scan_start_menu_dir(&dir, &mut apps);
        }

        scan_app_paths(HKCU, &mut apps);
        scan_app_paths(HKLM, &mut apps);

        let mut list: Vec<InstalledApp> = apps.into_values().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    fn start_menu_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Ok(program_data) = std::env::var("ProgramData") {
            dirs.push(PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        if let Ok(app_data) = std::env::var("AppData") {
            dirs.push(PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
        dirs
    }

    fn scan_start_menu_dir(dir: &Path, apps: &mut HashMap<String, InstalledApp>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                scan_start_menu_dir(&path, apps);
                continue;
            }

            let is_lnk = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("lnk"));
            if !is_lnk {
                continue;
            }

            let Ok(lnk) = parselnk::Lnk::try_from(path.as_path()) else {
                continue;
            };
            let Some(target) = lnk
                .link_info
                .local_base_path
                .clone()
                .or_else(|| lnk.link_info.local_base_path_unicode.clone())
            else {
                continue;
            };
            if !target.to_lowercase().ends_with(".exe") {
                continue;
            }

            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };

            apps.entry(target.to_lowercase()).or_insert(InstalledApp {
                name: name.to_string(),
                path: target,
            });
        }
    }

    fn scan_app_paths(hive: &winreg::RegKey, apps: &mut HashMap<String, InstalledApp>) {
        let Ok(app_paths) =
            hive.open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths")
        else {
            return;
        };

        for key_name in app_paths.enum_keys().flatten() {
            let Ok(subkey) = app_paths.open_subkey(&key_name) else {
                continue;
            };
            let Ok(path) = subkey.get_value::<String, _>("") else {
                continue;
            };
            if !path.to_lowercase().ends_with(".exe") {
                continue;
            }

            let name = key_name
                .trim_end_matches(".exe")
                .trim_end_matches(".EXE")
                .to_string();

            apps.entry(path.to_lowercase())
                .or_insert(InstalledApp { name, path });
        }
    }
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use crate::models::InstalledApp;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    /// Keyed by desktop file ID (the basename without `.desktop`, which the
    /// Desktop Entry spec already guarantees unique per application) rather
    /// than by launch command. Several distinct apps legitimately share a
    /// leading command — every Flatpak launches through `flatpak`, every Wine
    /// association through `env` — so keying on the command collapsed them
    /// into a single picker entry. Earlier directories win, which gives a
    /// user's `~/.local/share/applications` override precedence over the
    /// system copy, matching XDG lookup order.
    pub fn list_installed_apps() -> Vec<InstalledApp> {
        let mut apps: HashMap<String, InstalledApp> = HashMap::new();

        for dir in application_dirs() {
            scan_applications_dir(&dir, &mut apps);
        }

        let mut list: Vec<InstalledApp> = apps.into_values().collect();
        list.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        list
    }

    /// Directories searched for `.desktop` files, per the XDG Base
    /// Directory / Desktop Entry specifications. Covers the Flatpak and Snap
    /// exports that Ubuntu and Fedora add to `XDG_DATA_DIRS` too, since those
    /// are ordinary entries in an `applications` subdirectory.
    fn application_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Ok(data_home) = std::env::var("XDG_DATA_HOME") {
            dirs.push(PathBuf::from(data_home).join("applications"));
        } else if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/applications"));
        }

        let data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
        for dir in data_dirs.split(':').filter(|d| !d.is_empty()) {
            dirs.push(PathBuf::from(dir).join("applications"));
        }

        dirs
    }

    fn scan_applications_dir(dir: &Path, apps: &mut HashMap<String, InstalledApp>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                scan_applications_dir(&path, apps);
                continue;
            }

            let is_desktop_file = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("desktop"));
            if !is_desktop_file {
                continue;
            }

            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            // Cheap to check before reading, and keeps a system entry from
            // overwriting the user override that was scanned first.
            if apps.contains_key(id) {
                continue;
            }

            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };

            if let Some(app) = parse_desktop_entry(&contents) {
                apps.insert(id.to_string(), app);
            }
        }
    }

    /// Parses the `[Desktop Entry]` section of a `.desktop` file into an
    /// `InstalledApp`, skipping entries that shouldn't be launchable from
    /// a picker (`NoDisplay`/`Hidden`, or a non-`Application` `Type`) or
    /// that lack a usable `Exec` line.
    fn parse_desktop_entry(contents: &str) -> Option<InstalledApp> {
        let mut in_entry_section = false;
        let mut name = None;
        let mut exec = None;
        let mut entry_type = None;

        for line in contents.lines() {
            let line = line.trim();

            if line.starts_with('[') {
                in_entry_section = line == "[Desktop Entry]";
                continue;
            }
            if !in_entry_section || line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            // Unlocalized keys only: `Name[de]` and friends don't match, so
            // the C-locale name always wins.
            match key.trim() {
                "Name" => name = Some(value.trim().to_string()),
                "Exec" => exec = Some(value.trim().to_string()),
                "Type" => entry_type = Some(value.trim().to_string()),
                "NoDisplay" | "Hidden" if value.trim().eq_ignore_ascii_case("true") => {
                    return None;
                }
                _ => {}
            }
        }

        // `Type=Link`/`Type=Directory` entries have no Exec worth launching.
        if let Some(t) = entry_type {
            if !t.eq_ignore_ascii_case("Application") {
                return None;
            }
        }

        let name = name?;
        let command = exec_command(&exec?)?;

        Some(InstalledApp {
            name,
            path: command,
        })
    }

    /// Normalizes an `Exec=` line into a command line we can launch later.
    ///
    /// The whole line is kept, not just the first token: wrapper-based
    /// entries carry their meaning in the arguments (`flatpak run <app-id>`,
    /// `env WINEPREFIX=… wine start …`, `systemctl --user start …`), so
    /// dropping them left an unlaunchable bare `flatpak`/`env`. The result is
    /// re-quoted so that [`split_command`] round-trips it back to the same
    /// arguments.
    fn exec_command(exec: &str) -> Option<String> {
        let args: Vec<String> = split_exec(exec)
            .into_iter()
            .filter(|arg| is_path_field_code(arg) || !is_droppable_field_code(arg))
            .collect();

        if args.first().is_none_or(|program| program.is_empty()) {
            return None;
        }

        Some(
            args.iter()
                .map(|arg| quote_arg(arg))
                .collect::<Vec<_>>()
                .join(" "),
        )
    }

    /// Field codes that stand in for the file(s) being opened.
    ///
    /// These are deliberately kept in the stored command so the directory can
    /// be substituted at the position the entry expects. Flatpak wraps them in
    /// file-forwarding markers (`… --file-forwarding <app-id> @@u %u @@`), so
    /// dropping the code and appending the path at the end would leave the
    /// markers unpaired and the path never forwarded into the sandbox.
    fn is_path_field_code(arg: &str) -> bool {
        matches!(arg, "%f" | "%F" | "%u" | "%U")
    }

    /// Any other single-character field code — `%i`, `%c`, `%k` and the
    /// deprecated `%d`/`%n`/`%v`/`%m` — expands to icons, captions and similar
    /// metadata we have nothing to supply, so it's dropped. `%%` is a literal
    /// percent and is already unescaped by [`split_exec`].
    fn is_droppable_field_code(arg: &str) -> bool {
        let mut chars = arg.chars();
        chars.next() == Some('%') && chars.next().is_some() && chars.next().is_none()
    }

    /// Splits a raw `Exec=` value: like [`split_command`], plus the Desktop
    /// Entry spec's `%%` escape for a literal percent.
    fn split_exec(exec: &str) -> Vec<String> {
        split_with(exec, true)
    }

    /// Splits a stored command line — one we produced with [`exec_command`],
    /// or one the user typed into the "open with" field by hand.
    ///
    /// Percent escaping is deliberately not applied here: the stored form
    /// still contains real field codes like `%u` for [`open_with_command`] to
    /// substitute, and leaving `%` alone keeps the round trip exact.
    pub fn split_command(command: &str) -> Vec<String> {
        split_with(command, false)
    }

    /// Whitespace separates arguments, double quotes group them, and a
    /// backslash inside quotes escapes the next character.
    fn split_with(command: &str, unescape_percent: bool) -> Vec<String> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut has_current = false;
        let mut in_quotes = false;
        let mut chars = command.chars();

        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    // Outside quotes a lone backslash is literal; inside, it
                    // escapes `"`, `\`, `$` and backtick.
                    match chars.next() {
                        Some(next) if in_quotes => current.push(next),
                        Some(next) => {
                            current.push('\\');
                            current.push(next);
                        }
                        None => current.push('\\'),
                    }
                    has_current = true;
                }
                '"' => {
                    in_quotes = !in_quotes;
                    has_current = true;
                }
                '%' if unescape_percent && !in_quotes => {
                    // `%%` is an escaped literal percent.
                    if chars.as_str().starts_with('%') {
                        chars.next();
                    }
                    current.push('%');
                    has_current = true;
                }
                c if c.is_whitespace() && !in_quotes => {
                    if has_current {
                        args.push(std::mem::take(&mut current));
                        has_current = false;
                    }
                }
                c => {
                    current.push(c);
                    has_current = true;
                }
            }
        }

        if has_current {
            args.push(current);
        }

        args
    }

    /// Inverse of [`split_command`] for a single argument.
    fn quote_arg(arg: &str) -> String {
        let needs_quoting = arg.is_empty()
            || arg
                .chars()
                .any(|c| c.is_whitespace() || c == '"' || c == '\\');
        if !needs_quoting {
            return arg.to_string();
        }

        let mut quoted = String::with_capacity(arg.len() + 2);
        quoted.push('"');
        for c in arg.chars() {
            if c == '"' || c == '\\' {
                quoted.push('\\');
            }
            quoted.push(c);
        }
        quoted.push('"');
        quoted
    }

    /// Resolves a stored command line and a directory into the program and
    /// argument list to spawn.
    fn build_launch_args(command: &str, directory: &str) -> Result<(String, Vec<String>), String> {
        let mut args = split_command(command);
        if args.is_empty() {
            return Err(format!("'{}' is not a runnable command", command));
        }
        let program = args.remove(0);

        // Substitute the directory for the entry's file placeholder, keeping
        // any surrounding markers intact. Commands without a placeholder —
        // including anything the user typed by hand, like a bare `code` —
        // just take it as a trailing argument.
        let mut substituted = false;
        for arg in &mut args {
            if is_path_field_code(arg) {
                *arg = directory.to_string();
                substituted = true;
            }
        }
        if !substituted {
            args.push(directory.to_string());
        }

        Ok((program, args))
    }

    /// Launches `directory` with a specific application.
    ///
    /// `open::with_detached`, which the opener plugin uses, runs the whole
    /// `open_with` string as a single program name, so it can't launch the
    /// multi-argument commands real `.desktop` entries use. Splitting the
    /// stored command line ourselves and spawning it directly is what makes
    /// Flatpak, Snap and Wine entries work.
    pub fn open_with_command(directory: &str, command: &str) -> Result<(), String> {
        use std::os::unix::process::CommandExt as _;
        use std::process::{Command, Stdio};

        let (program, args) = build_launch_args(command, directory)?;

        let mut child = Command::new(&program)
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            // Own process group, so the editor we launch isn't taken down by
            // a Ctrl-C or terminal hangup aimed at this app.
            .process_group(0)
            .spawn()
            .map_err(|e| format!("Failed to launch '{}': {}", program, e))?;

        // Reap in the background: the child outlives this call, and without a
        // wait it would linger as a zombie for the lifetime of the app.
        std::thread::spawn(move || {
            let _ = child.wait();
        });

        Ok(())
    }

    #[cfg(test)]
    mod tests {
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
            let (program, args) =
                build_launch_args("code", "/home/me/proj").expect("should launch");
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
