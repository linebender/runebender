//! Colors and other things that we like.

pub use druid::theme::{SELECTION_COLOR, UI_FONT};
use druid::{Color, Env, FontDescriptor, Key};

pub const SIDEBAR_BACKGROUND: Key<Color> = Key::new("runebender.sidebar-background");
pub const SIDEBAR_EDGE_STROKE: Key<Color> = Key::new("runebender.sidebar-edge-stroke");

pub const GLYPH_LIST_BACKGROUND: Key<Color> = Key::new("runebender.background");
pub const GLYPH_LIST_STROKE: Key<Color> = Key::new("runebender.glyph-list-stroke");
pub const GLYPH_LIST_LABEL_TEXT_SIZE: Key<f64> = Key::new("runebender.glyph-list-label-font-size");

pub const PLACEHOLDER_GLYPH_COLOR: Key<Color> = Key::new("runebender.placeholder-color");
pub const GLYPH_COLOR: Key<Color> = Key::new("runebender.glyph-color");

/// The font used for things like hovering over points
pub const UI_DETAIL_FONT: Key<FontDescriptor> = Key::new("runebender.detail-font");

pub mod colors {
    use druid::Color;

    pub const LIGHT_GREY: Color = Color::rgb8(0xe7, 0xe7, 0xe7);
    pub const SIDEBAR_EDGE: Color = Color::rgb8(0xc7, 0xc7, 0xc7);
    pub const HIGHLIGHT_COLOR: Color = Color::rgb8(0xA6, 0xCC, 0xFF);
}

pub fn configure_env(env: &mut Env) {
    env.set(SIDEBAR_BACKGROUND, colors::LIGHT_GREY);
    env.set(SIDEBAR_EDGE_STROKE, colors::SIDEBAR_EDGE);
    env.set(PLACEHOLDER_GLYPH_COLOR, colors::LIGHT_GREY);
    env.set(GLYPH_LIST_STROKE, colors::LIGHT_GREY);
    env.set(GLYPH_LIST_BACKGROUND, Color::WHITE);
    env.set(GLYPH_LIST_LABEL_TEXT_SIZE, 12.0);
    env.set(GLYPH_COLOR, Color::BLACK);
    env.set(druid::theme::SELECTION_COLOR, colors::HIGHLIGHT_COLOR);
    env.set(druid::theme::LABEL_COLOR, Color::BLACK);
    env.set(druid::theme::WINDOW_BACKGROUND_COLOR, Color::WHITE);
    env.set(druid::theme::BACKGROUND_LIGHT, colors::LIGHT_GREY);
    env.set(druid::theme::CURSOR_COLOR, Color::BLACK);
    env.set(druid::theme::BUTTON_DARK, Color::grey8(200));
    env.set(druid::theme::BUTTON_LIGHT, Color::WHITE);
    env.set(UI_DETAIL_FONT, FontDescriptor::default().with_size(1.0));
}
