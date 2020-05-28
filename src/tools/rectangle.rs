//! The rectangle shape tool

use druid::kurbo::Vec2;
use druid::piet::{FontBuilder, PietTextLayout, Text, TextLayout, TextLayoutBuilder};
use druid::{
    Color, Env, EventCtx, KeyCode, KeyEvent, MouseEvent, PaintCtx, Point, Rect, RenderContext,
};

use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::{Path, PathPoint};
use crate::tools::{EditType, Tool};

/// The state of the rectangle tool.
#[derive(Debug, Default, Clone)]
pub struct Rectangle {
    gesture: GestureState,
    shift_locked: bool,
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

    fn label_text(&self, ctx: &mut PaintCtx) -> Option<PietTextLayout> {
        let (start, current) = self.pts_for_rect()?;
        let size = start - current;
        let mut text = ctx.text();
        let font = text.new_font_by_name("Helvetica", 10.0).build().unwrap();
        let label_text = format!("{}, {}", size.x.abs(), size.y.abs());
        text.new_text_layout(&font, &label_text, None).build().ok()
    }
}

impl Tool for Rectangle {
    fn name(&self) -> &'static str {
        "Rectangle"
    }

    fn key_down(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        _: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        if key.key_code == KeyCode::LeftShift || key.key_code == KeyCode::RightShift {
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
        if key.key_code == KeyCode::LeftShift || key.key_code == KeyCode::RightShift {
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, _env: &Env) {
        const LABEL_PADDING: f64 = 4.0;
        if let Some(rect) = self.current_drag_rect(data) {
            ctx.stroke(rect, &Color::BLACK, 1.0);
            let text = self.label_text(ctx).unwrap();
            let width = text.width();
            let height = text.line_metric(0).map(|m| m.height).unwrap_or_default();
            let ascent = text.line_metric(0).map(|m| m.baseline).unwrap_or_default();
            let text_x = rect.x1 - width - LABEL_PADDING;
            let text_y = rect.y1 + LABEL_PADDING;
            let text_pos = Point::new(text_x, text_y);

            let rect = Rect::from_origin_size(text_pos, (width, height))
                .inset(2.0)
                .to_rounded_rect(2.0);
            ctx.fill(rect, &Color::WHITE.with_alpha(0.5));
            ctx.draw_text(&text, text_pos + Vec2::new(0., ascent), &Color::BLACK);
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
            self.shift_locked = event.mods.shift;
        }
    }

    fn left_up(&mut self, _event: &MouseEvent, data: &mut EditSession) {
        if let Some((start, current)) = self.pts_for_rect() {
            let path = make_rect_path(start, current);
            data.paste_paths(vec![path]);
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

fn make_rect_path(p1: DPoint, p3: DPoint) -> Path {
    let path_id = crate::path::next_id();
    let p2 = DPoint::new(p3.x, p1.y);
    let p4 = DPoint::new(p1.x, p3.y);
    // first point goes last in closed paths
    let points = vec![
        PathPoint::on_curve(path_id, p2),
        PathPoint::on_curve(path_id, p3),
        PathPoint::on_curve(path_id, p4),
        PathPoint::on_curve(path_id, p1),
    ];
    Path::from_raw_parts(path_id, points, None, true)
}
