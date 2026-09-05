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
    fn there_are_eight_swatches_and_they_are_unique() {
        let mut sorted = SWATCHES.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 8);
    }
}
