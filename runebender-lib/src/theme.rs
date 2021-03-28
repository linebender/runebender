//! Colors and other things that we like.

pub use druid::theme::{
    BACKGROUND_LIGHT, BUTTON_DARK, BUTTON_LIGHT, CURSOR_COLOR, LABEL_COLOR,
    SELECTED_TEXT_BACKGROUND_COLOR, UI_FONT, WINDOW_BACKGROUND_COLOR,
};
use druid::{Color, Data, Env, FontDescriptor, Key, Widget};

// NOTE: Set the RB_THEME_PATH environment variable during compilation to change
// the default theme path.
include!(concat!(env!("OUT_DIR"), "/theme_path.rs"));

pub const SIDEBAR_BACKGROUND: Key<Color> = Key::new("runebender.sidebar-background");
pub const SIDEBAR_EDGE_STROKE: Key<Color> = Key::new("runebender.sidebar-edge-stroke");

pub const GLYPH_LIST_BACKGROUND: Key<Color> = Key::new("runebender.background");
pub const GLYPH_LIST_STROKE: Key<Color> = Key::new("runebender.glyph-list-stroke");

/// Colors for the root window glyph grids cells.
pub const GLYPH_GRID_CELL_BACKGROUND_COLOR: Key<Color> =
    Key::new("runebender.glyph-grid-cell-background-color");
pub const GLYPH_GRID_CELL_OUTLINE_COLOR: Key<Color> =
    Key::new("runebender.glyph-grid-cell-outline-color");
pub const FOCUS_BACKGROUND_COLOR: Key<Color> = Key::new("runebender.focus-background-color");
pub const FOCUS_OUTLINE_COLOR: Key<Color> = Key::new("runebender.focus-outline-color");

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

pub const PATH_STROKE_COLOR: Key<Color> = Key::new("runebender.path-stroke-color");
pub const PATH_FILL_COLOR: Key<Color> = Key::new("runebender.path-fill-color");
pub const METRICS_COLOR: Key<Color> = Key::new("runebender.metrics-color");
pub const GUIDE_COLOR: Key<Color> = Key::new("runebender.guide-color");
pub const SELECTED_GUIDE_COLOR: Key<Color> = Key::new("runebender.selected-guide-color");
pub const SELECTED_LINE_SEGMENT_COLOR: Key<Color> =
    Key::new("runebender.selected-line-segment-color");
pub const SELECTED_POINT_INNER_COLOR: Key<Color> =
    Key::new("runebender.selected-point-inner-color");
pub const SELECTED_POINT_OUTER_COLOR: Key<Color> =
    Key::new("runebender.selected-point-outer-color");
pub const SMOOTH_POINT_OUTER_COLOR: Key<Color> = Key::new("runebender.smooth-point-outer-color");
pub const SMOOTH_POINT_INNER_COLOR: Key<Color> = Key::new("runebender.smooth-point-inner-color");
pub const CORNER_POINT_OUTER_COLOR: Key<Color> = Key::new("runebender.corner-point-outer-color");
pub const CORNER_POINT_INNER_COLOR: Key<Color> = Key::new("runebender.corner-point-inner-color");
pub const OFF_CURVE_POINT_OUTER_COLOR: Key<Color> =
    Key::new("runebender.off-curve-point-outer-color");
pub const OFF_CURVE_POINT_INNER_COLOR: Key<Color> =
    Key::new("runebender.off-curve-point-inner-color");
pub const OFF_CURVE_HANDLE_COLOR: Key<Color> = Key::new("runebender.off-curve-handle-color");
pub const DIRECTION_ARROW_COLOR: Key<Color> = Key::new("runebender.direction-arrow-color");
pub const COMPONENT_FILL_COLOR: Key<Color> = Key::new("runebender.component-fill-color");

// Colors used by tools in the tool menu
pub const KNIFE_GUIDE: Key<Color> = Key::new("runebender.knife-guide");
pub const KNIFE_GUIDE_INTERSECTION: Key<Color> = Key::new("runebender.knife-guide-intersection");

pub const SMOOTH_RADIUS: Key<f64> = Key::new("runebender.smooth-point-radius");
pub const SMOOTH_SELECTED_RADIUS: Key<f64> = Key::new("runebender.smooth-point-selected-radius");
pub const CORNER_RADIUS: Key<f64> = Key::new("runebender.corner-point-radius");
pub const CORNER_SELECTED_RADIUS: Key<f64> = Key::new("runebender.corner-point-selected-radius");
pub const OFF_CURVE_RADIUS: Key<f64> = Key::new("runebender.off-curve-point-radius");
pub const OFF_CURVE_SELECTED_RADIUS: Key<f64> =
    Key::new("runebender.off-curve-point-selected-radius");

pub fn configure_env(env: &mut Env) {
    env.set(UI_DETAIL_FONT, FontDescriptor::default().with_size(12.0));
}

druid_theme_loader::loadable_theme!(pub MyTheme {
    SIDEBAR_BACKGROUND,
    SIDEBAR_EDGE_STROKE,
    PLACEHOLDER_GLYPH_COLOR,
    GLYPH_LIST_STROKE,
    GLYPH_LIST_BACKGROUND,
    GLYPH_GRID_CELL_BACKGROUND_COLOR,
    GLYPH_GRID_CELL_OUTLINE_COLOR,
    FOCUS_BACKGROUND_COLOR,
    FOCUS_OUTLINE_COLOR,
    PRIMARY_TEXT_COLOR,
    SECONDARY_TEXT_COLOR,
    SELECTION_RECT_STROKE_COLOR,
    SELECTION_RECT_FILL_COLOR,
    SELECTED_TEXT_BACKGROUND_COLOR,
    LABEL_COLOR,
    WINDOW_BACKGROUND_COLOR,
    BACKGROUND_LIGHT,
    CURSOR_COLOR,
    BUTTON_DARK,
    BUTTON_LIGHT,
    PATH_STROKE_COLOR,
    PATH_FILL_COLOR,
    METRICS_COLOR,
    GUIDE_COLOR,
    SELECTED_GUIDE_COLOR,
    SELECTED_LINE_SEGMENT_COLOR,
    SELECTED_POINT_INNER_COLOR,
    SELECTED_POINT_OUTER_COLOR,
    SMOOTH_POINT_OUTER_COLOR,
    SMOOTH_POINT_INNER_COLOR,
    CORNER_POINT_OUTER_COLOR,
    CORNER_POINT_INNER_COLOR,
    OFF_CURVE_POINT_OUTER_COLOR,
    OFF_CURVE_POINT_INNER_COLOR,
    OFF_CURVE_HANDLE_COLOR,
    DIRECTION_ARROW_COLOR,
    COMPONENT_FILL_COLOR,
    KNIFE_GUIDE,
    KNIFE_GUIDE_INTERSECTION,
    SMOOTH_RADIUS,
    SMOOTH_SELECTED_RADIUS,
    CORNER_RADIUS,
    CORNER_SELECTED_RADIUS,
    OFF_CURVE_RADIUS,
    OFF_CURVE_SELECTED_RADIUS,

});

pub fn wrap_in_theme_loader<T: Data>(widget: impl Widget<T>) -> impl Widget<T> {
    druid_theme_loader::ThemeLoader::new(THEME_FILE_PATH, MyTheme, widget)
}
