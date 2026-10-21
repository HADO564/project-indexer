//! Custom icons as images: the SVG → pixels step, and which icons get an image.
//! A real terminal's protocol can't be asked here, so these use a half-block
//! picker — the browser itself never does, but the drawing path is the same.

use std::path::Path;

use indexer_core::IconStore;
use ratatui_image::picker::Picker;
use tempfile::TempDir;

use crate::tui::images::{may_show_images, rasterize, IconImages};

/// A red square filling a 10 × 10 view box.
pub(super) const SQUARE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="red"/></svg>"#;

/// A store in a fresh temporary folder holding `logo` (the red square).
pub(super) fn store_with_logo() -> (TempDir, IconStore) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("logo.svg");
    std::fs::write(&file, SQUARE).unwrap();
    let store = IconStore::new(dir.path().join("icons"));
    store.import(&file).unwrap();
    (dir, store)
}

fn pixel(image: &image::RgbaImage, x: u32, y: u32) -> [u8; 4] {
    image.get_pixel(x, y).0
}

#[test]
fn an_svg_fills_the_image_it_is_drawn_into() {
    let image = rasterize(SQUARE, 40, 40).expect("a valid SVG renders");
    assert_eq!(image.dimensions(), (40, 40));
    assert_eq!(pixel(&image, 20, 20), [255, 0, 0, 255]);
}

#[test]
fn a_wide_svg_is_centred_with_clear_space_above_and_below() {
    let wide = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 10"><rect width="20" height="10" fill="red"/></svg>"#;
    let image = rasterize(wide, 40, 40).unwrap();
    // Scaled to 40 × 20 and centred: rows 10..30 are drawn, the rest clear.
    assert_eq!(pixel(&image, 20, 20), [255, 0, 0, 255]);
    assert_eq!(pixel(&image, 20, 2)[3], 0, "above the icon is transparent");
    assert_eq!(pixel(&image, 20, 37)[3], 0, "below the icon is transparent");
}

#[test]
fn soft_edges_keep_their_colour() {
    // Half-transparent red must stay red, not darken toward black, once the
    // premultiplied pixels are turned back into straight ones.
    let faint = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="red" fill-opacity="0.5"/></svg>"#;
    let [r, g, b, a] = pixel(&rasterize(faint, 10, 10).unwrap(), 5, 5);
    assert!(r >= 250 && g == 0 && b == 0, "{r},{g},{b}");
    assert!((120..=135).contains(&a), "{a}");
}

#[test]
fn an_svg_that_will_not_render_gives_nothing() {
    assert!(rasterize("not an svg at all", 40, 40).is_none());
    assert!(rasterize(SQUARE, 0, 40).is_none(), "an empty image");
}

#[test]
fn only_a_custom_icon_in_the_store_gets_an_image() {
    let (_dir, store) = store_with_logo();
    let mut images = IconImages::with_picker(Picker::halfblocks(), &store);

    assert!(images.get("custom:logo").is_some());
    assert!(images.get("custom:missing").is_none(), "not in the store");
    assert!(images.get("rocket").is_none(), "a bundled icon is a glyph");
}

#[test]
fn reload_forgets_a_deleted_icon() {
    let (_dir, store) = store_with_logo();
    let mut images = IconImages::with_picker(Picker::halfblocks(), &store);
    assert!(images.get("custom:logo").is_some());

    store.delete("logo").unwrap();
    images.reload(&store);
    assert!(images.get("custom:logo").is_none());
}

#[test]
fn a_store_that_does_not_exist_has_no_images() {
    let store = IconStore::new(Path::new("/nonexistent/dexily-test-icons").to_path_buf());
    let mut images = IconImages::with_picker(Picker::halfblocks(), &store);
    assert!(images.get("custom:logo").is_none());
}

/// [`may_show_images`] for an environment of just these variables.
fn shows(vars: &[(&str, &str)]) -> bool {
    may_show_images(|name| {
        vars.iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.to_string())
    })
}

#[test]
fn terminals_known_to_show_images_are_asked() {
    assert!(shows(&[("TERM", "xterm-kitty")]));
    assert!(shows(&[("KITTY_WINDOW_ID", "1")]));
    assert!(shows(&[("TERM_PROGRAM", "WezTerm")]));
    assert!(shows(&[(
        "WEZTERM_EXECUTABLE",
        "/Applications/WezTerm.app"
    )]));
    assert!(shows(&[("TERM", "xterm-ghostty")]));
    assert!(shows(&[("TERM_PROGRAM", "iTerm.app")]));
    assert!(shows(&[("TERM", "foot")]));
}

#[test]
fn other_terminals_are_not_asked_and_so_wait_for_nothing() {
    assert!(!shows(&[]));
    assert!(!shows(&[("TERM", "xterm-256color")]));
    assert!(!shows(&[("TERM_PROGRAM", "Apple_Terminal")]));
    assert!(
        !shows(&[("KITTY_WINDOW_ID", "")]),
        "set but empty is not set"
    );
}

#[test]
fn tmux_and_screen_are_never_asked_whatever_runs_outside() {
    assert!(!shows(&[
        ("TMUX", "/tmp/tmux-501/default,1,0"),
        ("KITTY_WINDOW_ID", "1")
    ]));
    assert!(!shows(&[
        ("TERM", "tmux-256color"),
        ("WEZTERM_EXECUTABLE", "x")
    ]));
    assert!(!shows(&[
        ("TERM", "screen-256color"),
        ("ITERM_SESSION_ID", "w0")
    ]));
    assert!(!shows(&[
        ("TERM_PROGRAM", "tmux"),
        ("KITTY_WINDOW_ID", "1")
    ]));
}

#[test]
fn reload_redraws_an_icon_whose_svg_changed() {
    let (dir, store) = store_with_logo();
    let mut images = IconImages::with_picker(Picker::halfblocks(), &store);
    assert!(images.get("custom:logo").is_some());

    // The same name, now something that will not render: the old image must
    // not survive the reload.
    std::fs::write(dir.path().join("icons").join("logo.svg"), "not an svg").unwrap();
    images.reload(&store);
    assert!(images.get("custom:logo").is_none());
}
