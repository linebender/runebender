//! The bezier pen tool.

use druid::kurbo::Point;
use druid::{Env, EventCtx, KbKey, KeyEvent, MouseEvent};

use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::tools::{EditType, Tool, ToolId};

/// The state of the pen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Pen {
    this_edit_type: Option<EditType>,
    is_draggable: bool,
}

impl MouseDelegate<EditSession> for Pen {
    fn cancel(&mut self, canvas: &mut EditSession) {
        canvas.selection_mut().clear();
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        self.is_draggable = false;
        let vport = data.viewport;
        if event.count == 1 {
            let hit = data.hit_test_filtered(event.pos, None, |p| p.is_on_curve());
            if let Some(hit) = hit {
                if let Some(path) = data.active_path() {
                    if path.start_point().id == hit && !path.is_closed() {
                        if let Some(path) = data.active_path_mut() {
                            let start = path.start_point().id;
                            self.this_edit_type = Some(EditType::Normal);
                            path.close();
                            data.set_selection_one(start);
                            self.is_draggable = true;
                            return;
                        }
                    }
                }
                // TODO: more stuff when clicking on points
                // If selection is empty, and point is endpoint of open path,
                // select that point.
                // Otherwise maybe cut path at that point?
                return;
            }

            // Handle clicking on segment (split).
            if let Some((seg, t)) = data.hit_test_segments(event.pos, None) {
                self.this_edit_type = Some(EditType::Normal);
                let path = data.path_for_point_mut(seg.start_id()).unwrap();
                path.split_segment_at_point(seg, t);
                return;
            }

            let point = match data.active_path() {
                // lock to nearest vertical or horizontal axis if shift is pressed
                Some(path) if event.mods.shift() => {
                    let last_point = path.points().last().unwrap().to_screen(vport);
                    axis_locked_point(event.pos, last_point)
                }
                _ => event.pos,
            };

            self.this_edit_type = Some(EditType::Normal);
            data.add_point(point);
            self.is_draggable = true;
        } else if event.count == 2 {
            // This is not what Glyphs does; rather, it sets the currently active
            // point to non-smooth.
            data.selection_mut().clear();
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, data: &mut EditSession) {
        if let Some(path) = data.active_path_mut() {
            if path.is_closed() || path.points().len() > 1 && !path.last_segment_is_curve() {
                path.clear_trailing();
            }
        }
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        if !self.is_draggable {
            return;
        }
        let Drag { start, current, .. } = drag;
        let handle_point = if current.mods.shift() {
            axis_locked_point(current.pos, start.pos)
        } else {
            current.pos
        };
        data.update_for_drag(handle_point);
        self.this_edit_type = Some(EditType::Drag);
    }

    fn left_drag_ended(&mut self, _: Drag, _: &mut EditSession) {
        // TODO: this logic needs rework. A click-drag sequence should be a single
        // undo group.
        self.this_edit_type = Some(EditType::DragUp);
    }
}

impl Tool for Pen {
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

/// Lock the smallest axis of `point` (from `prev`) to that axis on `prev`.
/// (aka shift + click)
fn axis_locked_point(point: Point, prev: Point) -> Point {
    let dxy = prev - point;
    if dxy.x.abs() > dxy.y.abs() {
        Point::new(point.x, prev.y)
    } else {
        Point::new(prev.x, point.y)
    }
}
