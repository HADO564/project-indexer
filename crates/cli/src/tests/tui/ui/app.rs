//! The browser's drawing, rendered to a `TestBackend` so the panes are
//! exercised without a terminal — this catches a layout that panics or a label
//! that goes missing, which the pure-state tests in `tests/tui/app.rs` cannot.

use indexer_core::domain::{Group, Project};
use indexer_core::IconStore;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui_image::picker::Picker;
use serde_json::json;

use crate::tui::app::App;
use crate::tui::images::IconImages;
use crate::tui::ui::app::draw;

fn press(app: &mut App, code: KeyCode) {
    app.handle(KeyEvent::new(code, KeyModifiers::NONE));
}

fn project(name: &str, group_id: Option<&str>) -> Project {
    serde_json::from_value(json!({
        "id": format!("{name}-0000-4000-8000-000000000000"),
        "name": name,
        "directory": format!("/home/me/{name}"),
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "last_opened_at": null,
        "trackers": [],
        "group_id": group_id,
    }))
    .unwrap()
}

fn group(id: &str, name: &str) -> Group {
    serde_json::from_value(json!({
        "id": id,
        "name": name,
        "color": "cyan",
        "icon": "briefcase",
        "position": 0,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
    }))
    .unwrap()
}

fn sample() -> App {
    App::new(
        vec![project("alpha", None), project("bravo", Some("g-work"))],
        vec![],
        vec![group("g-work", "Work")],
    )
}

fn render(app: &App, width: u16, height: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| draw(frame, app, None)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn renders_the_three_panes_the_status_and_the_hint() {
    let text = render(&sample(), 90, 20).join("\n");
    for needle in [
        "VIEWS",     // sidebar: the fixed views block
        "All",       //   its first entry
        "GROUPS",    // sidebar: the groups block
        "Ungrouped", //   the no-group bucket, leading the groups
        "Work",      //   the one group
        "PROJECTS",  // the list pane title
        "alpha",     //   a project in the All view
        "DETAIL",    // the detail pane
        "projects",  // the status line ("2 projects · All")
        "quit",      // the hint line
    ] {
        assert!(text.contains(needle), "expected {needle:?} in:\n{text}");
    }
}

#[test]
fn does_not_panic_in_a_small_terminal() {
    // A cramped terminal must still render — ratatui clips, it does not crash.
    let _ = render(&sample(), 20, 6);
}

#[test]
fn the_action_menu_overlays_the_panes() {
    let mut app = sample();
    press(&mut app, KeyCode::Char(' '));
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("Actions"), "{text}");
    assert!(text.contains("Open"), "{text}");
}

#[test]
fn the_help_overlay_lists_the_keys() {
    let mut app = sample();
    press(&mut app, KeyCode::Char('?'));
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("Keys"), "{text}");
    assert!(text.contains("in Groups: new / rename / delete"), "{text}");
    assert!(text.contains("add a project here"), "{text}");
    // The last row still fits a 20-line terminal.
    assert!(text.contains("quit"), "{text}");
}

#[test]
fn the_groups_pane_hints_its_own_keys() {
    let mut app = sample();
    press(&mut app, KeyCode::Char(']')); // Ungrouped
    let text = render(&app, 110, 20).join("\n");
    assert!(text.contains("n new group"), "{text}");
    assert!(
        !text.contains("e rename"),
        "Ungrouped can't be renamed:\n{text}"
    );
    press(&mut app, KeyCode::Char('j')); // Work
    let text = render(&app, 110, 20).join("\n");
    assert!(text.contains("e rename · d delete"), "{text}");
}

#[test]
fn the_name_dialog_says_what_it_names() {
    let mut app = sample();
    press(&mut app, KeyCode::Char(']'));
    press(&mut app, KeyCode::Char('n'));
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("New group"), "{text}");
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Char('e'));
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("Rename \"Work\""), "{text}");
    assert!(text.contains("> Work"), "{text}");
}

#[test]
fn the_scan_dialog_overlays_the_panes() {
    let mut app = sample();
    press(&mut app, KeyCode::Char('s'));
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("Scan a folder"), "{text}");
}

#[test]
fn the_folder_browser_overlays_the_panes() {
    let mut app = sample();
    app.set_browse(
        std::path::PathBuf::from("/home/me/code"),
        vec!["api".into(), "web".into()],
    );
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("Scan —"), "{text}");
    assert!(text.contains("api/"), "{text}");
}

#[test]
fn the_search_line_renders_and_filters_the_list() {
    let mut app = sample(); // alpha (ungrouped), bravo (Work)
    press(&mut app, KeyCode::Char('/'));
    for ch in "alpha".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("/alpha"), "search line missing:\n{text}");
    assert!(
        !text.contains("bravo"),
        "bravo should be filtered out:\n{text}"
    );
}

#[test]
fn an_empty_view_says_so() {
    // No projects at all: the All view is empty, so the list pane names the
    // emptiness rather than leaving a blank frame.
    let app = App::new(vec![], vec![], vec![]);
    let text = render(&app, 90, 20).join("\n");
    assert!(text.contains("no projects in this view"), "{text}");
}

