//! Colours the user can pick for highlighted text, such as the project folder
//! in `indexer list`.
//!
//! Two families:
//!
//! - **The 16 standard terminal colours.** Every terminal supports them, and
//!   the terminal's own theme decides the exact shade, so they always match
//!   the user's setup.
//! - **Named 24-bit colours**, ordered around the colour wheel. They look the
//!   same everywhere but need a terminal with true-colour support; most modern
//!   ones have it (iTerm2, WezTerm, Kitty, Alacritty, Windows Terminal, VS Code).
//!
//! The command-line value is the variant name in kebab-case: `BrightRed` is
//! `--folder-color bright-red`.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Serialised under the same kebab-case names the flag accepts, so the
/// settings file reads `"folder_color": "hot-pink"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Color {
    // The 16 standard terminal colours.
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    #[value(alias = "gray", alias = "grey")]
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,

    // Reds and pinks.
    Crimson,
    Rose,
    HotPink,
    Pink,
    Salmon,
    Coral,

    // Oranges and browns.
    Orange,
    Amber,
    Chocolate,
    Brown,
    Tan,

    // Yellows.
    Gold,
    Khaki,
    Beige,

    // Greens.
    Olive,
    Chartreuse,
    Lime,
    Mint,
    Emerald,
    Forest,

    // Cyans and blues.
    Teal,
    Turquoise,
    Sky,
    Azure,
    Cobalt,
    Navy,

    // Purples.
    Indigo,
    Purple,
    Violet,
    Orchid,
    Plum,
    Lavender,

    // Neutrals.
    Slate,
    Silver,
}

impl Color {
    /// Used when neither `--folder-color` nor a saved setting picks one.
    pub const DEFAULT: Color = Color::Cyan;

    /// The name a user types for this colour, e.g. `"hot-pink"`.
    pub fn name(self) -> String {
        self.to_possible_value()
            .map(|value| value.get_name().to_string())
            .unwrap_or_default()
    }

    /// The SGR parameters that switch the foreground to this colour: the part
    /// between `\x1b[` and `m`.
    pub fn sgr(self) -> String {
        match self.code() {
            Code::Ansi(n) => n.to_string(),
            Code::Rgb(r, g, b) => format!("38;2;{r};{g};{b}"),
        }
    }

    /// `text` in this colour and bold, then back to normal.
    pub fn paint(self, text: &str) -> String {
        format!("\x1b[1;{}m{text}\x1b[0m", self.sgr())
    }

    fn code(self) -> Code {
        use Code::{Ansi, Rgb};
        match self {
            Color::Black => Ansi(30),
            Color::Red => Ansi(31),
            Color::Green => Ansi(32),
            Color::Yellow => Ansi(33),
            Color::Blue => Ansi(34),
            Color::Magenta => Ansi(35),
            Color::Cyan => Ansi(36),
            Color::White => Ansi(37),
            Color::BrightBlack => Ansi(90),
            Color::BrightRed => Ansi(91),
            Color::BrightGreen => Ansi(92),
            Color::BrightYellow => Ansi(93),
            Color::BrightBlue => Ansi(94),
            Color::BrightMagenta => Ansi(95),
            Color::BrightCyan => Ansi(96),
            Color::BrightWhite => Ansi(97),

            Color::Crimson => Rgb(220, 20, 60),
            Color::Rose => Rgb(255, 0, 127),
            Color::HotPink => Rgb(255, 105, 180),
            Color::Pink => Rgb(255, 182, 193),
            Color::Salmon => Rgb(250, 128, 114),
            Color::Coral => Rgb(255, 127, 80),

            Color::Orange => Rgb(255, 165, 0),
            Color::Amber => Rgb(255, 191, 0),
            Color::Chocolate => Rgb(210, 105, 30),
            Color::Brown => Rgb(165, 42, 42),
            Color::Tan => Rgb(210, 180, 140),

            Color::Gold => Rgb(255, 215, 0),
            Color::Khaki => Rgb(240, 230, 140),
            Color::Beige => Rgb(245, 245, 220),

            Color::Olive => Rgb(128, 128, 0),
            Color::Chartreuse => Rgb(127, 255, 0),
            Color::Lime => Rgb(50, 205, 50),
            Color::Mint => Rgb(152, 255, 152),
            Color::Emerald => Rgb(80, 200, 120),
            Color::Forest => Rgb(34, 139, 34),

            Color::Teal => Rgb(0, 128, 128),
            Color::Turquoise => Rgb(64, 224, 208),
            Color::Sky => Rgb(135, 206, 235),
            Color::Azure => Rgb(0, 127, 255),
            Color::Cobalt => Rgb(0, 71, 171),
            Color::Navy => Rgb(0, 0, 128),

            Color::Indigo => Rgb(75, 0, 130),
            Color::Purple => Rgb(160, 32, 240),
            Color::Violet => Rgb(238, 130, 238),
            Color::Orchid => Rgb(218, 112, 214),
            Color::Plum => Rgb(221, 160, 221),
            Color::Lavender => Rgb(181, 126, 220),

            Color::Slate => Rgb(112, 128, 144),
            Color::Silver => Rgb(192, 192, 192),
        }
    }
}

enum Code {
    /// A standard terminal colour; the terminal's theme picks the shade.
    Ansi(u8),
    /// An exact 24-bit colour.
    Rgb(u8, u8, u8),
}
