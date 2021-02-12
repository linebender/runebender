use druid::kurbo::{BezPath, Circle, Insets, Point, Rect, Vec2};
use druid::piet::{RenderContext, StrokeStyle};
use druid::{Data, Env, EventCtx, HotKey, KbKey, KeyEvent, MouseEvent, PaintCtx, RawMods};

use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::Segment;
use crate::point::EntityId;
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

/// An item that can be selected.
#[derive(Debug, Clone)]
enum Item {
    SelectionHandle(Quadrant),
    Point(EntityId),
    Guide(EntityId),
    Segment(Box<Segment>),
}

/// The internal state of the mouse.
#[derive(Debug, Clone)]
enum MouseState {
    /// The mouse is idle; it may be hovering on an item.
    Idle(Option<Item>),
    /// The mouse has clicked, and may have clicked an item.
    Down(Option<Item>),
    /// The mouse is down but we should not transition to a drag if one
    /// begins.
    SuppressDrag,
    /// A drag gesture is in progress.
    Drag(DragState),
    /// The mouse is up after clicking an item; if a double-click occurs
    /// it will modify that item
    WaitDoubleClick(Item),
    /// Internal: the state is in transition. This should only be present
    /// during event handling.
    Transition,
}

/// The possible states for the select tool.
#[derive(Debug, Clone)]
enum DragState {
    /// State for a drag that is a rectangular selection.
    Select {
        previous: Selection,
        rect: Rect,
        toggle: bool,
    },
    /// State for a drag that is moving a selected object.
    Move { previous: EditSession, delta: DVec2 },
    TransformSelection {
        quadrant: Quadrant,
        previous: EditSession,
        delta: DVec2,
        /// The paths before this transform; we want to draw these faintly
        /// until the gesture completes
        pre_paths: BezPath,
    },
}

