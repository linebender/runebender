//! shared constants

use druid::kurbo::Size;
use druid::FormatId;

pub const CANVAS_SIZE: Size = Size::new(5000., 5000.);
pub const GLYPHS_APP_PASTEBOARD_TYPE: FormatId = "Glyphs elements pasteboard type";

/// Commands and Selectors
pub mod cmd {
    use druid::kurbo::Point;
    use druid::Selector;
    use norad::GlyphName;

    use crate::path::EntityId;
    use crate::tools::ToolId;

    /// Sent when windows should rebuild their menus.
    pub const REBUILD_MENUS: Selector = Selector::new("runebender.rebuid-menus");

    /// sent by the 'delete' menu item
    pub const DELETE: Selector = Selector::new("runebender.delete");

    /// sent by the 'select' menu item
    pub const SELECT_ALL: Selector = Selector::new("runebender.select-all");

    /// sent by the 'deselect' menu item
    pub const DESELECT_ALL: Selector = Selector::new("runebender.deselect-all");

    /// sent by the 'new glyph' menu item
    pub const NEW_GLYPH: Selector = Selector::new("runebender.new-glyph");

    /// sent by the 'delete glyph' menu item
    pub const DELETE_SELECTED_GLYPH: Selector = Selector::new("runebender.delete-selected-glyph");

    /// Sent to the root to rename a glyph.
    ///
    /// The arguments **must** be a `RenameGlyphArgs`
    pub const RENAME_GLYPH: Selector<RenameGlyphArgs> = Selector::new("runebender.rename-glyph");

    /// Arguments passed with the RENAME_GLYPH command.
    pub struct RenameGlyphArgs {
        pub old: GlyphName,
        pub new: GlyphName,
    }

    /// sent by the 'add component' menu item
    pub const ADD_COMPONENT: Selector = Selector::new("runebender.add-component");

    /// Sent when a new tool has been selected.
    ///
    /// The payload must be a `ToolId`.
    pub const SET_TOOL: Selector<ToolId> = Selector::new("runebender.set-tool");

    /// Sent when the preview tool is toggled  temporarily.
    ///
    /// This is normally bound to spacebar.
    ///
    /// The argument should be a bool indicating whether this is a keydown (true)
    /// or a keyup (false).
    pub const TOGGLE_PREVIEW_TOOL: Selector<bool> = Selector::new("runebender.tool-preview-toggle");

    /// Sent when the 'zoom in' menu item is selected
    pub const ZOOM_IN: Selector = Selector::new("runebender.zoom-in");

    /// Sent when the 'zoom out' menu item is selected
    pub const ZOOM_OUT: Selector = Selector::new("runebender.zoom-out");

    /// Sent when the 'reset zoom' menu item is selected
    pub const ZOOM_DEFAULT: Selector = Selector::new("runebender.zoom-default");

    /// Sent when the 'add guide' context menu item is selected
    ///
    /// The arguments **must** be a `Point`, where the guide will be added.
    pub const ADD_GUIDE: Selector<Point> = Selector::new("runebender.add-guide");

    /// Sent when the 'toggle guide' context menu item is selected
    ///
    /// The arguments **must** be a `ToggleGuideCmdArgs`.
    pub const TOGGLE_GUIDE: Selector<ToggleGuideCmdArgs> = Selector::new("runebender.toggle-guide");

    /// Arguments passed along with the TOGGLE_GUIDE command
    pub struct ToggleGuideCmdArgs {
        pub id: EntityId,
        pub pos: Point,
    }

    /// A hack: asks the editor view to take focus, so that it can handle
    /// keyboard events.
    ///
    /// This is sent by the `EditorController` when focus is changing to 'no widget',
    /// as might happen after we finish editing a coordinate via a text field.
    pub const TAKE_FOCUS: Selector = Selector::new("runebender.editor-steal-focus");
}
