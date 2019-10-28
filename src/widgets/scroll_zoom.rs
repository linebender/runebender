use druid::kurbo::{Point, Size, Vec2};
use druid::widget::Scroll;
use druid::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::data::EditorState;

use super::CANVAS_SIZE;

const MIN_ZOOM: f64 = 0.02;
const MAX_ZOOM: f64 = 50.;
/// mouse wheel deltas are big, so we scale them down
const ZOOM_SCALE: f64 = 0.001;

/// A widget that wraps a scroll widget, adding zoom.
pub struct ScrollZoom<T: Widget<EditorState>> {
    mouse: Point,
    child: Scroll<EditorState, T>,
    is_setup: bool,
}

impl<T: Widget<EditorState>> ScrollZoom<T> {
    pub fn new(inner: T) -> ScrollZoom<T> {
        ScrollZoom {
            child: Scroll::new(inner),
            mouse: Point::ZERO,
            is_setup: false,
        }
    }

    fn zoom(&mut self, data: &mut EditorState, delta: Vec2, size: Size) {
        let delta = most_significant_axis(delta);
        let delta = delta.round() * ZOOM_SCALE;
        if delta == 0. {
            return;
        }

        let last_zoom = data.session.viewport.zoom;
        let next_zoom = (last_zoom + delta).min(MAX_ZOOM).max(MIN_ZOOM);
        let delta_zoom = next_zoom / last_zoom;

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
        data.session.viewport.zoom = next_zoom;
    }

    /// center the glyph on the canvas
    fn set_initial_scroll(&mut self, data: &EditorState, view_size: Size) {
        // set scroll offsets so that the work is centered on load
        let work_rect = data.content_region();
        let canvas_size = CANVAS_SIZE * data.session.viewport.zoom;
        let work_size = work_rect.size() * data.session.viewport.zoom;

        // the top left of our glyph's layout rect
        let x_off = canvas_size.width / 2. - work_size.width / 2.;
        let y_off = canvas_size.height / 2. - work_size.height / 2.;

        // the center of our glyph's metric bounds
        let bonus_x = (view_size.width - work_size.width) / 2.;
        let bonus_y = (view_size.height - work_size.height) / 2.;

        let off = Vec2::new(x_off - bonus_x, y_off - bonus_y);
        let delta_off = off - self.child.offset();
        self.child.scroll(delta_off, view_size);
    }

    fn set_initial_zoom(&mut self, data: &mut EditorState, view_size: Size) {
        let work_size = data.content_region().size();
        let fit_ratio =
            (work_size.width / view_size.width).max(work_size.height / view_size.height);
        if fit_ratio > 1.0 {
            let new_zoom = 0.9 / fit_ratio;
            data.session.viewport.zoom = new_zoom;
        }
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
            self.set_initial_scroll(data, size);
            self.is_setup = true;
        }
        size
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut EditorState, env: &Env) {
        match event {
            Event::Size(size) if !self.is_setup => {
                self.set_initial_zoom(data, *size);
                ctx.request_focus();
            }
            Event::MouseMoved(mouse) => {
                self.mouse = mouse.pos;
            }
            Event::Wheel(wheel) if wheel.mods.meta => {
                self.zoom(data, wheel.delta, ctx.size());
                self.child.reset_scrollbar_fade(ctx);
                ctx.set_handled();
                ctx.invalidate();
                return;
            }
            // for debugging zoom:
            Event::KeyUp(key) if key.unmod_text() == Some("j") => {
                self.zoom(data, Vec2::new(50., 0.), ctx.size());
                self.child.reset_scrollbar_fade(ctx);
                ctx.invalidate();
            }
            Event::KeyUp(key) if key.unmod_text() == Some("k") => {
                self.zoom(data, Vec2::new(-50., 0.), ctx.size());
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
    }
}

fn most_significant_axis(delta: Vec2) -> f64 {
    if delta.x.abs() > delta.y.abs() {
        delta.x
    } else {
        delta.y
    }
}
