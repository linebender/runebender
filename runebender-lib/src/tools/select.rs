use druid::kurbo::{BezPath, Circle, Insets, Point, Rect, Shape, Vec2};
use druid::piet::{RenderContext, StrokeStyle};
use druid::{Data, Env, EventCtx, HotKey, KbKey, KeyEvent, MouseEvent, PaintCtx, RawMods};

use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::point_list::Segment;
use crate::tools::{EditType, Tool, ToolId};
use crate::{
    design_space::{DPoint, DVec2, ViewPort},
    quadrant::Quadrant,
    selection::Selection,
    theme,
};

// distance from edges of the selection bbox to where we draw the handles
const SELECTION_BBOX_HANDLE_PADDING: Insets = Insets::uniform(6.0);
const SELECTION_HANDLE_RADIUS: f64 = 4.;

/// A set of states that are possible while handling a mouse drag.
#[derive(Debug, Clone)]
enum DragState {
    /// State for a drag that is a rectangular selection.
    Select {
        previous: Selection,
        rect: Rect,
    },
    /// State for a drag that is moving a selected object.
    Move {
        delta: DVec2,
    },
    /// State for a drag that is moving an off-curve point.
    MoveHandle,
    /// State if some earlier gesture consumed the mouse-down, and we should not
    /// recognize a drag.
    Suppress,
    TransformSelection {
        quadrant: Quadrant,
        previous: EditSession,
        delta: DVec2,
        /// The paths before this transform; we want to draw these faintly
        /// until the gesture completes
        pre_paths: BezPath,
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

impl Tool for Select {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, env: &Env) {
        let selection_stroke = env.get(theme::SELECTION_RECT_STROKE_COLOR);
        match &self.drag {
            DragState::Select { rect, .. } => {
                ctx.fill(rect, &env.get(theme::SELECTION_RECT_FILL_COLOR));
                ctx.stroke(rect, &selection_stroke, 1.0);
            }
            // draw the selection bounding box
            DragState::None if data.selection.len() > 1 => {
                let bbox = data.viewport.rect_to_screen(data.selection_dpoint_bbox());
                let style = StrokeStyle::new().dash(vec![2.0, 4.0], 0.0);
                ctx.stroke_styled(&bbox, &selection_stroke, 0.5, &style);

                for (_, circle) in iter_handle_circles(data) {
                    if circle.contains(self.last_pos) {
                        ctx.fill(circle, &selection_stroke);
                    }
                    ctx.stroke(circle, &selection_stroke, 0.5);
                }
            }
            DragState::TransformSelection { pre_paths, .. } => {
                ctx.stroke(
                    data.viewport.affine() * pre_paths,
                    &env.get(theme::PLACEHOLDER_GLYPH_COLOR),
                    1.0,
                );
                let bbox = data.viewport.rect_to_screen(data.selection_dpoint_bbox());
                let style = StrokeStyle::new().dash(vec![2.0, 4.0], 0.0);
                ctx.stroke_styled(&bbox, &selection_stroke, 0.5, &style);

                for (_loc, circle) in iter_handle_circles(data) {
                    //FIXME: we don't fill while dragging because we would
                    //fill the wrong handle when scale goes negative. Lots of
                    //ways to be fancy here, but for now we can just leave it.
                    //if loc == *quadrant {
                    //ctx.fill(circle, &selection_stroke);
                    //}
                    ctx.stroke(circle, &selection_stroke, 0.5);
                }
            }
            _ => (),
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
        match event {
            e if e.key == KbKey::ArrowLeft
                || e.key == KbKey::ArrowDown
                || e.key == KbKey::ArrowUp
                || e.key == KbKey::ArrowRight =>
            {
                self.nudge(data, event);
            }
            e if e.key == KbKey::Backspace => {
                data.delete_selection();
                self.this_edit_type = Some(EditType::Normal);
            }
            e if HotKey::new(None, KbKey::Tab).matches(e) => data.select_next(),
            //TODO: add Shift to SysMods
            e if HotKey::new(RawMods::Shift, KbKey::Tab).matches(e) => data.select_prev(),
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
        let (mut nudge, edit_type) = match event.key {
            KbKey::ArrowLeft => (Vec2::new(-1.0, 0.), EditType::NudgeLeft),
            KbKey::ArrowRight => (Vec2::new(1.0, 0.), EditType::NudgeRight),
            KbKey::ArrowUp => (Vec2::new(0.0, 1.0), EditType::NudgeUp),
            KbKey::ArrowDown => (Vec2::new(0.0, -1.0), EditType::NudgeDown),
            _ => unreachable!(),
        };

        if event.mods.meta() {
            nudge *= 100.;
        } else if event.mods.shift() {
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

    fn selection_handle_hit(&self, data: &EditSession, pos: Point) -> Option<Quadrant> {
        if data.selection.len() <= 1 {
            return None;
        }

        let (handle, handle_dist) = iter_handle_circles(data)
            .map(|(loc, circ)| (loc, circ.center.distance(pos)))
            .fold(
                (Quadrant::Center, f64::MAX),
                |(best_loc, closest), (this_loc, this_dist)| {
                    let best_loc = if this_dist < closest {
                        this_loc
                    } else {
                        best_loc
                    };
                    (best_loc, this_dist.min(closest))
                },
            );
        if handle_dist <= SELECTION_HANDLE_RADIUS {
            Some(handle)
        } else {
            None
        }
    }
}

impl MouseDelegate<EditSession> for Select {
    fn mouse_moved(&mut self, event: &MouseEvent, _data: &mut EditSession) {
        self.last_pos = event.pos;
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        assert!(matches!(self.drag, DragState::None));
        if event.count == 1 {
            // if we have an existing multi-point selection we first hit-test
            // our own selection handles. If we're on one of them, we don't
            // do anything further; we will start a transform in drag_began
            if self.selection_handle_hit(data, event.pos).is_some() {
                return;
            }

            let sel = data.hit_test_all(event.pos, None);
            if let Some(point_id) = sel {
                if !event.mods.shift() {
                    // when clicking a point, if it is not selected we set it as the selection,
                    // otherwise we keep the selection intact for a drag.
                    if !data.selection.contains(&point_id) {
                        data.selection.select_one(point_id);
                    }
                } else if !data.selection.remove(&point_id) {
                    data.selection.insert(point_id);
                }
            } else if let Some((seg, _t)) = data.hit_test_segments(event.pos, None) {
                let ids = seg.ids();
                let all_selected = ids.iter().all(|id| data.selection.contains(id));
                let append_mode = event.mods.shift();
                self.drag = DragState::Suppress;

                if event.mods.alt() && matches!(seg, Segment::Line(..)) {
                    let path = data.path_for_point_mut(seg.start_id()).unwrap();
                    path.upgrade_line_seg(seg);
                    self.this_edit_type = Some(EditType::Normal);
                    return;
                }
                if !append_mode && !all_selected {
                    data.selection.clear();
                    data.selection.extend(ids);
                } else if !append_mode && all_selected {
                    // we allow a drag gesture to begin only if the clicked
                    // segment was previously selected.
                    self.drag = DragState::None;
                } else if append_mode && all_selected {
                    for id in &ids {
                        data.selection.remove(id);
                    }
                } else if append_mode {
                    data.selection.extend(ids);
                }
            } else if !event.mods.shift() {
                data.selection.clear();
            }
        } else if event.count == 2 {
            let sel = data.hit_test_all(event.pos, None);
            match sel {
                Some(id) if data.path_point_for_id(id).is_some() => {
                    data.toggle_point_type(id);
                    self.this_edit_type = Some(EditType::Normal);
                }
                Some(id) if id.is_guide() => {
                    data.toggle_guide(id, event.pos);
                    self.this_edit_type = Some(EditType::Normal);
                }
                _ => {
                    data.select_path(event.pos, event.mods.shift());
                }
            }
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, _data: &mut EditSession) {
        self.drag = DragState::None;
    }

    fn left_drag_began(&mut self, drag: Drag, data: &mut EditSession) {
        if matches!(self.drag, DragState::Suppress) {
            return;
        }

        // are we dragging a selection rect handle?
        if let Some(quadrant) = self.selection_handle_hit(data, drag.start.pos) {
            let pre_paths = data.to_bezier();
            self.drag = DragState::TransformSelection {
                delta: DVec2::ZERO,
                quadrant,
                previous: data.clone(),
                pre_paths,
            };
            return;
        }

        // if we're starting a rectangular selection, we save the previous selection
        let sel = data.hit_test_all(drag.start.pos, None);
        self.drag = if let Some(pt) = sel.and_then(|id| data.path_point_for_id(id)) {
            let is_handle = !pt.is_on_curve();
            let is_dragging_handle = data.selection.len() == 1 && is_handle;
            if is_dragging_handle {
                DragState::MoveHandle
            } else {
                DragState::Move { delta: DVec2::ZERO }
            }
        } else if data.hit_test_segments(drag.start.pos, None).is_some() {
            DragState::Move { delta: DVec2::ZERO }
        } else {
            // if we're starting a rectangular selection, we save the previous selection
            DragState::Select {
                previous: data.selection.clone(),
                rect: Rect::from_points(drag.start.pos, drag.current.pos),
            }
        }
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        self.last_pos = drag.current.pos;
        match &mut self.drag {
            DragState::Select { previous, rect } => {
                *rect = Rect::from_points(drag.current.pos, drag.start.pos);
                update_selection_for_drag(data, previous, *rect, drag.current.mods.shift());
            }
            DragState::Move { delta } => {
                let mut new_delta = delta_for_drag_change(&drag, data.viewport);
                if drag.current.mods.shift() {
                    new_delta = new_delta.axis_locked();
                }
                let drag_delta = new_delta - *delta;
                if drag_delta.hypot() > 0. {
                    data.nudge_selection(drag_delta);
                    *delta = new_delta;
                }
            }
            DragState::MoveHandle => {
                data.update_handle(drag.current.pos, drag.current.mods.shift());
            }
            DragState::Suppress => (),
            DragState::TransformSelection {
                quadrant,
                previous,
                delta,
                ..
            } => {
                let new_delta = delta_for_drag_change(&drag, data.viewport);
                let new_delta = quadrant.lock_delta(new_delta);
                if new_delta.hypot() > 0.0 && new_delta != *delta {
                    *delta = new_delta;
                    let mut new_data = previous.clone();
                    let sel_rect = previous.selection_dpoint_bbox();
                    let scale = quadrant.scale_dspace_rect(sel_rect, new_delta);
                    let anchor = quadrant.inverse().point_in_dspace_rect(sel_rect);
                    new_data.scale_selection(scale, DPoint::from_raw(anchor));
                    *data = new_data;
                }
            }
            DragState::None => unreachable!("invalid state"),
        }

        if self.drag.is_move() || self.drag.is_transform() {
            self.this_edit_type = Some(EditType::Drag);
        }
    }

    fn left_drag_ended(&mut self, _drag: Drag, _data: &mut EditSession) {
        if self.drag.is_move() || self.drag.is_transform() {
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

/// When dragging, we only update positions when they change in design-space,
/// so we keep track of the current total design-space delta.
fn delta_for_drag_change(drag: &Drag, viewport: ViewPort) -> DVec2 {
    let drag_start = viewport.from_screen(drag.start.pos);
    let drag_pos = viewport.from_screen(drag.current.pos);
    drag_pos - drag_start
}

fn iter_handle_circles(session: &EditSession) -> impl Iterator<Item = (Quadrant, Circle)> {
    let bbox = session
        .viewport
        .rect_to_screen(session.selection_dpoint_bbox());
    let handle_frame = bbox + SELECTION_BBOX_HANDLE_PADDING;
    Quadrant::all()
        .iter()
        .filter(move |q| {
            !(bbox.width() == 0. && q.modifies_x_axis())
                && !(bbox.height() == 0. && q.modifies_y_axis())
                && !matches!(q, Quadrant::Center)
        })
        .map(move |loc| {
            let center = loc.point_in_rect(handle_frame);
            let circle = Circle::new(center, SELECTION_HANDLE_RADIUS);
            (*loc, circle)
        })
}

fn update_selection_for_drag(
    data: &mut EditSession,
    prev_sel: &Selection,
    rect: Rect,
    shift: bool,
) {
    let in_select_rect = data
        .iter_points()
        .filter(|p| rect.contains(p.to_screen(data.viewport)))
        .map(|p| p.id)
        .collect();
    data.selection = if shift {
        prev_sel.symmetric_difference(&in_select_rect)
    } else {
        prev_sel.union(&in_select_rect)
    };
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
        matches!(self, DragState::Move { .. })
    }

    fn is_transform(&self) -> bool {
        matches!(self, DragState::TransformSelection{ .. })
    }
}
