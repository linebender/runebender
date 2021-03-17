//! The 'preview' tool
//!
//! This is generally represented as the 'hand', and allows the user to pan around
//! the workspace by clicking and dragging, although whether this makes sense
//! in the era of the touchpad is an open question.

use druid::widget::prelude::*;
use druid::{Cursor, MouseEvent};

use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::tools::{EditType, Tool, ToolId};

/// The state of the preview tool.
#[derive(Debug, Default, Clone)]
pub struct Preview {
    state: State,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Ready,
    Dragging,
}

impl Tool for Preview {
    fn name(&self) -> ToolId {
        "Preview"
    }

    fn cancel(
        &mut self,
        mouse: &mut Mouse,
        _ctx: &mut EventCtx,
        data: &mut EditSession,
    ) -> Option<EditType> {
        mouse.cancel(data, self);
        None
    }

    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _env: &Env,
    ) -> Option<EditType> {
        let pre_state = self.state;
        mouse.mouse_event(event, data, self);
        if pre_state != self.state {
            #[allow(deprecated)]
            match self.state {
                State::Ready => ctx.set_cursor(&Cursor::OpenHand),
                State::Dragging => ctx.set_cursor(&Cursor::OpenHand),
            }
        }
        None
    }

    #[allow(deprecated)]
    fn default_cursor(&self) -> Cursor {
        Cursor::OpenHand
    }
}

impl MouseDelegate<EditSession> for Preview {
    fn cancel(&mut self, _data: &mut EditSession) {
        self.state = State::Ready;
    }

    fn left_down(&mut self, _event: &MouseEvent, _data: &mut EditSession) {
        self.state = State::Dragging;
    }

    fn left_up(&mut self, _event: &MouseEvent, _data: &mut EditSession) {
        self.state = State::Ready;
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        let offset = data.viewport.offset();
        let delta = drag.current.pos - drag.prev.pos;
        data.viewport.set_offset(offset + delta);
    }
}

impl Default for State {
    fn default() -> Self {
        State::Ready
    }
}
