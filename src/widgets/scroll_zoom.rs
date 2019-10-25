use druid::kurbo::{Point, Size, Vec2};
use druid::widget::Scroll;
use druid::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::data::{glyph_rect, EditorState};

use super::CANVAS_SIZE;

const MIN_ZOOM: f64 = 0.02;
const MAX_ZOOM: f64 = 50.;

/// A widget that wraps a scroll widget, adding zoom.
pub struct ScrollZoom<T: Widget<EditorState>> {
    mouse: Point,
    child: Scroll<EditorState, T>,
    zoom: f64,
    is_setup: bool,
}

impl<T: Widget<EditorState>> ScrollZoom<T> {
    pub fn new(inner: T) -> ScrollZoom<T> {
        ScrollZoom {
            child: Scroll::new(inner),
            mouse: Point::ZERO,
            zoom: 1.0,
            is_setup: false,
        }
    }

    fn zoom(&mut self, delta: Vec2, size: Size) {
        let delta = most_significant_axis(delta);
        // deltas are big, so we scale them down
        let delta = delta.round() * 0.001;
        if delta == 0. {
            return;
        }

        let next_zoom = (self.zoom + delta).min(MAX_ZOOM).max(MIN_ZOOM);
        let delta_zoom = next_zoom / self.zoom;

        // prevents jitter when we're near our max or min zoom levels
        if (delta_zoom).abs() < 0.001 {
            return;
        }
        // we keep the mouse in the same relative position after zoom
        // by adjusting the scroll offsets:
        let scroll_off = self.child.offset() + self.mouse.to_vec2();
        let next_off = scroll_off * delta_zoom;
        let delta_off = next_off - scroll_off;
        self.child.scroll(delta_off, size);
        self.zoom = next_zoom;
    }

    /// center the glyph on the canvas
    fn setup(&mut self, data: &EditorState, view_size: Size) {
        // set scroll offsets so that the work is centered on load
        let work_rect = glyph_rect(data);

        // the top left of our glyph's layout rect
        let x_off = CANVAS_SIZE.width / 2. - work_rect.width() / 2.;
        let y_off = CANVAS_SIZE.height / 2. - work_rect.height() / 2.;

        let bonus_x = (view_size.width - work_rect.width()) / 2.;
        let bonus_y = (view_size.height - work_rect.height()) / 2.;

        let off = Vec2::new(x_off - bonus_x, y_off - bonus_y);

        self.child.scroll(off - self.child.offset(), view_size);
    }
}

impl<T: Widget<EditorState>> Widget<EditorState> for ScrollZoom<T> {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &EditorState, env: &Env) {
        use druid::piet::{Color, RenderContext};
        //TODO: paint grid here?
        ctx.clear(Color::rgb8(100, 100, 20));
        self.child.paint(ctx, state, data, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &EditorState,
        env: &Env,
    ) -> Size {
        let size = self.child.layout(ctx, bc, data, env);
        if !self.is_setup {
            self.setup(data, size);
            self.is_setup = true;
        }
        size
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut EditorState, env: &Env) {
        if let Event::MouseMoved(mouse) = event {
            self.mouse = mouse.pos;
            ctx.request_focus();
        }

        match event {
            Event::Wheel(wheel) if wheel.mods.meta => {
                self.zoom(wheel.delta, ctx.size());
                data.session.viewport.zoom = self.zoom;
                self.child.reset_scrollbar_fade(ctx);
                ctx.set_handled();
                ctx.invalidate();
                return;
            }

            // for debugging zoom:
            Event::KeyUp(key) if key.unmod_text() == Some("j") => {
                self.zoom(Vec2::new(50., 0.), ctx.size());
                data.session.viewport.zoom = self.zoom;
                self.child.reset_scrollbar_fade(ctx);
                ctx.invalidate();
            }
            Event::KeyUp(key) if key.unmod_text() == Some("k") => {
                self.zoom(Vec2::new(-50., 0.), ctx.size());
                data.session.viewport.zoom = self.zoom;
                self.child.reset_scrollbar_fade(ctx);
                ctx.invalidate();
            }
            _ => (),
        }

        self.child.event(event, ctx, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old: Option<&EditorState>,
        new: &EditorState,
        env: &Env,
    ) {
        self.child.update(ctx, old, new, env);
        //if Some(new.session.viewport.zoom) != old.map(|o| o.session.viewport.zoom) {
        //self.child.reset_scrollbar_fade();
        //}
    }
}

fn most_significant_axis(delta: Vec2) -> f64 {
    if delta.x.abs() > delta.y.abs() {
        delta.x
    } else {
        delta.y
    }
}
