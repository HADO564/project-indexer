//! Custom icons as images, on terminals that can show one.
//!
//! A custom icon is an SVG, and no terminal draws SVG. Many draw images,
//! though — Kitty, WezTerm, Ghostty, iTerm2 and the sixel terminals — each in a
//! protocol of its own. So the browser asks the terminal once, at start, which
//! protocol it speaks ([`IconImages::detect`]); turns each icon's SVG into
//! pixels with `resvg` ([`rasterize`]); and hands those to `ratatui-image`,
//! which encodes them for that protocol.
//!
//! Only a terminal that says what it is in its environment, and is one known to
//! show images, is asked ([`may_show_images`]): one that never answers would
//! hold the first frame back until the question timed out. tmux is never asked
//! — an image there shows only with passthrough on, which can't be told from
//! inside, and asking would turn it on behind the user's back.
//!
//! Where any of that fails — a terminal with no image protocol (macOS
//! Terminal), tmux, an SVG that will not render — the browser draws the folder
//! glyph, as it always has. That is the TUI only: `list` and `show` print
//! text, and stay as they are.

use std::collections::HashMap;
use std::io::IsTerminal;
use std::time::Duration;

use image::{DynamicImage, RgbaImage};
use indexer_core::IconStore;
use ratatui::layout::Size;
use ratatui_image::picker::cap_parser::QueryStdioOptions;
use ratatui_image::picker::{Picker, ProtocolType};
use ratatui_image::protocol::Protocol;
use ratatui_image::Resize;
use resvg::{tiny_skia, usvg};

use crate::appearance;

/// How many columns and rows the detail pane gives an icon: about square,
/// since a cell is roughly twice as tall as it is wide.
pub(crate) const ICON_COLS: u16 = 8;
pub(crate) const ICON_ROWS: u16 = 4;

/// How long a terminal that should know the question gets to answer it. A
/// local one answers in milliseconds; this is for one over a slow link.
const QUERY_TIMEOUT: Duration = Duration::from_millis(1000);

/// The custom icons, ready to draw in the terminal's own image protocol.
pub(crate) struct IconImages {
    picker: Picker,
    /// Each icon's SVG, by name, as the store held it at the last load.
    svgs: HashMap<String, String>,
    /// What each icon became, once asked for: `None` for one that would not
    /// render, so it is not tried again on every frame.
    drawn: HashMap<String, Option<Protocol>>,
}

impl IconImages {
    /// Asks the terminal whether it can show images, and if it can, loads the
    /// icons from `store`. `None` when it can't, and the browser draws glyphs.
    ///
    /// Call this with the browser's screen up and before reading any key: the
    /// question and its answer travel over stdin and stdout. Only a terminal
    /// [`may_show_images`] names is asked, and an answer of half blocks — far
    /// too coarse for an icon a few cells wide — counts as no. With stdout not
    /// a terminal there is no one to ask, so it doesn't.
    pub(crate) fn detect(store: &IconStore) -> Option<Self> {
        if !std::io::stdout().is_terminal() || !may_show_images(|name| std::env::var(name).ok()) {
            return None;
        }
        let options = QueryStdioOptions {
            timeout: QUERY_TIMEOUT,
            ..QueryStdioOptions::default()
        };
        let picker = Picker::from_query_stdio_with_options(options).ok()?;
        if picker.protocol_type() == ProtocolType::Halfblocks {
            return None;
        }
        Some(Self::with_picker(picker, store))
    }

    /// The icons in `store`, drawn through `picker`. [`detect`](Self::detect)
    /// is the way in for a real terminal; tests pass a half-block picker.
    pub(crate) fn with_picker(picker: Picker, store: &IconStore) -> Self {
        let mut images = Self {
            picker,
            svgs: HashMap::new(),
            drawn: HashMap::new(),
        };
        images.reload(store);
        images
    }

