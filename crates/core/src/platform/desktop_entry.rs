//! The freedesktop Desktop Entry format: parsing a `.desktop` file, and the
//! `Exec` line grammar it defines — field codes, splitting, shell quoting.
//!
//! Its own module because both halves of app support depend on it and neither
//! owns it: discovery parses entries to find applications, launching rebuilds
//! a command line from what discovery stored. Linux only.

use crate::domain::InstalledApp;

/// Parses the `[Desktop Entry]` section of a `.desktop` file into an
/// `InstalledApp`, skipping entries that shouldn't be launchable from
/// a picker (`NoDisplay`/`Hidden`, or a non-`Application` `Type`) or
/// that lack a usable `Exec` line.
pub(crate) fn parse_desktop_entry(contents: &str) -> Option<InstalledApp> {
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
pub(crate) fn exec_command(exec: &str) -> Option<String> {
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
pub(crate) fn is_path_field_code(arg: &str) -> bool {
    matches!(arg, "%f" | "%F" | "%u" | "%U")
}

/// Any other single-character field code — `%i`, `%c`, `%k` and the
/// deprecated `%d`/`%n`/`%v`/`%m` — expands to icons, captions and similar
/// metadata we have nothing to supply, so it's dropped. `%%` is a literal
/// percent and is already unescaped by [`split_exec`].
pub(crate) fn is_droppable_field_code(arg: &str) -> bool {
    let mut chars = arg.chars();
    chars.next() == Some('%') && chars.next().is_some() && chars.next().is_none()
}

/// Splits a raw `Exec=` value: like [`split_command`], plus the Desktop
/// Entry spec's `%%` escape for a literal percent.
pub(crate) fn split_exec(exec: &str) -> Vec<String> {
    split_with(exec, true)
}

/// Whitespace separates arguments, double quotes group them, and a
/// backslash inside quotes escapes the next character.
pub(crate) fn split_with(command: &str, unescape_percent: bool) -> Vec<String> {
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
pub(crate) fn quote_arg(arg: &str) -> String {
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
