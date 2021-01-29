use druid::kurbo::{Circle, Line, Point, Rect, Size, Vec2};
use druid::piet::{Color, FontFamily, RenderContext, Text, TextLayout, TextLayoutBuilder};
use druid::{Data, Env, EventCtx, PaintCtx};

use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::tools::{EditType, Tool};

#[derive(Default)]
pub struct Measure {
    line: Option<Line>,
}

const MEASURE_LINE_STROKE_COLOR: Color = Color::rgb8(0x73, 0x9B, 0xCB);
const MEASURE_INFO_BG_COLOR: Color = Color::rgb8(0x70, 0x70, 0x70);
const MEASURE_INFO_FG_COLOR: Color = Color::rgb8(0xf8, 0xf8, 0xf8);
const MEASURE_INFO_ONCURVE_COLOR: Color = Color::rgb8(0x80, 0x80, 0xe0);
const MEASURE_INFO_OFFCURVE_COLOR: Color = Color::rgb8(0x60, 0xc0, 0x60);
const MEASURE_INFO_DELTA_COLOR: Color = Color::rgb8(0xa0, 0x20, 0x20);
const MEASURE_INFO_FONT_SIZE: f64 = 9.0;
const MEASURE_INTERSECTION_RADIUS: f64 = 3.0;

// Don't report segments smaller than this.
const MEASURE_FUZZY_TOLERANCE: f64 = 0.1;

fn draw_info_bubble(ctx: &mut PaintCtx, pos: Point, label: impl Into<String>) {
    let text = ctx.text();
    let layout = text
        .new_text_layout(label.into())
        .font(FontFamily::SYSTEM_UI, MEASURE_INFO_FONT_SIZE)
        .text_color(MEASURE_INFO_FG_COLOR)
        .build()
        .unwrap();
    let width = layout.size().width;
    let bubble = Rect::from_center_size(pos, Size::new(width + 6.0, 12.0)).to_rounded_rect(6.0);
    let origin = pos - Vec2::new(0.5 * width, 6.5);
    ctx.fill(bubble, &MEASURE_INFO_BG_COLOR);
    ctx.draw_text(&layout, origin);
}

fn atan_to_angle(atan: f64) -> f64 {
    if !atan.is_finite() {
        return 0.0;
    }
    let mut angle = atan * (-180.0 / std::f64::consts::PI);
    if angle < -90.0 {
        angle += 360.0;
    }
    angle
}

fn draw_label(ctx: &mut PaintCtx, label: String, pos: Point, color: Color) {
    let text = ctx.text();
    let layout = text
        .new_text_layout(label)
        .font(FontFamily::SYSTEM_UI, MEASURE_INFO_FONT_SIZE)
        .text_color(color)
        .build()
        .unwrap();
    ctx.draw_text(&layout, pos);
}

fn format_pt(pt: DPoint) -> String {
    let x = format!("{:.1}", pt.x);
    let y = format!("{:.1}", pt.y);
    format!("{}, {}", x.trim_end_matches(".0"), y.trim_end_matches(".0"))
}