    /// Reads the icons again, for one added, changed or deleted since. A store
    /// that can't be read leaves no icons, so every project falls back to its
    /// glyph. An icon whose SVG is unchanged keeps its image: a reload follows
    /// every write to the database, and redrawing each one sends it to the
    /// terminal all over again.
    pub(crate) fn reload(&mut self, store: &IconStore) {
        let svgs: HashMap<String, String> = store
            .list()
            .map(|icons| {
                icons
                    .into_iter()
                    .map(|icon| (icon.name, icon.svg))
                    .collect()
            })
            .unwrap_or_default();
        let old = &self.svgs;
        self.drawn.retain(|name, _| old.get(name) == svgs.get(name));
        self.svgs = svgs;
    }

    /// The image for a project's `icon`, when it is a custom icon that exists
    /// and renders; `None` for a bundled icon or any failure.
    pub(crate) fn get(&mut self, icon: &str) -> Option<&Protocol> {
        let name = appearance::custom_name(icon)?;
        if !self.drawn.contains_key(name) {
            let drawn = self.svgs.get(name).and_then(|svg| self.draw(svg));
            self.drawn.insert(name.to_string(), drawn);
        }
        self.drawn.get(name)?.as_ref()
    }

    /// `svg` at exactly the size the detail pane gives it, in pixels of this
    /// terminal's font, so `ratatui-image` has nothing left to resize.
    fn draw(&self, svg: &str) -> Option<Protocol> {
        let font = self.picker.font_size();
        let width = u32::from(ICON_COLS) * u32::from(font.width);
        let height = u32::from(ICON_ROWS) * u32::from(font.height);
        let pixels = rasterize(svg, width, height)?;
        self.picker
            .new_protocol(
                DynamicImage::ImageRgba8(pixels),
                Size::new(ICON_COLS, ICON_ROWS),
                Resize::Fit(None),
            )
            .ok()
    }
}

/// Whether the environment names a terminal known to show images, so it is
/// worth asking which protocol. `var` reads a variable (`std::env::var` in the
/// browser), so tests can hand in their own.
///
/// Never inside tmux or screen, whatever the outer terminal: see the module
/// comment. `TERM` survives `ssh`, so a kitty or ghostty session over SSH still
/// counts.
pub(crate) fn may_show_images(var: impl Fn(&str) -> Option<String>) -> bool {
    let set = |name: &str| var(name).is_some_and(|value| !value.is_empty());
    let term = var("TERM").unwrap_or_default();
    let program = var("TERM_PROGRAM").unwrap_or_default();
    if set("TMUX") || term.starts_with("tmux") || term.starts_with("screen") || program == "tmux" {
        return false;
    }
    set("KITTY_WINDOW_ID")
        || set("WEZTERM_EXECUTABLE")
        || set("ITERM_SESSION_ID")
        || set("GHOSTTY_RESOURCES_DIR")
        || set("KONSOLE_VERSION")
        || ["xterm-kitty", "xterm-ghostty", "wezterm"].contains(&term.as_str())
        || term.starts_with("foot")
        || ["WezTerm", "iTerm.app", "ghostty"].contains(&program.as_str())
}

/// `svg` drawn into a `width` × `height` image: scaled to fit, centred, and
/// transparent around it. `None` for an SVG that will not parse, has no size,
/// or for an empty image.
///
/// Drawn as it is, untinted, as the app draws a custom icon — there it is an
/// `<img>`, which cannot take the project's colour either.
pub(crate) fn rasterize(svg: &str, width: u32, height: u32) -> Option<RgbaImage> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let scale = (width as f32 / size.width()).min(height as f32 / size.height());
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    let dx = (width as f32 - size.width() * scale) / 2.0;
    let dy = (height as f32 - size.height() * scale) / 2.0;
    let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
    let transform = tiny_skia::Transform::from_row(scale, 0.0, 0.0, scale, dx, dy);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // tiny-skia keeps its colours premultiplied by alpha; an image wants them
    // straight, or every soft edge comes out dark.
    let data = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let c = pixel.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    RgbaImage::from_raw(width, height, data)
}
