//! A project's own colour and icon, as `edit --color` / `--icon` and the full
//! form take them: checked here, before core, so a typo is refused with the
//! choices rather than stored.
//!
//! Core checks a colour itself but refuses only with the bad value; an icon it
//! does not check at all, and the GUI draws an unknown one as a folder, so a
//! mistyped `--icon rockt` would save and quietly show the wrong glyph.

use anyhow::bail;
use clap::ValueEnum;
use indexer_core::domain::palette::{is_valid_color, SWATCHES};
use indexer_core::IconStore;
use serde::{Deserialize, Serialize};

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

/// Whether `typed` names a custom icon rather than a bundled one.
pub fn is_custom(typed: &str) -> bool {
    typed.trim().starts_with(CUSTOM_PREFIX)
}

/// [`icon`] against the app's own custom icons, read only for a `custom:`
/// one, so a bundled icon never touches the store.
pub fn checked_icon(typed: &str) -> anyhow::Result<Option<String>> {
    let custom = if is_custom(typed) {
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

/// How icons are drawn in a terminal, which can only draw characters — never
/// the app's SVGs. Set with `config icons`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IconStyle {
    /// No icons: the default, since no program can tell which font a
    /// terminal uses, and a missing glyph would show as `□`.
    #[default]
    Off,
    /// Nerd Font glyphs: line icons like the app's, drawn in the project's
    /// colour. Needs a Nerd Font in the terminal.
    Nerd,
    /// Emoji: shown almost everywhere, but in their own colours.
    Emoji,
}

impl IconStyle {
    /// The name a user types for this style, as `config icons` prints it.
    pub fn name(self) -> String {
        self.to_possible_value()
            .map(|value| value.get_name().to_string())
            .unwrap_or_default()
    }
}

/// The character `icon` is drawn as in `style`, or `None` with icons off.
/// A custom icon is an SVG, which no terminal can draw, and an unknown name
/// is one this build has never heard of: both fall back to the folder, as the
/// app falls back to it for an unknown name.
pub fn glyph(icon: &str, style: IconStyle) -> Option<&'static str> {
    let table = match style {
        IconStyle::Off => return None,
        IconStyle::Nerd => &NERD_GLYPHS,
        IconStyle::Emoji => &EMOJI_GLYPHS,
    };
    let at = BUNDLED_ICONS
        .iter()
        .position(|&name| name == icon)
        .unwrap_or(0);
    Some(table[at])
}

/// One per [`BUNDLED_ICONS`], in its order: Font Awesome's glyphs, which every
/// Nerd Font carries at these code points.
const NERD_GLYPHS: [&str; 25] = [
    "\u{f07b}", // folder
    "\u{f0b1}", // briefcase
    "\u{f121}", // code
    "\u{f120}", // terminal
    "\u{f11b}", // gamepad
    "\u{f1fc}", // palette (a paint brush)
    "\u{f02d}", // book
    "\u{f001}", // music
    "\u{f030}", // camera
    "\u{f008}", // film
    "\u{f0c3}", // flask
    "\u{f135}", // rocket
    "\u{f0ac}", // globe
    "\u{f1c0}", // database
    "\u{f233}", // server
    "\u{f2db}", // cpu (a microchip)
    "\u{f1b2}", // box (a cube)
    "\u{f24d}", // layers (two sheets)
    "\u{f040}", // pen
    "\u{f0ad}", // wrench
    "\u{f004}", // heart
    "\u{f005}", // star
    "\u{f024}", // flag
    "\u{f015}", // home
    "\u{f1f8}", // trash
];

/// One per [`BUNDLED_ICONS`], in its order. Only emoji drawn as emoji without
/// a variation selector, which terminals disagree on the width of: a table
/// measuring one width and a terminal drawing another would fall out of line.
const EMOJI_GLYPHS: [&str; 25] = [
    "📁", "💼", "📜", "💻", "🎮", "🎨", "📖", "🎵", "📷", "🎬", "🧪", "🚀", "🌐", "💾", "🔌", "🧠",
    "📦", "📚", "📝", "🔧", "💖", "⭐", "🚩", "🏠", "🚮",
];

/// The colour a project's mark is drawn in, as the app does: its own, else its
/// group's. `None` when neither has one this can draw.
pub fn mark_rgb(own: Option<&str>, group: Option<&str>) -> Option<(u8, u8, u8)> {
    own.and_then(rgb).or_else(|| group.and_then(rgb))
}
