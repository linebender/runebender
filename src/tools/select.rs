use std::collections::BTreeSet;
use std::sync::Arc;

use druid::kurbo::{Point, Rect, Vec2};
use druid::piet::{
    Color, FontBuilder, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder,
};
use druid::{Data, Env, EventCtx, HotKey, KeyCode, KeyEvent, MouseEvent, PaintCtx, RawMods};

use crate::design_space::{DPoint, DVec2};
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::{EntityId, PathPoint};
use crate::tools::{EditType, Tool, ToolId};

const SELECTION_RECT_BG_COLOR: Color = Color::rgba8(0xDD, 0xDD, 0xDD, 0x55);
const SELECTION_RECT_STROKE_COLOR: Color = Color::rgb8(0x53, 0x8B, 0xBB);
const HOVER_LABEL_PADDING: f64 = 8.0;

#[derive(Debug, Clone)]
enum DragState {
    /// State for a drag that is a rectangular selection.
    Select {
        previous: Arc<BTreeSet<EntityId>>,
        rect: Rect,
    },
    /// State for a drag that is moving a selected object.
    Move {
        last_used_pos: DPoint,
    },
    None,
}

/// The state of the selection tool.
#[derive(Debug, Default, Clone)]
pub struct Select {
    /// the state preserved between drag events.
    drag: DragState,
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

fn make_text_for_point(ctx: &mut PaintCtx, pt: PathPoint) -> PietTextLayout {
    let mut text = ctx.text();
    let font = text.new_font_by_name("Helvetica", 10.0).build().unwrap();
    let label_text = format!("{}, {}", pt.point.x, pt.point.y);
    text.new_text_layout(&font, &label_text, None)
        .build()
        .unwrap()
}

impl Tool for Select {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, env: &Env) {
        if let DragState::Select { rect, .. } = self.drag {
            ctx.fill(rect, &SELECTION_RECT_BG_COLOR);
            ctx.stroke(rect, &SELECTION_RECT_STROKE_COLOR, 1.0);
        }

        if data.settings().show_coordinate_on_hover {
            let closest = data
                .iter_items_near_point(self.last_pos, Some(8.0))
                .filter(|(id, _)| !id.is_guide())
                .fold((f64::MAX, None), |(nearest, its_id), (id, dist)| {
                    if dist < nearest {
                        (dist, Some(id))
                    } else {
                        (nearest, its_id)
                    }
                })
                .1
                .and_then(|id| data.path_point_for_id(id));
            if let Some(closest) = closest {
                let text = make_text_for_point(ctx, closest);
                //FIXME: be more methodical about picking a drawing location
                let mut draw_pt = closest.to_screen(data.viewport);
                let height = text.line_metric(0).map(|m| m.height).unwrap_or_default();
                let ascent = text.line_metric(0).map(|m| m.baseline).unwrap_or_default();
                draw_pt.x -= text.width() + HOVER_LABEL_PADDING;
                if draw_pt.x + text.width() > ctx.size().width {
                    draw_pt.x -= text.width();
                }
                if draw_pt.y + height > ctx.size().height {
                    draw_pt.y -= 10.0;
                }
                let rect =
                    Rect::from_origin_size(draw_pt - Vec2::new(0., ascent), (text.width(), height))
                        .inset(2.0)
                        .to_rounded_rect(2.0);
                ctx.fill(rect, &Color::WHITE.with_alpha(0.5));
                ctx.draw_text(&text, draw_pt, &env.get(druid::theme::LABEL_COLOR));
            }
        }
    }

    fn key_down(
        &mut self,
        event: &KeyEvent,
        _ctx: &mut EventCtx,
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
            e if HotKey::new(RawMods::Shift, KeyCode::Tab).matches(e) => data.select_prev(),
            _ => return None,
        }
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
        if data.settings().show_coordinate_on_hover {
            ctx.request_paint();
        }
        assert!(self.this_edit_type.is_none());
        let pre_rect = self.drag.drag_rect();
        mouse.mouse_event(event, data, self);
        if !pre_rect.same(&self.drag.drag_rect()) {
            ctx.request_paint();
        }
        self.this_edit_type.take()
    }

    fn name(&self) -> ToolId {
        "Select"
    }
}

impl Select {
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
            if let Some((point_id, _)) = sel {
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
            let sel = data.iter_items_near_point(event.pos, None).next();
            match sel {
                Some((id, _))
                    if data
                        .path_point_for_id(id)
                        .map(|p| p.is_on_curve())
                        .unwrap_or(false) =>
                {
                    data.toggle_selected_on_curve_type();
                    self.this_edit_type = Some(EditType::Normal);
                }
                Some((id, _)) if id.is_guide() => {
                    data.toggle_guide(id, event.pos);
                    self.this_edit_type = Some(EditType::Normal);
                }
                _ => {
                    data.select_path(event.pos, event.mods.shift);
                }
            }
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, _data: &mut EditSession) {
        self.drag = DragState::None;
    }

    fn left_drag_began(&mut self, drag: Drag, data: &mut EditSession) {
        // if we're starting a rectangular selection, we save the previous selection
        let is_dragging_item = data
            .iter_items_near_point(drag.start.pos, None)
            .any(|_| true);
        self.drag = if is_dragging_item {
            DragState::Move {
                last_used_pos: data.viewport.from_screen(drag.start.pos),
            }
        } else {
            DragState::Select {
                previous: data.selection.clone(),
                rect: Rect::from_points(drag.start.pos, drag.current.pos),
            }
        }
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        match &mut self.drag {
            DragState::Select {
                previous,
                ref mut rect,
            } => {
                *rect = Rect::from_points(drag.current.pos, drag.start.pos);
                update_selection_for_drag(data, previous, *rect, drag.current.mods.shift);
            }
            DragState::Move {
                ref mut last_used_pos,
            } => {
                let drag_pos = data.viewport.from_screen(drag.current.pos);
                let drag_delta = drag_pos - *last_used_pos;
                if drag_delta.hypot() > 0. {
                    data.nudge_selection(drag_delta);
                    *last_used_pos = drag_pos;
                }
            }
            DragState::None => unreachable!("invalid state"),
        }

        if self.drag.is_move() {
            self.this_edit_type = Some(EditType::Drag);
        }
    }

    fn left_drag_ended(&mut self, _drag: Drag, _data: &mut EditSession) {
        if self.drag.is_move() {
            self.this_edit_type = Some(EditType::DragUp);
        }
    }

    fn cancel(&mut self, data: &mut EditSession) {
        let old_state = std::mem::replace(&mut self.drag, DragState::None);
        if let DragState::Select { previous, .. } = old_state {
            data.selection = previous;
        }
    }
}

fn update_selection_for_drag(
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

impl Default for DragState {
    fn default() -> Self {
        DragState::None
    }
}

impl DragState {
    fn drag_rect(&self) -> Option<Rect> {
        if let DragState::Select { rect, .. } = self {
            Some(*rect)
        } else {
            None
        }
    }

    fn is_move(&self) -> bool {
        if let DragState::Move { .. } = self {
            true
        } else {
            false
        }
    }
}
