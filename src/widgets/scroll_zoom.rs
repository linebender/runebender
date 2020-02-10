use druid::piet::{Color, FontBuilder, RenderContext, Text, TextLayout, TextLayoutBuilder};

use druid::kurbo::{Point, Rect, RoundedRect, Size, Vec2};
use druid::widget::Scroll;
use druid::{
    BoxConstraints, Command, Env, Event, EventCtx, HotKey, KeyCode, LayoutCtx, LifeCycle,
    LifeCycleCtx, PaintCtx, Selector, UpdateCtx, Widget,
};

use crate::consts::CANVAS_SIZE;
use crate::data::EditorState;

const MIN_ZOOM: f64 = 0.02;
const MAX_ZOOM: f64 = 50.;
/// mouse wheel deltas are big, so we scale them down
const ZOOM_SCALE: f64 = 0.001;
const TOOL_LABEL_SIZE: f64 = 18.0;
/// A general ballpark guess at what the ascender height is, as a ratio of font size.
const LIKELY_ASCENDER_RATIO: f64 = 0.8;

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
    fn wheel_zoom(
        &mut self,
        data: &mut EditorState,
        delta: Vec2,
        size: Size,
        fixed_point: Option<Vec2>,
    ) {
        let last_zoom = data.session.viewport.zoom;
        let delta = most_significant_axis(delta);
        // we want to zoom in smaller units at smaller scales
        let zoom_scale = (last_zoom + 1.0).ln() * ZOOM_SCALE;

        let delta = delta.round() * zoom_scale;
        if delta == 0. {
            return;
        }

        let next_zoom = (last_zoom + delta).min(MAX_ZOOM).max(MIN_ZOOM);
        self.set_zoom(data, next_zoom, size, fixed_point)
    }

    fn pinch_zoom(&mut self, data: &mut EditorState, delta: f64, size: Size) {
        let next_zoom = (data.session.viewport.zoom + delta)
            .min(MAX_ZOOM)
            .max(MIN_ZOOM);
        self.set_zoom(data, next_zoom, size, None)
    }

    /// Set the zoom multiplier directly.
    fn set_zoom(
        &mut self,
        data: &mut EditorState,
        new_zoom: f64,
        size: Size,
        fixed_point: Option<Vec2>,
    ) {
        let fixed_point = fixed_point.unwrap_or(self.mouse.to_vec2());
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
        self.child.scroll(delta_off, size);
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
        data.session_mut().viewport.set_offset(work_offset);
        data.session_mut().viewport.zoom = new_zoom;
    }

    fn handle_zoom_cmd(&mut self, sel: &Selector, view_size: Size, data: &mut EditorState) {
        use crate::consts::cmd;
        let view_center = Rect::ZERO.with_size(view_size).center().to_vec2();
        match sel {
            &cmd::ZOOM_IN => {
                self.wheel_zoom(data, Vec2::new(50.0, 0.), view_size, Some(view_center))
            }
            &cmd::ZOOM_OUT => {
                self.wheel_zoom(data, Vec2::new(-50.0, 0.), view_size, Some(view_center))
            }
            &cmd::ZOOM_DEFAULT => {
                self.set_zoom(data, 1.0, view_size, None);
                self.needs_center_after_layout = true;
            }
            _ => unreachable!("selectors have already been validated"),
        };
    }

    fn paint_label(&mut self, ctx: &mut PaintCtx, data: &EditorState, _: &Env) {
        let font_name = label_font_name();
        let bg_color = Color::WHITE.with_alpha(0.7);
        let text_color = Color::grey(0.45);

        let font = ctx
            .text()
            .new_font_by_name(font_name, TOOL_LABEL_SIZE)
            .build()
            .unwrap();
        let layout = ctx
            .text()
            .new_text_layout(&font, &data.session.tool_desc)
            .build()
            .unwrap();
        let text_size = Size::new(layout.width(), TOOL_LABEL_SIZE);
        let text_draw_origin = Point::new(10.0, 10.0 + TOOL_LABEL_SIZE * LIKELY_ASCENDER_RATIO);
        let text_bounds = Rect::from_origin_size((10.0, 10.0), text_size);
        let text_bg_bounds = text_bounds.inset(4.0);
        //TODO: use Rect::to_rounded_rect when available
        let text_box = RoundedRect::from_rect(text_bg_bounds, 4.0);
        ctx.fill(text_box, &bg_color);
        ctx.stroke(text_box, &text_color, 0.5);
        ctx.draw_text(&layout, text_draw_origin, &text_color);
    }
}

impl<T: Widget<EditorState>> Widget<EditorState> for ScrollZoom<T> {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorState, env: &Env) {
        //TODO: paint grid here?
        ctx.clear(Color::rgb8(100, 100, 20));
        self.child.paint(ctx, data, env);
        self.paint_label(ctx, data, env);
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
                if c.selector == cmd::ZOOM_IN
                    || c.selector == cmd::ZOOM_OUT
                    || c.selector == cmd::ZOOM_DEFAULT =>
            {
                self.handle_zoom_cmd(&c.selector, ctx.size(), data);
                self.child.reset_scrollbar_fade(ctx, env);
                ctx.request_paint();
                ctx.set_handled();
                return;
            }
            Event::Size(size) if self.needs_center_after_layout => {
                self.set_initial_viewport(data, *size)
            }
            Event::KeyDown(k) if HotKey::new(None, "v").matches(k) => {
                ctx.submit_command(cmd::SELECT_TOOL, None)
            }
            Event::KeyDown(k) if HotKey::new(None, "p").matches(k) => {
                ctx.submit_command(cmd::PEN_TOOL, None)
            }
            Event::KeyDown(k) if HotKey::new(None, "h").matches(k) => {
                ctx.submit_command(cmd::PREVIEW_TOOL, None)
            }
            Event::KeyDown(k) if !k.is_repeat && k.key_code == KeyCode::Space => {
                let cmd = Command::new(cmd::TOGGLE_PREVIEW_TOOL, true);
                ctx.submit_command(cmd, None);
            }
            Event::KeyUp(k) if k.key_code == KeyCode::Space => {
                let cmd = Command::new(cmd::TOGGLE_PREVIEW_TOOL, false);
                ctx.submit_command(cmd, None);
            }
            Event::MouseMoved(mouse) => {
                self.mouse = mouse.pos;
            }
            Event::Wheel(wheel) if wheel.mods.meta => {
                self.wheel_zoom(data, wheel.delta, ctx.size(), None);
                self.child.reset_scrollbar_fade(ctx, env);
                ctx.set_handled();
                ctx.request_paint();
                return;
            }
            Event::Zoom(delta) => {
                self.pinch_zoom(data, *delta, ctx.size());
                self.child.reset_scrollbar_fade(ctx, env);
                ctx.set_handled();
                ctx.request_paint();
                return;
            }
            _ => (),
        }
        self.child.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &EditorState, _: &Env) {}

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

#[allow(unreachable_code)]
fn label_font_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        return "Helvetica Neue Bold";
    }
    #[cfg(target_os = "windows")]
    {
        return "Segoe UI Bold";
    }
    "sans-serif"
}
