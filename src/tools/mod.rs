//! A tool accepts user input and modifies the canvas.

mod ellipse;
mod knife;
mod pen;
mod preview;
mod rectangle;
mod select;

pub use ellipse::Ellipse;
pub use knife::Knife;
pub use pen::Pen;
pub use preview::Preview;
pub use rectangle::Rectangle;
pub use select::Select;

use crate::edit_session::EditSession;
use crate::mouse::{Mouse, TaggedEvent};
use druid::{Env, EventCtx, KeyEvent, PaintCtx};

/// Something to pass around instead of a Box<dyn Tool>
pub type ToolId = &'static str;

/// Types of state modifications, for the purposes of undo.
///
/// Certain state modifications group together in undo; for instance when dragging
/// a point, each individual edit (each time we receive a `MouseMouved`` event)
/// is combined into a single edit representing the entire drag.
///
/// When a `Tool` handles an event, it returns an `Option<EditType>`, that describes
/// what (if any) sort of modification it made to the state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditType {
    /// Any change that always gets its own undo group
    Normal,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    /// An edit where a drag of some kind is in progress.
    Drag,
    /// An edit that finishes a drag; it combines with the previous undo
    /// group, but not with any subsequent event.
    DragUp,
}

/// A trait for representing the logic of a tool; that is, something that handles
/// mouse and keyboard events, and modifies the current [`EditSession`].
pub trait Tool {
    /// Called once per `paint()` call in the editor widget, this gives tools
    /// an opportunity to draw on the canvas.
    ///
    /// As an example, the `Select` (arrow) widget uses this to paint the current
    /// selection rectangle, if a drag gesture is in progress.
    ///
    /// # Note:
    ///
    /// When drawing, coordinates in 'design space' may need to be converted to
    /// 'screen space'; conversion methods are available via the [`ViewPort`]
    /// at `data.viewport`.
    ///
    /// [`EditSession`]: struct.EditSession.html
    /// [`ViewPort`]: struct.ViewPort.html
    #[allow(unused)]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, env: &Env) {}
    /// Called on each key_down event in the parent.
    #[allow(unused)]
    fn key_down(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        env: &Env,
    ) -> Option<EditType> {
        None
    }
    /// Called on each key_up event in the parent.
    #[allow(unused)]
    fn key_up(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        env: &Env,
    ) -> Option<EditType> {
        None
    }

    /// Called whenever a tool is first activated, so that it can access or modify
    /// mouse settings.
    #[allow(unused)]
    fn init_mouse(&mut self, mouse: &mut Mouse) {}

    /// Called with each mouse event. The `mouse` argument is a reference to a [`Mouse`]
    /// struct that is shared between all tools; a particular `Tool` can implement the
    /// [`MouseDelegate`] trait and pass the events to `Mouse` instance.
    ///
    /// [`Mouse`]: struct.Mouse.html
    /// [`MouseDelegate`]: ../mouse/trait.MouseDelegate.html
    #[allow(unused)]
    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        env: &Env,
    ) -> Option<EditType> {
        None
    }

    fn name(&self) -> ToolId;
}

/// Returns the tool for the given `ToolId`.
pub fn tool_for_id(id: ToolId) -> Option<Box<dyn Tool>> {
    match id {
        "Preview" => Some(Box::new(Preview::default())),
        "Pen" => Some(Box::new(Pen::default())),
        "Select" => Some(Box::new(Select::default())),
        "Rectangle" => Some(Box::new(Rectangle::default())),
        "Ellipse" => Some(Box::new(Ellipse::default())),
        "Knife" => Some(Box::new(Knife::default())),
        _ => None,
    }
}

impl EditType {
    pub fn needs_new_undo_group(self, other: EditType) -> bool {
        match (self, other) {
            (EditType::NudgeDown, EditType::NudgeDown) => false,
            (EditType::NudgeUp, EditType::NudgeUp) => false,
            (EditType::NudgeLeft, EditType::NudgeLeft) => false,
            (EditType::NudgeRight, EditType::NudgeRight) => false,
            (EditType::Drag, EditType::Drag) => false,
            _ => true,
        }
    }
}
