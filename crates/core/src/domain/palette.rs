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

/// What a *project* may store: a palette name, or a hex literal from the
/// colour picker. A group is still palette-only — its colour drives a sidebar
/// entry and a card edge, where staying inside the theme matters most.
pub fn is_valid_project_color(name: &str) -> bool {
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
    fn a_project_may_take_a_palette_name_or_a_hex_literal_but_a_group_may_not() {
        assert!(is_valid_project_color("cyan"));
        assert!(is_valid_project_color("#e7b64e"));
        assert!(!is_valid_project_color("chartreuse"));

        // Groups stay palette-only: their colour drives sidebar entries and
        // card edges, where staying inside the theme matters most.
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
