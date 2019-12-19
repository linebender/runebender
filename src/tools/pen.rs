//! The bezier pen tool.

use druid::kurbo::Point;
use druid::{Env, EventCtx, MouseEvent};

use crate::edit_session::{EditSession, MIN_CLICK_DISTANCE};
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::tools::{EditType, Tool};

/// The state of the pen.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Pen {
    this_edit_type: Option<EditType>,
}

impl MouseDelegate<EditSession> for Pen {
    fn cancel(&mut self, canvas: &mut EditSession) {
        canvas.selection_mut().clear();
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        let vport = data.viewport;
        if event.count == 1 {
            let point = match data.active_path() {
                Some(path)
                    if path.start_point().screen_dist(vport, event.pos) < MIN_CLICK_DISTANCE =>
                {
                    path.start_point().to_screen(vport)
                }
                // lock to nearest vertical or horizontal axis if shift is pressed
                Some(path) if event.mods.shift => {
                    let last_point = path.points().last().unwrap().to_screen(vport);
                    axis_locked_point(event.pos, last_point)
                }
                _ => event.pos,
            };

            self.this_edit_type = Some(EditType::Normal);
            data.add_point(point);
        } else if event.count == 2 {
            data.selection_mut().clear();
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, data: &mut EditSession) {
        if let Some(path) = data.active_path_mut() {
            if path.is_closed() || path.points().len() > 1 && !path.last_segment_is_curve() {
                path.clear_trailing();
            }
        }

        if data
            .active_path_mut()
            .map(|p| p.is_closed())
            .unwrap_or(false)
        {
            data.selection_mut().clear();
        }
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        let Drag { start, current, .. } = drag;
        let handle_point = if current.mods.shift {
            axis_locked_point(current.pos, start.pos)
        } else {
            current.pos
        };
        data.update_for_drag(handle_point);
        self.this_edit_type = Some(EditType::Drag);
    }

    fn left_drag_began(&mut self, _: Drag, _: &mut EditSession) {
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

    fn name(&self) -> &str {
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
