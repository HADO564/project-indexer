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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_swatches_are_recognized() {
        assert!(is_known_swatch("cyan"));
        assert!(is_known_swatch("pink"));
    }

    #[test]
    fn unknown_swatch_is_rejected() {
        assert!(!is_known_swatch("chartreuse"));
        assert!(!is_known_swatch("#e7b64e"));
        assert!(!is_known_swatch(""));
    }

    #[test]
    fn hex_literals_are_accepted_only_in_the_exact_six_digit_form() {
        assert!(is_hex_literal("#e7b64e"));
        assert!(is_hex_literal("#FFFFFF"));

        // Everything else is refused rather than normalised — these values end
        // up in a `style` attribute.
        assert!(!is_hex_literal("#abc"));
        assert!(!is_hex_literal("#e7b64"));
        assert!(!is_hex_literal("#e7b64ee"));
        assert!(!is_hex_literal("e7b64e"));
        assert!(!is_hex_literal("#gggggg"));
        assert!(!is_hex_literal("red"));
        assert!(!is_hex_literal("rgb(1,2,3)"));
        assert!(!is_hex_literal("#e7b64e; background: url(x)"));
        assert!(!is_hex_literal(""));
    }

    #[test]
    fn a_stored_colour_is_a_palette_name_or_a_hex_literal_and_nothing_else() {
        assert!(is_valid_color("cyan"));
        assert!(is_valid_color("#e7b64e"));

        assert!(!is_valid_color("chartreuse"));
        assert!(!is_valid_color("red"));
        assert!(!is_valid_color("#abc"));
        assert!(!is_valid_color("#e7b64e; background: url(x)"));
        assert!(!is_valid_color(""));

        // is_known_swatch stays the narrower question — "is this one of the
        // eight?" — which the pickers use to decide what to highlight.
        assert!(!is_known_swatch("#e7b64e"));
    }

    #[test]
    fn there_are_eight_swatches_and_they_are_unique() {
        let mut sorted = SWATCHES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 8);
    }
}
