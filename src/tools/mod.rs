//! A tool accepts user input and modifies the canvas.

mod select;
pub use select::Select;

use crate::edit_session::EditSession;
use crate::mouse::{Mouse, TaggedEvent};
use druid::{Env, EventCtx, KeyEvent, PaintCtx};

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
    #[allow(unused)]
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, env: &Env) {}
    /// Called on each key_down event in the parent.
    #[allow(unused)]
    fn key_down(&mut self, key: &KeyEvent, ctx: &mut EventCtx, data: &mut EditSession, env: &Env) {}
    /// Called on each key_up event in the parent.
    #[allow(unused)]
    fn key_up(&mut self, key: &KeyEvent, ctx: &mut EventCtx, data: &mut EditSession, env: &Env) {}
    /// Called with each mouse event. The `mouse` argument is a reference to a [`Mouse`]
    /// struct that is shared between all tools; a particular `Tool` can implement the
    /// [`MouseDelegate`] trait and pass the events to `Mouse` instance.
    #[allow(unused)]
    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        env: &Env,
    ) {
    }
}
