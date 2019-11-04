use druid::kurbo::{Point, Rect, Size, Vec2};
use druid::widget::Scroll;
use druid::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, Selector, UpdateCtx,
    Widget,
};

use crate::consts::{cmd::REQUEST_FOCUS, CANVAS_SIZE};
use crate::data::EditorState;

const MIN_ZOOM: f64 = 0.02;
const MAX_ZOOM: f64 = 50.;
/// mouse wheel deltas are big, so we scale them down
const ZOOM_SCALE: f64 = 0.001;

/// A widget that wraps a scroll widget, adding zoom.
pub struct ScrollZoom<T: Widget<EditorState>> {
    mouse: Point,
    child: Scroll<EditorState, T>,
    needs_center_after_layout: bool,
}

impl<T: Widget<EditorState>> ScrollZoom<T> {
    pub fn new(inner: T) -> ScrollZoom<T> {
        ScrollZoom {
            child: Scroll::new(inner),
            mouse: Point::ZERO,
            needs_center_after_layout: true,
        }
    }

    fn zoom(&mut self, data: &mut EditorState, delta: Vec2, size: Size, fixed_point: Option<Vec2>) {
        let fixed_point = fixed_point.unwrap_or(self.mouse.to_vec2());
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
        let scroll_off = self.child.offset() + fixed_point;
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

    /// set the initial zoom and offset, so that the work is positioned in the center
    /// of the canvas.
    fn set_initial_viewport(&mut self, data: &mut EditorState, view_size: Size) {
        let content_region = data.content_region();
        let work_size = data.content_region().size();
        let fit_ratio =
            (work_size.width / view_size.width).max(work_size.height / view_size.height);
        let new_zoom = if fit_ratio > 1.0 {
            0.9 / fit_ratio
        } else {
            1.0
        };

        let canvas_rect = Rect::ZERO.with_size(CANVAS_SIZE);
        let work_offset = canvas_rect.center() - content_region.center();
        data.session.viewport.set_offset(work_offset);
        data.session.viewport.zoom = new_zoom;
    }

    fn handle_zoom_cmd(&mut self, sel: &Selector, view_size: Size, data: &mut EditorState) {
        use crate::consts::cmd;
        let view_center = Rect::ZERO.with_size(view_size).center().to_vec2();
        match sel {
            &cmd::ZOOM_IN => self.zoom(data, Vec2::new(50.0, 0.), view_size, Some(view_center)),
            &cmd::ZOOM_OUT => self.zoom(data, Vec2::new(-50.0, 0.), view_size, Some(view_center)),
            &cmd::ZOOM_DEFAULT => {
                let current_zoom = data.session.viewport.zoom;
                let delta = (1.0 - current_zoom) * ZOOM_SCALE.recip();
                let dzoom = Vec2::new(delta, 0.0);
                self.zoom(data, dzoom, view_size, None);
                self.needs_center_after_layout = true;
            }
            _ => unreachable!("selectors have already been validated"),
        };
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
        if self.needs_center_after_layout {
            self.set_initial_scroll(data, size);
            self.needs_center_after_layout = false;
        }
        size
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut EditorState, env: &Env) {
        use crate::consts::cmd;
        match event {
            Event::Command(c)
                if c.selector == cmd::ZOOM_IN
                    || c.selector == cmd::ZOOM_OUT
                    || c.selector == cmd::ZOOM_DEFAULT =>
            {
                self.handle_zoom_cmd(&c.selector, ctx.size(), data);
                self.child.reset_scrollbar_fade(ctx);
                ctx.invalidate();
                ctx.set_handled();
                return;
            }
            Event::Size(size) if self.needs_center_after_layout => {
                self.set_initial_viewport(data, *size);
                //HACK: because of how WidgetPod works this event isn't propogated,
                //so we need to use a command to tell the Editor struct to request focus
                ctx.submit_command(REQUEST_FOCUS.into(), None);
            }
            Event::MouseMoved(mouse) => {
                self.mouse = mouse.pos;
            }
            Event::Wheel(wheel) if wheel.mods.meta => {
                self.zoom(data, wheel.delta, ctx.size(), None);
                self.child.reset_scrollbar_fade(ctx);
                ctx.set_handled();
                ctx.invalidate();
                return;
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
