//! Colors and other things that we like.

pub use druid::theme::{
    BACKGROUND_LIGHT, BUTTON_DARK, BUTTON_LIGHT, CURSOR_COLOR, LABEL_COLOR, SELECTION_COLOR,
    UI_FONT, WINDOW_BACKGROUND_COLOR,
};
use druid::{Color, Data, Env, FontDescriptor, Key, Widget};

const THEME_FILE_PATH: &str = "resources/default.theme";

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

pub fn configure_env(env: &mut Env) {
    env.set(UI_DETAIL_FONT, FontDescriptor::default().with_size(12.0));
}

druid_theme_loader::loadable_theme!(pub MyTheme {
    SIDEBAR_BACKGROUND,
    SIDEBAR_EDGE_STROKE,
    PLACEHOLDER_GLYPH_COLOR,
    GLYPH_LIST_STROKE,
    GLYPH_LIST_BACKGROUND,
    PRIMARY_TEXT_COLOR,
    SECONDARY_TEXT_COLOR,
    SELECTION_RECT_STROKE_COLOR,
    SELECTION_RECT_FILL_COLOR,
    SELECTION_COLOR,
    LABEL_COLOR,
    WINDOW_BACKGROUND_COLOR,
    BACKGROUND_LIGHT,
    CURSOR_COLOR,
    BUTTON_DARK,
    BUTTON_LIGHT,
});

pub fn wrap_in_theme_loader<T: Data>(widget: impl Widget<T>) -> impl Widget<T> {
    druid_theme_loader::ThemeLoader::new(THEME_FILE_PATH, MyTheme, widget)
}
