use std::collections::BTreeSet;
use std::sync::Arc;

use druid::kurbo::{Point, Rect, Vec2};
use druid::piet::{Color, RenderContext};
use druid::{Data, Env, EventCtx, HotKey, KeyCode, KeyEvent, MouseEvent, PaintCtx, RawMods};

use crate::design_space::DVec2;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::EntityId;
use crate::tools::{EditType, Tool};

const SELECTION_RECT_BG_COLOR: Color = Color::rgba8(0xDD, 0xDD, 0xDD, 0x55);
const SELECTION_RECT_STROKE_COLOR: Color = Color::rgb8(0x53, 0x8B, 0xBB);

/// The state of the selection tool.
#[derive(Debug, Default, Clone)]
pub struct Select {
    /// when a drag is in progress, this is the state of the selection at the start
    /// of the drag.
    prev_selection: Option<Arc<BTreeSet<EntityId>>>,
    drag_rect: Option<Rect>,
    last_drag_pos: Option<Point>,
    last_pos: Point,
    /// The edit type produced by the current event, if any.
    ///
    /// This is stashed here because we can't return anything from the methods in
    /// `MouseDelegate`.
    ///
    /// It is an invariant that this is always `None`, except while we are in
    /// a `key_down`, `key_up`, or `mouse_event` method.
    this_edit_type: Option<EditType>,
}

impl Tool for Select {
    fn paint(&mut self, ctx: &mut PaintCtx, _data: &EditSession, _env: &Env) {
        if let Some(rect) = self.drag_rect {
            ctx.fill(rect, &SELECTION_RECT_BG_COLOR);
            ctx.stroke(rect, &SELECTION_RECT_STROKE_COLOR, 1.0);
        }
    }

    fn key_down(
        &mut self,
        event: &KeyEvent,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        assert!(self.this_edit_type.is_none());
        use KeyCode::*;
        match event {
            e if e.key_code == ArrowLeft
                || e.key_code == ArrowDown
                || e.key_code == ArrowUp
                || e.key_code == ArrowRight =>
            {
                self.nudge(data, event);
            }
            e if e.key_code == Backspace => {
                data.delete_selection();
                self.this_edit_type = Some(EditType::Normal);
            }
            e if HotKey::new(None, KeyCode::Tab).matches(e) => data.select_next(),
            //TODO: add Shift to SysMods
            e if HotKey::new(RawMods::Shift, KeyCode::Tab).matches(e) => data.select_next(),
            _ => return None,
        }
        ctx.invalidate();
        self.this_edit_type.take()
    }

    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        assert!(self.this_edit_type.is_none());
        let pre_rect = self.drag_rect;
        let pre_data = data.clone();
        mouse.mouse_event(event, data, self);
        if !rect_equality(pre_rect, self.drag_rect) || !pre_data.same(data) {
            ctx.invalidate();
        }
        self.this_edit_type.take()
    }
}

impl Select {
    fn update_selection_for_drag(
        &self,
        data: &mut EditSession,
        prev_sel: &BTreeSet<EntityId>,
        rect: Rect,
        shift: bool,
    ) {
        let in_select_rect = data
            .iter_points()
            .filter(|p| rect.contains(p.to_screen(data.viewport)))
            .map(|p| p.id)
            .collect();
        let new_sel = if shift {
            prev_sel
                .symmetric_difference(&in_select_rect)
                .copied()
                .collect()
        } else {
            prev_sel.union(&in_select_rect).copied().collect()
        };
        *data.selection_mut() = new_sel;
    }

    fn nudge(&mut self, data: &mut EditSession, event: &KeyEvent) {
        let (mut nudge, edit_type) = match event.key_code {
            KeyCode::ArrowLeft => (Vec2::new(-1.0, 0.), EditType::NudgeLeft),
            KeyCode::ArrowRight => (Vec2::new(1.0, 0.), EditType::NudgeRight),
            KeyCode::ArrowUp => (Vec2::new(0.0, 1.0), EditType::NudgeUp),
            KeyCode::ArrowDown => (Vec2::new(0.0, -1.0), EditType::NudgeDown),
            _ => unreachable!(),
        };

        if event.mods.meta {
            nudge *= 100.;
        } else if event.mods.shift {
            nudge *= 10.;
        }

        data.nudge_selection(DVec2::from_raw(nudge));

        // for the purposes of undo, we only combine single-unit nudges
        if nudge.hypot().abs() > 1.0 {
            self.this_edit_type = Some(EditType::Normal);
        } else {
            self.this_edit_type = Some(edit_type);
        }
    }
}

