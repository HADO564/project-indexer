//! A project's own colour and icon, as `edit --color` / `--icon` and the full
//! form take them: checked here, before core, so a typo is refused with the
//! choices rather than stored.
//!
//! Core checks a colour itself but refuses only with the bad value; an icon it
//! does not check at all, and the GUI draws an unknown one as a folder, so a
//! mistyped `--icon rockt` would save and quietly show the wrong glyph.

use anyhow::bail;
use indexer_core::domain::palette::{is_valid_color, SWATCHES};
use indexer_core::IconStore;

use crate::paths;

/// The GUI's bundled icons, by name: `ICON_NAMES` in `src/lib/icons.ts`.
/// Core does not own that list, so this is a copy, and
/// `bundled_icons_match_the_app` fails when the two drift apart.
pub const BUNDLED_ICONS: [&str; 25] = [
    "folder",
    "briefcase",
    "code",
    "terminal",
    "gamepad",
    "palette",
    "book",
    "music",
    "camera",
    "film",
    "flask",
    "rocket",
    "globe",
    "database",
    "server",
    "cpu",
    "box",
    "layers",
    "pen",
    "wrench",
    "heart",
    "star",
    "flag",
    "home",
    "trash",
];

/// How a user-supplied icon is spelled in a project: `custom:<name>`.
const CUSTOM_PREFIX: &str = "custom:";

/// A typed colour as `UpdateProject.color` takes its inner box: `None` to
/// clear it, else a palette name or `#rrggbb`, lowercased, as the GUI stores
/// them — `Cyan` and `#FF8800` are accepted as typed and saved as `cyan` and
/// `#ff8800`.
pub fn color(typed: &str) -> anyhow::Result<Option<String>> {
    let color = typed.trim().to_lowercase();
    if color.is_empty() {
        return Ok(None);
    }
    if !is_valid_color(&color) {
        bail!(
            "not a colour: \"{}\". Use one of {}, or #rrggbb",
            typed.trim(),
            SWATCHES.join(", ")
        );
    }
    Ok(Some(color))
}

/// A typed icon as `UpdateProject.icon` takes its inner box: `None` to clear
/// it, else a bundled name or `custom:<name>` for one of `custom`, the icons
/// added in the app.
pub fn icon(typed: &str, custom: &[String]) -> anyhow::Result<Option<String>> {
    let icon = typed.trim();
    if icon.is_empty() {
        return Ok(None);
    }
    if let Some(name) = icon.strip_prefix(CUSTOM_PREFIX) {
        // The app stores every custom icon under a lowercase name, so
        // `custom:Logo` means `custom:logo`.
        let name = name.to_lowercase();
        if custom.contains(&name) {
            return Ok(Some(format!("{CUSTOM_PREFIX}{name}")));
        }
        if custom.is_empty() {
            bail!("no custom icon named \"{name}\": none have been added in the app yet");
        }
        bail!(
            "no custom icon named \"{name}\". Custom icons: {}",
            custom.join(", ")
        );
    }
    let lower = icon.to_lowercase();
    if BUNDLED_ICONS.contains(&lower.as_str()) {
        return Ok(Some(lower));
    }
    bail!(
        "not an icon: \"{icon}\". Use one of {}, or custom:<name> for one added in the app",
        BUNDLED_ICONS.join(", ")
    );
}

/// [`icon`] against the app's own custom icons, read only for a `custom:`
/// one, so a bundled icon never touches the store.
pub fn checked_icon(typed: &str) -> anyhow::Result<Option<String>> {
    let custom = if typed.trim().starts_with(CUSTOM_PREFIX) {
        custom_icons()?
    } else {
        Vec::new()
    };
    icon(typed, &custom)
}

/// The names of the icons added in the app, from the store it keeps beside
/// the database. A store never written to is empty, not an error.
pub fn custom_icons() -> anyhow::Result<Vec<String>> {
    let store = IconStore::new(paths::config_dir()?.join("icons"));
    Ok(store.list()?.into_iter().map(|icon| icon.name).collect())
}

/// The colour as red, green and blue, for the form's swatch: a palette name's
/// shade as the app's dark theme paints it, or a `#rrggbb` as written.
/// `None` for anything else.
pub fn rgb(color: &str) -> Option<(u8, u8, u8)> {
    let hex = match color {
        // `--color-swatch-*` in `src/app.css`.
        "cyan" => "#56c8c4",
        "gold" => "#e7b64e",
        "amber" => "#e2903a",
        "rust" => "#e2605a",
        "violet" => "#a98cf0",
        "green" => "#7fc98a",
        "blue" => "#6fa8e6",
        "pink" => "#e58bbd",
        other => other,
    };
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let channel = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}
