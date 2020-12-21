//! Colors and other things that we like.

pub use druid::theme::{SELECTION_COLOR, UI_FONT};
use druid::{Color, Env, FontDescriptor, Key};

pub const SIDEBAR_BACKGROUND: Key<Color> = Key::new("runebender.sidebar-background");
pub const SIDEBAR_EDGE_STROKE: Key<Color> = Key::new("runebender.sidebar-edge-stroke");

pub const GLYPH_LIST_BACKGROUND: Key<Color> = Key::new("runebender.background");
pub const GLYPH_LIST_STROKE: Key<Color> = Key::new("runebender.glyph-list-stroke");

/// The color for placeholder glyphs
pub const PLACEHOLDER_GLYPH_COLOR: Key<Color> = Key::new("runebender.placeholder-glyph-color");
/// The color for primary text, filled glyph outlines, etc
pub const PRIMARY_TEXT_COLOR: Key<Color> = Key::new("runebender.primary-text-color");
/// The color for secondary text like less important labels
pub const SECONDARY_TEXT_COLOR: Key<Color> = Key::new("runebender.secondary-text-color");

/// The fill color of the rectangle when dragging a selection
pub const SELECTION_RECT_FILL_COLOR: Key<Color> = Key::new("runebender.selection-rect-fill-color");

/// The stroke color of the rectangle when dragging a selection
pub const SELECTION_RECT_STROKE_COLOR: Key<Color> =
    Key::new("runebender.selection-rect-stroke-color");

/// The font used for things like hovering over points
pub const UI_DETAIL_FONT: Key<FontDescriptor> = Key::new("runebender.detail-font");

pub mod colors {
    use druid::Color;

    pub const LIGHT_GREY: Color = Color::grey8(0xe7);
    pub const MEDIUM_GREY: Color = Color::grey8(0x88);
    pub const SIDEBAR_EDGE: Color = Color::grey8(0xc7);
    pub const HIGHLIGHT_COLOR: Color = Color::rgb8(0xA6, 0xCC, 0xFF);
    pub const LIGHT_BLUE: Color = Color::rgb8(0x53, 0x8B, 0xBB);
    pub const TRANSPARENT_LIGHT_GREY: Color = Color::rgba8(0xDD, 0xDD, 0xDD, 0x55);
}

pub fn configure_env(env: &mut Env) {
    env.set(SIDEBAR_BACKGROUND, colors::LIGHT_GREY);
    env.set(SIDEBAR_EDGE_STROKE, colors::SIDEBAR_EDGE);
    env.set(PLACEHOLDER_GLYPH_COLOR, Color::grey8(0xBB));
    env.set(GLYPH_LIST_STROKE, colors::LIGHT_GREY);
    env.set(GLYPH_LIST_BACKGROUND, Color::grey8(0xF0));
    env.set(PRIMARY_TEXT_COLOR, Color::BLACK);
    env.set(SECONDARY_TEXT_COLOR, colors::MEDIUM_GREY);
    env.set(SELECTION_RECT_STROKE_COLOR, colors::LIGHT_BLUE);
    env.set(SELECTION_RECT_FILL_COLOR, colors::TRANSPARENT_LIGHT_GREY);
    env.set(druid::theme::SELECTION_COLOR, colors::HIGHLIGHT_COLOR);
    env.set(druid::theme::LABEL_COLOR, Color::BLACK);
    env.set(druid::theme::WINDOW_BACKGROUND_COLOR, Color::WHITE);
    env.set(druid::theme::BACKGROUND_LIGHT, colors::LIGHT_GREY);
    env.set(druid::theme::CURSOR_COLOR, Color::BLACK);
    env.set(druid::theme::BUTTON_DARK, Color::grey8(200));
    env.set(druid::theme::BUTTON_LIGHT, Color::WHITE);
    env.set(UI_DETAIL_FONT, FontDescriptor::default().with_size(12.0));
}
