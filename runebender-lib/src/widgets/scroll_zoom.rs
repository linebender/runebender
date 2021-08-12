use druid::widget::prelude::*;
use druid::widget::Scroll;
use druid::{Color, Command, KbKey, Point, Vec2};

use crate::consts::CANVAS_SIZE;
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

    /// Updates zoom based on a delta from a scroll wheel
    fn wheel_zoom(&mut self, data: &mut EditorState, delta: Vec2, fixed_point: Option<Vec2>) {
        let last_zoom = data.session.viewport.zoom;
        let delta = most_significant_axis(delta);
        // we want to zoom in smaller units at smaller scales
        let zoom_scale = (last_zoom + 1.0).ln() * ZOOM_SCALE;

        let delta = delta.round() * zoom_scale;
        if delta == 0. {
            return;
        }

        let next_zoom = (last_zoom + delta).min(MAX_ZOOM).max(MIN_ZOOM);
        self.set_zoom(data, next_zoom, fixed_point)
    }

    fn pinch_zoom(&mut self, data: &mut EditorState, delta: f64) {
        let next_zoom = (data.session.viewport.zoom + delta)
            .min(MAX_ZOOM)
            .max(MIN_ZOOM);
        self.set_zoom(data, next_zoom, None)
    }

    /// Set the zoom multiplier directly.
    fn set_zoom(&mut self, data: &mut EditorState, new_zoom: f64, fixed_point: Option<Vec2>) {
        let fixed_point = fixed_point.unwrap_or_else(|| self.mouse.to_vec2());
        let delta_zoom = new_zoom / data.session.viewport.zoom;
        // prevents jitter when we're near our max or min zoom levels
        if (delta_zoom).abs() < 0.001 {
            return;
        }
        // we keep the mouse in the same relative position after zoom
        // by adjusting the scroll offsets:
        let scroll_off = self.child.offset() + fixed_point;
        let next_off = scroll_off * delta_zoom;
        let delta_off = next_off - scroll_off;
        self.child.scroll_by(delta_off);
        data.session_mut().viewport.zoom = new_zoom;
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
        self.child.scroll_by(delta_off);
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

        let canvas_rect = CANVAS_SIZE.to_rect();
        let work_offset = canvas_rect.center() - content_region.center();
        data.session_mut().viewport.set_offset(work_offset);
        data.session_mut().viewport.zoom = new_zoom;
    }

    fn handle_zoom_cmd(&mut self, cmd: &Command, view_size: Size, data: &mut EditorState) {
        use crate::consts::cmd;
        const ZOOM_DELTA: Vec2 = Vec2::new(50.0, 0.0);
        let view_center = view_size.to_rect().center().to_vec2();
        if cmd.is(cmd::ZOOM_IN) {
            self.wheel_zoom(data, ZOOM_DELTA, Some(view_center))
        } else if cmd.is(cmd::ZOOM_OUT) {
            self.wheel_zoom(data, -ZOOM_DELTA, Some(view_center))
        } else if cmd.is(cmd::ZOOM_DEFAULT) {
            self.set_zoom(data, 1.0, None);
            self.needs_center_after_layout = true;
        }
    }

    fn after_zoom_changed(&mut self, ctx: &mut EventCtx, _env: &Env) {
        //FIXME: this no longer exists in druid; we should rewrite this widget
        //using the druid 'scroll component' stuff, maybe?
        //self.child
        //.reset_scrollbar_fade(|d| ctx.request_timer(d), env);
        ctx.request_layout();
        ctx.set_handled();
    }
}

impl<T: Widget<EditorState>> Widget<EditorState> for ScrollZoom<T> {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorState, env: &Env) {
        //TODO: paint grid here?
        ctx.clear(Color::rgb8(100, 100, 20));
        self.child.paint(ctx, data, env);
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

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut EditorState, env: &Env) {
        use crate::consts::cmd;
        match event {
            Event::Command(c)
                if c.is(cmd::ZOOM_IN) || c.is(cmd::ZOOM_OUT) || c.is(cmd::ZOOM_DEFAULT) =>
            {
                self.handle_zoom_cmd(c, ctx.size(), data);
                self.after_zoom_changed(ctx, env);
                return;
            }
            Event::WindowSize(size) if self.needs_center_after_layout => {
                self.set_initial_viewport(data, *size);
                ctx.request_layout();
            }
            Event::KeyDown(k)
                if !k.repeat && matches!(&k.key, KbKey::Character(s) if s.as_str() == " ") =>
            {
                let cmd = cmd::TOGGLE_PREVIEW_TOOL.with(true);
                ctx.submit_command(cmd);
            }
            Event::KeyUp(k) if matches!(&k.key, KbKey::Character(s) if s.as_str() == " ") => {
                let cmd = cmd::TOGGLE_PREVIEW_TOOL.with(false);
                ctx.submit_command(cmd);
            }
            Event::MouseMove(mouse) => {
                self.mouse = mouse.pos;
            }
            Event::Wheel(wheel) if wheel.mods.alt() => {
                self.wheel_zoom(data, wheel.wheel_delta, None);
                self.after_zoom_changed(ctx, env);
                return;
            }
            Event::Zoom(delta) => {
                self.pinch_zoom(data, *delta);
                self.after_zoom_changed(ctx, env);
                return;
            }
            _ => (),
        }
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &EditorState,
        env: &Env,
    ) {
        self.child.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &EditorState, new: &EditorState, env: &Env) {
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