impl MouseDelegate<EditSession> for Select {
    fn mouse_moved(&mut self, event: &MouseEvent, _data: &mut EditSession) {
        self.last_pos = event.pos;
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        if event.count == 1 {
            let sel = data.iter_items_near_point(event.pos, None).next();
            if let Some(point_id) = sel {
                if !event.mods.shift {
                    // when clicking a point, if it is not selected we set it as the selection,
                    // otherwise we keep the selection intact for a drag.
                    if !data.selection.contains(&point_id) {
                        data.selection_mut().clear();
                        data.selection_mut().insert(point_id);
                    }
                } else if !data.selection_mut().remove(&point_id) {
                    data.selection_mut().insert(point_id);
                }
            } else if !event.mods.shift {
                data.selection_mut().clear();
            }
        } else if event.count == 2 {
            let sel = data.iter_items_near_point(event.pos, None).next().clone();
            match sel {
                Some(id)
                    if data
                        .path_point_for_id(id)
                        .map(|p| p.is_on_curve())
                        .unwrap_or(false) =>
                {
                    data.toggle_selected_on_curve_type();
                    self.this_edit_type = Some(EditType::Normal);
                }
                Some(id) if id.is_guide() => data.toggle_guide(id, event.pos),
                _ => {
                    data.select_path(event.pos, event.mods.shift);
                }
            }
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, _data: &mut EditSession) {
        self.prev_selection = None;
        self.drag_rect = None;
    }

    fn left_drag_began(&mut self, drag: Drag, data: &mut EditSession) {
        // if we're starting a rectangular selection, we save the previous selection
        self.prev_selection = if data
            .iter_items_near_point(drag.start.pos, None)
            .next()
            .is_none()
        {
            Some(data.selection.clone())
        // otherwise we are dragging some object
        } else {
            None
        };
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        if let Some(prev_selection) = self.prev_selection.as_ref() {
            let rect = Rect::from_points(drag.current.pos, drag.start.pos);
            self.drag_rect = Some(rect);
            self.update_selection_for_drag(data, prev_selection, rect, drag.current.mods.shift);
        } else {
            let last_drag_pos = self.last_drag_pos.unwrap_or(drag.start.pos);
            let dvec = drag.current.pos - last_drag_pos;
            let drag_vec = dvec * data.viewport.zoom.recip();
            let drag_vec = DVec2::from_raw((drag_vec.x.floor(), drag_vec.y.floor()));
            if drag_vec.hypot() > 0. {
                // multiple small drag updates that don't make up a single point in design
                // space should be aggregated
                let aligned_drag_delta = drag_vec.to_screen(data.viewport);
                let aligned_last_drag = last_drag_pos + aligned_drag_delta;
                self.last_drag_pos = Some(aligned_last_drag);
                //HACK: because this is used to compute last_drag_pos,
                //we only swap the y-axis at the end  ¯\_(ツ)_/¯
                let mut drag_vec = drag_vec;
                drag_vec.y = -drag_vec.y;
                data.nudge_selection(drag_vec);
                self.this_edit_type = Some(EditType::Drag);
            }
        }
    }

    fn left_drag_ended(&mut self, _drag: Drag, _data: &mut EditSession) {
        self.last_drag_pos = None;
        if self.prev_selection.take().is_none() {
            self.this_edit_type = Some(EditType::DragUp);
        }
    }

    fn cancel(&mut self, data: &mut EditSession) {
        if let Some(prev) = self.prev_selection.take() {
            data.selection = prev;
        }
        self.drag_rect = None;
    }
}

fn rect_equality(one: Option<Rect>, two: Option<Rect>) -> bool {
    match (one, two) {
        (None, None) => true,
        (Some(one), Some(two)) => {
            one.x0 == two.x0 && one.x1 == two.x1 && one.y0 == two.y0 && one.y1 == two.y1
        }
        _ => false,
    }
}
