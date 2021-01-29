//! The ellipse shape tool

// this share a lot of code with the rectangle tool :shrug:

use druid::kurbo::{PathEl, Shape};
use druid::{Color, Env, EventCtx, KbKey, KeyEvent, PaintCtx, Rect, RenderContext};

use crate::cubic_path::CubicPath;
use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::tools::{EditType, Tool};

/// The state of the ellipse tool.
#[derive(Debug, Default, Clone)]
pub struct Ellipse {
    gesture: GestureState,
    shift_locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GestureState {
    Ready,
    Begun { start: DPoint, current: DPoint },
    Finished,
}

impl Ellipse {
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

impl Tool for Ellipse {
    fn name(&self) -> &'static str {
        "Ellipse"
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, _env: &Env) {
        if let Some(rect) = self.current_drag_rect(data) {
            let ellipse = rect.to_ellipse();
            ctx.stroke(ellipse, &Color::grey(0.7), 1.0);
        }
    }
}

impl MouseDelegate<EditSession> for Ellipse {
    fn cancel(&mut self, _data: &mut EditSession) {
        self.gesture = GestureState::Ready;
    }

    fn left_drag_ended(&mut self, _drag: Drag, data: &mut EditSession) {
        if let Some((start, current)) = self.pts_for_rect() {
            let rect = Rect::from_points(start.to_raw(), current.to_raw());
            let ellipse = rect.to_ellipse();
            if let Ok(path) = CubicPath::from_bezpath(
                ellipse
                    .path_elements(1.0)
                    .chain(std::iter::once(PathEl::ClosePath)),
            ) {
                data.paste_paths(vec![path.into()]);
            }
            self.gesture = GestureState::Finished;
        }
    }

    fn left_drag_began(&mut self, event: Drag, data: &mut EditSession) {
        let start = data.viewport.from_screen(event.start.pos);
        let current = data.viewport.from_screen(event.current.pos);
        self.gesture = GestureState::Begun { start, current };
        self.shift_locked = event.current.mods.shift();
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