impl Measure {
    #[allow(clippy::float_cmp)]
    fn compute_measurement(&self, data: &EditSession, design_line: Line) -> Vec<f64> {
        // We scale the intersections to fixed point to make them easier to sort.
        const T_SCALE: f64 = (1u64 << 63) as f64;
        let mut intersections = vec![0, T_SCALE as u64];
        for path in &*data.paths {
            for seg in path.iter_segments() {
                for intersection in seg.intersect_line(design_line) {
                    let t_fixed = (intersection.line_t.max(0.0).min(1.0) * T_SCALE) as u64;
                    intersections.push(t_fixed);
                }
            }
        }
        intersections.sort_unstable();

        // Fuzzy deduplication
        let thresh = MEASURE_FUZZY_TOLERANCE / (design_line.p1 - design_line.p0).hypot();
        let mut result = Vec::with_capacity(intersections.len());
        let mut t_cluster_start = -1.0;
        let mut t_last = -1.0;
        for t_fixed in intersections {
            let t = t_fixed as f64 / T_SCALE;
            if t - t_last > thresh {
                t_cluster_start = t;
                result.push(t);
            } else {
                let cluster_t = if t_cluster_start == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    0.5 * (t_cluster_start + t)
                };
                *result.last_mut().unwrap() = cluster_t;
            }
            t_last = t;
        }
        result
    }

    // This is split out in case we want to sometimes hide the coords.
    fn paint_coords(&self, ctx: &mut PaintCtx, data: &EditSession) {
        for path in &*data.paths {
            for pt in path.points() {
                let scr_pt = data.viewport.to_screen(pt.point);
                let label = format_pt(pt.point);
                let color = if pt.is_on_curve() {
                    MEASURE_INFO_ONCURVE_COLOR
                } else {
                    MEASURE_INFO_OFFCURVE_COLOR
                };
                draw_label(ctx, label, scr_pt, color);
            }
            for seg in path.iter_segments() {
                let mid_pt = seg.eval(0.5);
                let scr_pt = data.viewport.to_screen(DPoint::from_raw(mid_pt));
                let delta = seg.raw_segment().start().point - seg.raw_segment().end().point;
                let label = format_pt(DPoint::new(delta.x, delta.y));
                // TODO: nudge placement of label to reduce crowding
                draw_label(ctx, label, scr_pt, MEASURE_INFO_DELTA_COLOR);
            }
        }
    }
}

impl Tool for Measure {
    fn name(&self) -> &'static str {
        "Measure"
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, _env: &Env) {
        self.paint_coords(ctx, data);
        if let Some(line) = self.line {
            let angle = atan_to_angle((line.p1 - line.p0).atan2());
            let angle_offset = if angle < 90.0 {
                Vec2::new(14.0, -6.0)
            } else if angle < 180.0 {
                Vec2::new(-14.0, -6.0)
            } else {
                Vec2::new(-14.0, 8.0)
            };
            ctx.stroke(line, &MEASURE_LINE_STROKE_COLOR, 1.0);
            let label = format!("{:.1}°", angle);
            draw_info_bubble(ctx, line.p1 + angle_offset, label);
            // TODO: compute earlier than paint
            if let Some(line) = self.line {
                let p0 = data.viewport.from_screen(line.p0);
                let p1 = data.viewport.from_screen(line.p1);
                let design_line = Line::new(p0.to_raw(), p1.to_raw());
                let design_len = (design_line.p1 - design_line.p0).hypot();
                let intersections = self.compute_measurement(data, design_line);
                for t in &intersections {
                    let pt = line.p0.lerp(line.p1, *t);
                    let circle = Circle::new(pt, MEASURE_INTERSECTION_RADIUS);
                    ctx.fill(circle, &MEASURE_LINE_STROKE_COLOR);
                }
                for i in 0..intersections.len() - 1 {
                    let t0 = intersections[i];
                    let t1 = intersections[i + 1];
                    let tmid = 0.5 * (t0 + t1);
                    let seg_len = design_len * (t1 - t0);
                    let center = design_line.p0.lerp(design_line.p1, tmid);
                    let center_screen = data.viewport.to_screen(DPoint::from_raw(center));
                    let len_label = format!("{:.1}", seg_len);
                    draw_info_bubble(ctx, center_screen, len_label);
                }
            }
        }
    }

    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        let pre_line = self.line;
        mouse.mouse_event(event, data, self);
        if !pre_line.same(&self.line) {
            ctx.request_paint();
        }
        None
    }
}

impl MouseDelegate<EditSession> for Measure {
    fn cancel(&mut self, _data: &mut EditSession) {
        self.line = None;
    }

    fn left_drag_began(&mut self, drag: Drag, _data: &mut EditSession) {
        self.line = Some(Line::new(drag.start.pos, drag.current.pos));
    }

    fn left_drag_changed(&mut self, drag: Drag, _data: &mut EditSession) {
        if let Some(line) = &mut self.line {
            let mut pos = drag.current.pos;
            if drag.current.mods.shift() {
                pos = super::axis_locked_point(pos, drag.start.pos);
            }
            line.p1 = pos;
        }
    }
}
