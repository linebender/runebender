//! shared constants

use druid::kurbo::Size;

pub const CANVAS_SIZE: Size = Size::new(5000., 5000.);

/// Commands and Selectors
pub mod cmd {
    use druid::kurbo::Point;
    use druid::Selector;

    use crate::path::EntityId;

    /// Hack. Sent at launch to the editor widget, so it knows to request keyboard focus.
    pub const REQUEST_FOCUS: Selector = Selector::new("runebender.request-focus");

    /// sent by the 'delete' menu item
    pub const DELETE: Selector = Selector::new("runebender.delete");

    /// sent by the 'select' menu item
    pub const SELECT_ALL: Selector = Selector::new("runebender.select-all");

    /// sent by the 'deselect' menu item
    pub const DESELECT_ALL: Selector = Selector::new("runebender.deselect-all");

    /// sent by the 'add component' menu item
    pub const ADD_COMPONENT: Selector = Selector::new("runebender.add-component");

    /// Sent when the 'select' tool should be activated
    pub const SELECT_TOOL: Selector = Selector::new("runebender.tool-select");

    /// Sent when the 'pen' tool should be activated
    pub const PEN_TOOL: Selector = Selector::new("runebender.tool-pen");

    /// Sent when the 'zoom in' menu item is selected
    pub const ZOOM_IN: Selector = Selector::new("runebender.zoom-in");

    /// Sent when the 'zoom out' menu item is selected
    pub const ZOOM_OUT: Selector = Selector::new("runebender.zoom-out");

    /// Sent when the 'reset zoom' menu item is selected
    pub const ZOOM_DEFAULT: Selector = Selector::new("runebender.zoom-default");

    /// Sent when the 'add guide' context menu item is selected
    ///
    /// The arguments **must** be a `Point`, where the guide will be added.
    pub const ADD_GUIDE: Selector = Selector::new("runebender.add-guide");

    /// Sent when the 'toggle guide' context menu item is selected
    ///
    /// The arguments **must** be a `ToggleGuideCmdArgs`.
    pub const TOGGLE_GUIDE: Selector = Selector::new("runebender.toggle-guide");

    /// Sent when the 'Copy As Code' menu item is selected.
    pub const COPY_AS_CODE: Selector = Selector::new("runebender.copy-as-code");

    /// Arguments passed along with the TOGGLE_GUIDE command
    pub struct ToggleGuideCmdArgs {
        pub id: EntityId,
        pub pos: Point,
    }
}