/// One project carrying every field the detail pane can show beyond the
/// basics: a description, two properties and two lines of notes.
fn described() -> App {
    let project = serde_json::from_value(json!({
        "id": "shop-0000-4000-8000-000000000000",
        "name": "shop",
        "description": "Storefront",
        "directory": "/home/me/shop",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "last_opened_at": null,
        "trackers": [],
        "properties": { "client": "acme", "stage": "beta" },
        "notes": "ship on Friday\nask about the logo",
    }))
    .unwrap();
    App::new(vec![project], vec![], vec![])
}

#[test]
fn the_detail_pane_shows_the_description_properties_and_notes() {
    let text = render(&described(), 100, 24).join("\n");
    for needle in [
        "description",
        "Storefront",
        "properties",
        "client = acme",
        "stage = beta", // the second property, on a line of its own
        "notes",
        "ship on Friday",
        "ask about the logo", // the second line of the notes
    ] {
        assert!(text.contains(needle), "expected {needle:?} in:\n{text}");
    }
}

#[test]
fn a_long_description_wraps_rather_than_being_cut_off() {
    let mut app = described();
    let mut projects = vec![app.selected_project().unwrap().clone()];
    projects[0].description = "A storefront for the spring sale, with a basket and zebra".into();
    app.set_data(projects, vec![], vec![]);

    // The detail pane is 36 columns wide; the last word only shows if the
    // line wraps onto the next.
    let text = render(&app, 100, 24).join("\n");
    assert!(
        text.contains("zebra"),
        "expected the end of the description in:\n{text}"
    );
}

#[test]
fn fields_a_project_does_not_have_are_left_out() {
    // `alpha` has no description, properties or notes: no empty rows for them.
    let text = render(&sample(), 100, 24).join("\n");
    for absent in ["description", "properties", "notes"] {
        assert!(
            !text.contains(absent),
            "did not expect {absent:?} in:\n{text}"
        );
    }
}

/// `alpha`, wearing the custom icon `logo`, as the only project.
fn wearing_logo() -> App {
    let mut alpha = project("alpha", None);
    alpha.icon = Some("custom:logo".to_string());
    App::new(vec![alpha], vec![], vec![])
}

/// The frame `app` draws with `images`, 100 × `height`.
fn frame_with(app: &App, images: &mut IconImages, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, height)).unwrap();
    terminal
        .draw(|frame| draw(frame, app, Some(images)))
        .unwrap();
    terminal.backend().buffer().clone()
}

/// The text of each row of `buffer`'s detail pane: its last 36 columns.
fn detail_text(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (64..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// The detail pane's rows that hold any of the icon's red — the image, which
/// half blocks paint as coloured cells.
fn red_rows(buffer: &Buffer) -> Vec<u16> {
    let red = ratatui::style::Color::Rgb(255, 0, 0);
    (0..buffer.area.height)
        .filter(|&y| {
            (64..buffer.area.width).any(|x| buffer[(x, y)].bg == red || buffer[(x, y)].fg == red)
        })
        .collect()
}

#[test]
fn a_custom_icon_is_drawn_above_the_details_when_images_work() {
    let (_dir, store) = crate::tests::tui::images::store_with_logo();
    let mut images = IconImages::with_picker(Picker::halfblocks(), &store);
    let buffer = frame_with(&wearing_logo(), &mut images, 24);
    let text = detail_text(&buffer);

    // Rows 1–3 inside the border are the image; the name comes after a gap.
    assert_eq!(red_rows(&buffer), [1, 2, 3], "{}", text.join("\n"));
    assert!(
        text[5].contains("alpha"),
        "expected the name below it:\n{}",
        text.join("\n")
    );
}

#[test]
fn without_images_a_custom_icon_is_the_title_glyph_as_before() {
    let text = render(&wearing_logo(), 100, 24);
    let pane: Vec<String> = text
        .iter()
        .map(|row| row.chars().skip(64).collect())
        .collect();
    assert!(pane[1].contains("alpha"), "{}", pane.join("\n"));
}

#[test]
fn an_icon_that_is_not_in_the_store_falls_back_too() {
    let store = IconStore::new(std::path::PathBuf::from("/nonexistent/dexily-test-icons"));
    let mut images = IconImages::with_picker(Picker::halfblocks(), &store);
    let buffer = frame_with(&wearing_logo(), &mut images, 24);
    assert!(red_rows(&buffer).is_empty());
    assert!(detail_text(&buffer)[1].contains("alpha"));
}

#[test]
fn a_short_pane_leaves_the_image_out_rather_than_crowd_the_details() {
    let (_dir, store) = crate::tests::tui::images::store_with_logo();
    let mut images = IconImages::with_picker(Picker::halfblocks(), &store);
    // 8 rows: a 6-row pane, no room for a 3-row icon and the details too.
    let buffer = frame_with(&wearing_logo(), &mut images, 8);
    assert!(red_rows(&buffer).is_empty());
    assert!(detail_text(&buffer)[1].contains("alpha"));
}
