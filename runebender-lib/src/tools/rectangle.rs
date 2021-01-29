//! The rectangle shape tool

use druid::{
    Color, Env, EventCtx, KbKey, KeyEvent, MouseEvent, PaintCtx, Point, Rect, RenderContext,
    TextLayout,
};

use crate::cubic_path::CubicPath;
use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::point::{EntityId, PathPoint};
use crate::tools::{EditType, Tool};

/// The state of the rectangle tool.
#[derive(Debug, Clone)]
pub struct Rectangle {
    gesture: GestureState,
    shift_locked: bool,
    coord_text: TextLayout<String>,
}

impl Default for Rectangle {
    fn default() -> Self {
        let mut layout = TextLayout::new();
        layout.set_font(crate::theme::UI_DETAIL_FONT);
        Rectangle {
            gesture: Default::default(),
            shift_locked: false,
            coord_text: layout,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GestureState {
    Ready,
    Down(DPoint),
    Begun { start: DPoint, current: DPoint },
    Finished,
}

impl Rectangle {
    fn pts_for_rect(&self) -> Option<(DPoint, DPoint)> {
        if let GestureState::Begun { start, current } = self.gesture {
            let mut current = current;
            if self.shift_locked {
                let mut vec2 = current - start;
                vec2.y = if vec2.y.signum() > 0.0 {
                    vec2.x.abs()
                } else {
                    vec2.x.abs() * -1.0
                };
                current = start + vec2;
            }
            Some((start, current))
        } else {
            None
        }
    }

    fn current_drag_rect(&self, data: &EditSession) -> Option<Rect> {
        let (start, current) = self.pts_for_rect()?;
        Some(Rect::from_points(
            data.viewport.to_screen(start),
            data.viewport.to_screen(current),
        ))
    }
}

impl Tool for Rectangle {
    fn name(&self) -> &'static str {
        "Rectangle"
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

    fn key_down(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        _: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        if key.key == KbKey::Shift {
            self.shift_locked = true;
            ctx.request_paint();
        }
        None
    }

    fn key_up(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        _: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        if key.key == KbKey::Shift {
            self.shift_locked = false;
            ctx.request_paint();
        }
        None
    }

    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        let pre_state = self.gesture;
        mouse.mouse_event(event, data, self);
        if pre_state != self.gesture {
            ctx.request_paint();
        }

        if self.gesture == GestureState::Finished {
            self.gesture = GestureState::Ready;
            Some(EditType::Normal)
        } else {
            None
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, env: &Env) {
        const LABEL_PADDING: f64 = 4.0;
        if let Some(rect) = self.current_drag_rect(data) {
            ctx.stroke(rect, &Color::BLACK, 1.0);
            let (start, current) = self.pts_for_rect().unwrap();
            let size = start - current;
            let label_text = format!("{}, {}", size.x.abs(), size.y.abs());
            self.coord_text.set_text(label_text);
            self.coord_text.rebuild_if_needed(ctx.text(), env);
            let text_size = self.coord_text.size();

            let text_x = rect.x1 - text_size.width - LABEL_PADDING;
            let text_y = rect.y1 + LABEL_PADDING;
            let text_pos = Point::new(text_x, text_y);

            let rect = Rect::from_origin_size(text_pos, text_size)
                .inset(2.0)
                .to_rounded_rect(2.0);
            ctx.fill(rect, &Color::WHITE.with_alpha(0.5));
            self.coord_text.draw(ctx, text_pos);
        }
    }
}

impl MouseDelegate<EditSession> for Rectangle {
    fn cancel(&mut self, _data: &mut EditSession) {
        self.gesture = GestureState::Ready;
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        if event.count == 1 {
            let pt = data.viewport.from_screen(event.pos);
            self.gesture = GestureState::Down(pt);
            self.shift_locked = event.mods.shift();
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, data: &mut EditSession) {
        if let Some((start, current)) = self.pts_for_rect() {
            let path = make_rect_path(start, current);
            data.paste_paths(vec![path.into()]);
            self.gesture = GestureState::Finished;
        }
    }

    fn left_drag_began(&mut self, event: Drag, data: &mut EditSession) {
        if let GestureState::Down(start) = self.gesture {
            let current = data.viewport.from_screen(event.current.pos);
            self.gesture = GestureState::Begun { start, current };
        }
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        if let GestureState::Begun { current, .. } = &mut self.gesture {
            *current = data.viewport.from_screen(drag.current.pos);
        }
    }
}

impl Default for GestureState {
    fn default() -> Self {
        GestureState::Ready
    }
}

fn make_rect_path(p1: DPoint, p3: DPoint) -> CubicPath {
    let path_id = EntityId::next();
    let p2 = DPoint::new(p3.x, p1.y);
    let p4 = DPoint::new(p1.x, p3.y);
    // first point goes last in closed paths
    let points = vec![
        PathPoint::on_curve(path_id, p2),
        PathPoint::on_curve(path_id, p3),
        PathPoint::on_curve(path_id, p4),
        PathPoint::on_curve(path_id, p1),
    ];
    CubicPath::from_raw_parts(path_id, points, None, true)
}