/// The state of the selection tool.
#[derive(Debug, Default, Clone)]
pub struct Select {
    ///  state preserved between events.
    state: MouseState,
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
    fn cancel(
        &mut self,
        mouse: &mut Mouse,
        _ctx: &mut EventCtx,
        data: &mut EditSession,
    ) -> Option<EditType> {
        mouse.cancel(data, self);
        self.this_edit_type.take()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, env: &Env) {
        let selection_stroke = env.get(theme::SELECTION_RECT_STROKE_COLOR);
        match &self.state {
            MouseState::Idle(item) => {
                let quad = match &item {
                    Some(Item::SelectionHandle(quad)) => Some(*quad),
                    _ => None,
                };
                paint_selection_bbox(ctx, data, env, quad);
                match item {
                    Some(Item::Point(id)) => {
                        if let Some(pp) = data.path_point_for_id(*id) {
                            let point = data.viewport.to_screen(pp.point);
                            paint_hover_indicator(ctx, data, point, env);
                        }
                    }
                    Some(Item::Segment(seg)) => {
                        let seg_point = data.viewport.affine()
                            * seg.nearest_point(data.viewport.from_screen(self.last_pos));
                        paint_hover_indicator(ctx, data, seg_point, env);
                    }
                    Some(Item::Guide(id)) => {
                        if let Some(point) =
                            data.guides.iter().find(|g| g.id == *id).map(|guide| {
                                guide.nearest_screen_point(data.viewport, self.last_pos)
                            })
                        {
                            paint_hover_indicator(ctx, data, point, env);
                        }
                    }
                    Some(Item::SelectionHandle(_)) | None => (),
                }
            }
            MouseState::Drag(drag_state) => match drag_state {
                DragState::Select { rect, .. } => {
                    ctx.fill(rect, &env.get(theme::SELECTION_RECT_FILL_COLOR));
                    ctx.stroke(rect, &selection_stroke, 1.0);
                }
                // draw the selection bounding box
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
            },
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
        let pre_rect = self.state.drag_rect();
        mouse.mouse_event(event, data, self);
        if !pre_rect.same(&self.state.drag_rect()) {
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

    fn hover_item_for_mos_pos(&self, data: &EditSession, pos: Point) -> Option<Item> {
        if let Some(quadrant) = self.selection_handle_hit(data, pos) {
            return Some(Item::SelectionHandle(quadrant));
        }

        if let Some(id) = data.hit_test_all(pos, None) {
            if id.is_guide() {
                Some(Item::Guide(id))
            } else {
                Some(Item::Point(id))
            }
        } else if let Some((seg, _t)) =
            data.hit_test_segments(pos, Some(crate::edit_session::SEGMENT_CLICK_DISTANCE))
        {
            Some(Item::Segment(seg.into()))
        } else {
            None
        }
    }
}

impl MouseDelegate<EditSession> for Select {
    fn mouse_moved(&mut self, event: &MouseEvent, data: &mut EditSession) {
        let hover_item = self.hover_item_for_mos_pos(data, event.pos);
        self.state = MouseState::Idle(hover_item);
        self.last_pos = event.pos;
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        if event.pos != self.last_pos {
            log::info!(
                "left_down pos != mouse_move pos: {:.2}/{:.2}",
                event.pos,
                self.last_pos
            );
        }

        let append_mode = event.mods.shift();
        if event.count == 1 {
            let item = match self.state.transition() {
                MouseState::Idle(item) => item,
                MouseState::WaitDoubleClick(item) => Some(item),
                _ => None,
            };
            self.state = match item {
                Some(Item::SelectionHandle(_)) => MouseState::Down(item),
                Some(Item::Point(id)) | Some(Item::Guide(id)) => {
                    if !append_mode {
                        if !data.selection.contains(&id) {
                            data.selection.select_one(id);
                        }
                    } else if !data.selection.remove(&id) {
                        data.selection.insert(id);
                    }
                    MouseState::Down(item)
                }
                // toggle segment type
                Some(Item::Segment(seg)) if event.mods.alt() => {
                    if seg.is_line() {
                        if let Some(path) = data.path_for_point_mut(seg.start_id()) {
                            path.upgrade_line_seg(&seg, false);
                            self.this_edit_type = Some(EditType::Normal);
                        }
                    }
                    MouseState::SuppressDrag
                }
                Some(Item::Segment(seg)) => {
                    let all_selected = seg
                        .raw_segment()
                        .iter_ids()
                        .all(|id| data.selection.contains(&id));
                    if !append_mode {
                        data.selection.clear();
                        data.selection.extend(seg.raw_segment().iter_ids());
                        MouseState::Down(Some(Item::Segment(seg)))
                    } else if all_selected {
                        for id in seg.raw_segment().iter_ids() {
                            data.selection.remove(&id);
                        }
                        MouseState::SuppressDrag
                    } else {
                        data.selection.extend(seg.raw_segment().iter_ids());
                        MouseState::Down(Some(Item::Segment(seg)))
                    }
                }
                None => MouseState::Down(None),
            };
        } else if event.count == 2 {
            self.state = match self.state.transition() {
                MouseState::WaitDoubleClick(item) => {
                    match &item {
                        Item::Point(id) => {
                            data.toggle_point_type(*id);
                            self.this_edit_type = Some(EditType::Normal);
                        }
                        Item::Guide(id) => {
                            data.toggle_guide(*id, event.pos);
                            self.this_edit_type = Some(EditType::Normal);
                        }
                        Item::Segment(seg) => {
                            data.select_path(seg.start_id().parent(), append_mode);
                        }
                        Item::SelectionHandle(_) => (),
                    };
                    MouseState::WaitDoubleClick(item)
                }
                other => {
                    log::debug!("double-click mouse state: {:?}", other);
                    MouseState::SuppressDrag
                }
            }
        }
    }

    fn left_up(&mut self, event: &MouseEvent, data: &mut EditSession) {
        self.state = match self.state.transition() {
            MouseState::Down(Some(Item::SelectionHandle(handle))) => {
                MouseState::Idle(Some(Item::SelectionHandle(handle)))
            }
            MouseState::Down(Some(item)) => MouseState::WaitDoubleClick(item),
            MouseState::Down(None) => {
                data.selection.clear();
                MouseState::Idle(self.hover_item_for_mos_pos(data, event.pos))
            }
            _ => MouseState::Idle(self.hover_item_for_mos_pos(data, event.pos)),
        }
    }

    fn left_drag_began(&mut self, drag: Drag, data: &mut EditSession) {
        self.state = match self.state.transition() {
            // starting a rectangular selection
            MouseState::Down(None) => MouseState::Drag(DragState::Select {
                previous: data.selection.clone(),
                rect: Rect::from_points(drag.start.pos, drag.current.pos),
                toggle: drag.current.mods.shift(),
            }),
            MouseState::Down(Some(Item::SelectionHandle(handle))) => {
                MouseState::Drag(DragState::TransformSelection {
                    quadrant: handle,
                    previous: data.clone(),
                    delta: DVec2::ZERO,
                    pre_paths: data.to_bezier(),
                })
            }
            MouseState::Down(Some(_)) => MouseState::Drag(DragState::Move {
                previous: data.clone(),
                delta: DVec2::ZERO,
            }),
            MouseState::SuppressDrag => MouseState::SuppressDrag,
            other => {
                log::debug!("unexpected drag_began state: {:?}", other);
                MouseState::SuppressDrag
            }
        };
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        self.last_pos = drag.current.pos;
        if let Some(state) = self.state.drag_state_mut() {
            match state {
                DragState::Select {
                    previous,
                    rect,
                    toggle,
                } => {
                    *rect = Rect::from_points(drag.current.pos, drag.start.pos);
                    update_selection_for_drag(data, previous, *rect, *toggle);
                }
                DragState::Move { delta, .. } => {
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
            }
            if matches!(
                state,
                DragState::Move { .. } | DragState::TransformSelection { .. }
            ) {
                self.this_edit_type = Some(EditType::Drag);
            }
        } else {
            log::debug!("unexpected state in drag_changed: {:?}", self.state);
        }
    }

    fn left_drag_ended(&mut self, _drag: Drag, _data: &mut EditSession) {
        if let MouseState::Drag(state) = &self.state {
            if matches!(
                state,
                DragState::Move { .. } | DragState::TransformSelection { .. }
            ) {
                self.this_edit_type = Some(EditType::DragUp);
            }
        }
    }

    //FIXME: this is never actually called? :thinking:
    fn cancel(&mut self, data: &mut EditSession) {
        let old_state = std::mem::replace(&mut self.state, MouseState::Idle(None));
        if let MouseState::Drag(state) = old_state {
            match state {
                DragState::Select { previous, .. } => data.selection = previous,
                DragState::Move { previous, .. }
                | DragState::TransformSelection { previous, .. } => {
                    *data = previous;
                    // we use 'Drag' and not 'DragUp' because we want this all to combine
                    // with the previous undo group, and be a no-op?
                    self.this_edit_type = Some(EditType::Drag);
                }
            }
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
    //corresponds to shift being held
    toggle: bool,
) {
    let in_select_rect = data
        .iter_points()
        .filter(|p| rect.contains(p.to_screen(data.viewport)))
        .map(|p| p.id)
        .collect();
    data.selection = if toggle {
        prev_sel.symmetric_difference(&in_select_rect)
    } else {
        in_select_rect
    };
}

impl MouseState {
    /// Move to the Transition state, returning the previous state.
    fn transition(&mut self) -> Self {
        std::mem::replace(self, MouseState::Transition)
    }

    /// If we're in a drag gesture, return a mutable reference to the drag state.
    fn drag_state_mut(&mut self) -> Option<&mut DragState> {
        if let MouseState::Drag(s) = self {
            Some(s)
        } else {
            None
        }
    }

    fn drag_rect(&self) -> Option<Rect> {
        if let MouseState::Drag(s) = self {
            s.drag_rect()
        } else {
            None
        }
    }
}

impl Default for MouseState {
    fn default() -> Self {
        MouseState::Idle(None)
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
}

fn paint_selection_bbox(
    ctx: &mut PaintCtx,
    data: &EditSession,
    env: &Env,
    hot_quad: Option<Quadrant>,
) {
    if data.selection.len() > 1 {
        let selection_stroke = env.get(theme::SELECTION_RECT_STROKE_COLOR);
        let bbox = data.viewport.rect_to_screen(data.selection_dpoint_bbox());
        let style = StrokeStyle::new().dash(vec![2.0, 4.0], 0.0);
        ctx.stroke_styled(&bbox, &selection_stroke, 0.5, &style);

        for (quad, circle) in iter_handle_circles(data) {
            if Some(quad) == hot_quad {
                ctx.fill(circle, &selection_stroke);
            }
            ctx.stroke(circle, &selection_stroke, 0.5);
        }
    }
}

const HOVER_ACCENT_COLOR: druid::Color = druid::Color::rgba8(0, 0, 0, 0x58);

/// the point is in design space, but needn't be on the  grid.
fn paint_hover_indicator(ctx: &mut PaintCtx, _: &EditSession, point: Point, _env: &Env) {
    let circ = Circle::new(point, 3.0);
    ctx.fill(circ, &HOVER_ACCENT_COLOR);
}
