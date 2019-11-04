//! shared constants

use druid::kurbo::Size;

pub const CANVAS_SIZE: Size = Size::new(5000., 5000.);

/// Commands and Selectors
pub mod cmd {
    use druid::Selector;
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
}
