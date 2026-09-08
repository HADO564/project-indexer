/// The swatch names a project or group colour may take.
///
/// Stored values are these bare names, never literal colours — `src/lib/palette.ts`
/// resolves each to a `var(--color-swatch-*)` token at render, so a theme swap
/// recolours every project and group coherently. That file is a hand-maintained
/// mirror of this list; change one, change the other.
pub const SWATCHES: [&str; 8] = [
    "cyan", "gold", "amber", "rust", "violet", "green", "blue", "pink",
];

pub fn is_known_swatch(name: &str) -> bool {
    SWATCHES.contains(&name)
}

/// Exactly `#` followed by six hex digits — nothing else.
///
/// The strictness is the point, not pedantry: a stored colour is interpolated
/// into a `style` attribute by the frontend, and `style-src` still carries
/// `unsafe-inline`. Accepting only this shape is what guarantees a stored
/// value cannot carry arbitrary CSS. Shorthand (`#abc`), named CSS colours and
/// `rgb()` are all deliberately refused rather than normalised.
pub fn is_hex_literal(name: &str) -> bool {
    let Some(digits) = name.strip_prefix('#') else {
        return false;
    };
    digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_hexdigit())
}

/// What a project or a group may store: a palette name, or a hex literal from
/// the colour picker.
///
/// A palette name is still the better answer — it is what lets a future theme
/// recolour everything coherently — but that is a reason to offer the palette
/// first, not a reason to refuse a colour someone actually wants.
pub fn is_valid_color(name: &str) -> bool {
    is_known_swatch(name) || is_hex_literal(name)
}
