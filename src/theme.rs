//! Colors and other things that we like.

pub use druid::theme::{FONT_NAME, SELECTION_COLOR};
use druid::{Color, Env, Key};

pub const SIDEBAR_BACKGROUND: Key<Color> = Key::new("runebender.sidebar-background");
pub const SIDEBAR_EDGE_STROKE: Key<Color> = Key::new("runebender.sidebar-edge-stroke");

pub const GLYPH_LIST_BACKGROUND: Key<Color> = Key::new("runebender.background");
pub const GLYPH_LIST_STROKE: Key<Color> = Key::new("runebender.glyph-list-stroke");
pub const GLYPH_LIST_LABEL_TEXT_SIZE: Key<f64> = Key::new("runebender.glyph-list-label-font-size");

pub const PLACEHOLDER_GLYPH_COLOR: Key<Color> = Key::new("runebender.placeholder-color");
pub const GLYPH_COLOR: Key<Color> = Key::new("runebender.glyph-color");

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
}
