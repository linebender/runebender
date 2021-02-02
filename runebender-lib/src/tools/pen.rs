//! The bezier (and hyperbezier!) pen tool.

use druid::{Env, EventCtx, KbKey, KeyEvent, MouseEvent};

use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::Path;
use crate::point::EntityId;
use crate::tools::{EditType, Tool, ToolId};

/// The state of the pen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Pen {
    hyperbezier_mode: bool,
    this_edit_type: Option<EditType>,
    state: State,
}

impl Pen {
    pub fn cubic() -> Self {
        Pen {
            hyperbezier_mode: false,
            ..Default::default()
        }
    }

    pub fn hyper() -> Self {
        Pen {
            hyperbezier_mode: true,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Ready,
    /// The mouse is down and has added a new point.
    AddPoint(EntityId),
    /// The mouse is dragging a handle after adding a new point.
    DragHandle(EntityId),
}

impl MouseDelegate<EditSession> for Pen {
    fn cancel(&mut self, _canvas: &mut EditSession) {
        self.this_edit_type = match &self.state {
            State::DragHandle(..) => Some(EditType::DragUp),
            _ => None,
        };
        self.state = State::Ready;
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        let vport = data.viewport;
        assert!(matches!(self.state, State::Ready));
        if event.count == 1 {
            let hit = data.hit_test_filtered(event.pos, None, |_| true);
            if let Some(hit) = hit {
                if let Some(path) = data.active_path() {
                    if path.start_point().id == hit && !path.is_closed() {
                        if let Some(path) = data.active_path_mut() {
                            let selection = path.close(event.mods.alt());
                            data.selection.select_one(selection);
                            self.this_edit_type = Some(EditType::Normal);
                            self.state = State::AddPoint(selection);
                            return;
                        }
                    } else if event.mods.alt() && path.is_hyper() {
                        data.toggle_point_type(hit);
                        self.this_edit_type = Some(EditType::Normal);
                        return;
                    }
                }

                // TODO: hit-test *other* points? more stuff when clicking on
                // If selection is empty, and point is endpoint of open path,
                // select that point.
            }

            // Handle clicking on segment (split).
            if let Some((seg, t)) = data.hit_test_segments(event.pos, None) {
                self.this_edit_type = Some(EditType::Normal);
                let path = data.path_for_point_mut(seg.start_id()).unwrap();
                path.split_segment_at_point(seg, t);
                return;
            }

            let dpoint = vport.from_screen(event.pos);
            let new_point =
                if let Some(active) = data.active_path_mut().filter(|path| !path.is_closed()) {
                    let dpoint = if event.mods.shift() {
                        let last_point = active.points().last().unwrap();
                        dpoint.axis_locked_to(last_point.point)
                    } else {
                        dpoint
                    };
                    let is_smooth = event.mods.alt();
                    active.line_to(dpoint, is_smooth)
                } else {
                    let path = if self.hyperbezier_mode {
                        Path::new_hyper(dpoint)
                    } else {
                        Path::new(dpoint)
                    };
                    let selection = path.points().first().unwrap().id;
                    data.add_path(path);
                    selection
                };

            data.selection.select_one(new_point);
            self.state = State::AddPoint(new_point);
            self.this_edit_type = Some(EditType::Normal);
        } else if event.count == 2 {
            // This is not what Glyphs does; rather, it sets the currently active
            // point to non-smooth.
            data.selection.clear();
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, data: &mut EditSession) {
        if let Some(path) = data.active_path_mut() {
            if path.is_hyper()
                || (path.is_closed() || path.points().len() > 1 && !path.last_segment_is_curve())
            {
                path.clear_trailing();
            }
        }
        self.state = State::Ready;
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        if let State::DragHandle(id) = self.state {
            let new_pos = current_drag_pos(&drag, data);
            let path = bail!(data.path_for_point_mut(id));
            path.update_trailing(id, new_pos);
            self.this_edit_type = Some(EditType::Drag);
        }
    }

    fn left_drag_began(&mut self, drag: Drag, data: &mut EditSession) {
        if let State::AddPoint(id) = self.state {
            let pos = current_drag_pos(&drag, data);
            let path = bail!(data.path_for_point_mut(id));
            let seg = path.iter_segments().find(|seg| seg.end_id() == id);
            if let Some(seg) = seg {
                if seg.is_line() {
                    if !seg.end().is_smooth() {
                        path.toggle_point_type(id);
                    }
                    path.upgrade_line_seg(&seg, true);
                }
            }
            path.update_trailing(id, pos);
            self.state = State::DragHandle(id);
            self.this_edit_type = Some(EditType::Drag);
        }
    }

    fn left_drag_ended(&mut self, _: Drag, _: &mut EditSession) {
        // TODO: this logic needs rework. A click-drag sequence should be a single
        // undo group.
        self.this_edit_type = Some(EditType::DragUp);
    }
}

fn current_drag_pos(drag: &Drag, data: &EditSession) -> DPoint {
    let start = data.viewport.from_screen(drag.start.pos);
    let current = data.viewport.from_screen(drag.current.pos);
    if drag.current.mods.shift() {
        current.axis_locked_to(start)
    } else {
        current
    }
}

impl Tool for Pen {
    fn cancel(
        &mut self,
        mouse: &mut Mouse,
        _ctx: &mut EventCtx,
        data: &mut EditSession,
    ) -> Option<EditType> {
        mouse.cancel(data, self);
        self.this_edit_type.take()
    }

    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        _ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        assert!(self.this_edit_type.is_none());
        mouse.mouse_event(event, data, self);
        self.this_edit_type.take()
    }

    fn key_down(
        &mut self,
        event: &KeyEvent,
        _ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        assert!(self.this_edit_type.is_none());
        match event {
            e if e.key == KbKey::Backspace => {
                data.delete_selection();
                self.this_edit_type = Some(EditType::Normal);
            }
            // TODO: should support nudging; basically a lot of this should
            // be shared with selection.
            _ => return None,
        }
        self.this_edit_type.take()
    }

    fn name(&self) -> ToolId {
        "Pen"
    }
}

impl Default for State {
    fn default() -> Self {
        State::Ready
    }
}
