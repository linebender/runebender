//! The bezier (and hyperbezier!) pen tool.

use druid::{Env, EventCtx, KbKey, KeyEvent, MouseEvent};

use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::Path;
use crate::tools::{EditType, Tool, ToolId};

/// The state of the pen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Pen {
    hyperbezier_mode: bool,
    this_edit_type: Option<EditType>,
    is_draggable: bool,
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

impl MouseDelegate<EditSession> for Pen {
    fn cancel(&mut self, canvas: &mut EditSession) {
        canvas.selection.clear();
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        self.is_draggable = false;
        let vport = data.viewport;
        if event.count == 1 {
            let hit = data.hit_test_filtered(event.pos, None, |_| true);
            if let Some(hit) = hit {
                if let Some(path) = data.active_path() {
                    if path.start_point().id == hit && !path.is_closed() {
                        if let Some(path) = data.active_path_mut() {
                            let selection = path.close(event.mods.alt());
                            data.selection.select_one(selection);
                            self.this_edit_type = Some(EditType::Normal);
                            self.is_draggable = true;
                            return;
                        }
                    } else if event.mods.alt() && path.is_hyper() {
                        data.toggle_point_type(hit);
                        self.this_edit_type = Some(EditType::Normal);
                        self.is_draggable = true;
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
            if let Some(active) = data.active_path_mut().filter(|path| !path.is_closed()) {
                let dpoint = if event.mods.shift() {
                    let last_point = active.points().last().unwrap();
                    dpoint.axis_locked_to(last_point.point)
                } else {
                    dpoint
                };
                let is_smooth = event.mods.alt();
                let selection = active.line_to(dpoint, is_smooth);
                data.selection.select_one(selection);
            } else {
                let path = if self.hyperbezier_mode {
                    Path::new_hyper(dpoint)
                } else {
                    Path::new(dpoint)
                };
                let selection = path.points().first().unwrap().id;
                data.selection.select_one(selection);
                data.add_path(path);
            }

            self.this_edit_type = Some(EditType::Normal);
            self.is_draggable = true;
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
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        if !self.is_draggable {
            return;
        }
        let Drag { start, current, .. } = drag;
        let handle_point = if current.mods.shift() {
            super::axis_locked_point(current.pos, start.pos)
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
