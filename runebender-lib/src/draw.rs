//! Drawing algorithms and helpers

use std::sync::Arc;

use crate::component::Component;
use crate::data::{FontMetrics, Workspace};
use crate::design_space::ViewPort;
use crate::edit_session::EditSession;
use crate::guides::{Guide, GuideLine};
use crate::path::Path;
use crate::point::PointType;
use crate::point_list::RawSegment;
use crate::selection::Selection;
use crate::theme;

use druid::kurbo::{self, Affine, BezPath, Circle, CubicBez, Line, Point, Rect, Vec2};
use druid::piet::{Color, Piet, RenderContext};
use druid::{Env, PaintCtx};

use norad::Glyph;

/// A context for drawing that maps between screen space and design space.
struct DrawCtx<'a, 'b: 'a> {
    ctx: &'a mut Piet<'b>,
    env: &'a Env,
    space: ViewPort,
    /// the size of the drawing area
    visible_rect: Rect,
}

impl<'a, 'b> std::ops::Deref for DrawCtx<'a, 'b> {
    type Target = Piet<'b>;

    fn deref(&self) -> &Self::Target {
        self.ctx
    }
}

impl<'a, 'b> std::ops::DerefMut for DrawCtx<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl<'a, 'b: 'a> DrawCtx<'a, 'b> {
    fn new(ctx: &'a mut Piet<'b>, env: &'a Env, space: ViewPort, visible_rect: Rect) -> Self {
        DrawCtx {
            ctx,
            env,
            space,
            visible_rect,
        }
    }

    fn draw_metrics(&mut self, glyph: &Glyph, metrics: &FontMetrics, env: &Env) {
        let upm = metrics.units_per_em;
        let x_height = metrics.x_height.unwrap_or_else(|| (upm * 0.5).round());
        let cap_height = metrics.cap_height.unwrap_or_else(|| (upm * 0.7).round());
        let ascender = metrics.ascender.unwrap_or_else(|| (upm * 0.8).round());
        let descender = metrics.descender.unwrap_or_else(|| -(upm * 0.2).round());
        let hadvance = glyph
            .advance
            .as_ref()
            .map(|a| a.width as f64)
            .unwrap_or_else(|| (upm * 0.5).round());

        let metrics_color = env.get(theme::METRICS_COLOR);
        let bounds = Rect::from_points((0., descender), (hadvance, ascender));
        let bounds = self.space.rect_to_screen(bounds);
        self.stroke(bounds, &metrics_color, 1.0);

        let baseline = Line::new((0.0, 0.0), (hadvance, 0.0));
        let baseline = self.space.affine() * baseline;
        self.stroke(baseline, &metrics_color, 1.0);

        let x_height_guide = Line::new((0.0, x_height), (hadvance, x_height));
        let x_height_guide = self.space.affine() * x_height_guide;
        self.stroke(x_height_guide, &metrics_color, 1.0);

        let cap_height_guide = Line::new((0.0, cap_height), (hadvance, cap_height));
        let cap_height_guide = self.space.affine() * cap_height_guide;
        self.stroke(cap_height_guide, &metrics_color, 1.0);
    }

    fn draw_grid(&mut self) {
        const MIN_SCALE_FOR_GRID: f64 = 4.0;

        if self.space.zoom >= MIN_SCALE_FOR_GRID {
            // we draw the grid very lightly at low zoom levels.
            let grid_fade = ((self.space.zoom - MIN_SCALE_FOR_GRID) / 10.)
                .min(1.0)
                .max(0.05);
            let gray_val = 0xFF - (68. * grid_fade) as u8;
            let brush = Color::rgb8(gray_val, gray_val, gray_val);

            let visible_pixels =
                self.visible_rect.width().max(self.visible_rect.height()) / self.space.zoom;
            let visible_pixels = visible_pixels.ceil() as usize;

            let view_origin = self.space.inverse_affine() * self.visible_rect.origin();
            let Point { x, y } = view_origin.round();

            //NOTE: we are drawing in glyph space; y is up.

            // draw one line past what is visible.
            let x1 = x - 1.;
            let y1 = y + 1.;
            let len = 2.0 + visible_pixels as f64;
            for i in 0..=visible_pixels {
                let off = i as f64;
                let xmin = self.space.to_screen((x1 + off, y1));
                let xmax = self.space.to_screen((x1 + off, y1 - len));
                //TODO: this might mean that we draw lines at different pixel
                //intervals, based on how the rounding goes? is it better to floor()?
                let ymin = self.space.to_screen((x1, y1 - off)).round();
                let ymax = self.space.to_screen((x1 + len, y1 - off)).round();
                self.stroke(Line::new(xmin, xmax), &brush, 1.0);
                self.stroke(Line::new(ymin, ymax), &brush, 1.0);
            }
        }
    }

    fn draw_guides(&mut self, guides: &[Guide], sels: &Selection, env: &Env) {
        for guide in guides {
            let line = self.line_for_guide(guide);
            if sels.contains(&guide.id) {
                self.stroke(line, &env.get(theme::SELECTED_GUIDE_COLOR), 8.0);
            }
            self.stroke(line, &env.get(theme::GUIDE_COLOR), 0.5);
        }
    }

    fn line_for_guide(&self, guide: &Guide) -> Line {
        let view_origin = self.space.inverse_affine() * self.visible_rect.origin();
        let Point { x, y } = view_origin.round();
        let vis_size = self.visible_rect.size();
        let visible_pixels = ((vis_size.width.max(vis_size.height)) / self.space.zoom).ceil();
        match guide.guide {
            GuideLine::Horiz(p) => {
                let p1 = self.space.to_screen((x, p.y));
                let p2 = self.space.to_screen((x + visible_pixels, p.y));
                Line::new(p1, p2)
            }
            GuideLine::Vertical(p) => {
                let p1 = self.space.to_screen((p.x, y));
                let p2 = self.space.to_screen((p.x, y - visible_pixels));
                Line::new(p1, p2)
            }
            GuideLine::Angle { p1, p2 } => {
                let p1 = p1.to_screen(self.space);
                let p2 = p2.to_screen(self.space);
                let vec = (p2 - p1).normalize();
                let p1 = p2 - vec * 5000.; // an arbitrary number
                let p2 = p2 + vec * 5000.;
                Line::new(p1, p2)
            }
        }
    }

    fn draw_selected_segments(&mut self, path: &Path, sels: &Selection) {
        //FIXME: this is less efficient than it could be; we create and
        //check all segments of all paths, and we could at least just keep track
        //of whether a path contained *any* selected points, and short-circuit.
        let selected_seg_color = self.env.get(theme::SELECTED_LINE_SEGMENT_COLOR);
        for segment in path.segments_for_points(sels) {
            for segment in segment.kurbo_segments() {
                let seg = self.space.affine() * segment;
                //TODO: add width to theme
                self.stroke(&seg, &selected_seg_color, 3.0);
            }
        }
    }

    fn draw_path(&mut self, bez: &BezPath) {
        let path_color = self.env.get(theme::PATH_STROKE_COLOR);
        self.stroke(bez, &path_color, 1.0);
    }

    fn draw_filled(&mut self, session: &EditSession, font: &Workspace) {
        let bez = self.space.affine() * session.to_bezier();
        let fill_color = self.env.get(theme::PATH_FILL_COLOR);
        self.fill(bez, &fill_color);

        for comp in session.components.iter() {
            self.draw_component(comp, font, &fill_color);
        }
    }

    fn draw_control_point_lines(&mut self, path: &Path) {
        // if there is a trailing handle (the last operation was a click_drag
        // we need to draw that from the end point, which we track here.)
        let mut end_point = path.start_point().to_screen(self.space);

        for seg in path.iter_segments() {
            match seg.raw_segment() {
                RawSegment::Line(_, p1) => end_point = p1.to_screen(self.space),
                RawSegment::Cubic(p0, p1, p2, p3) => {
                    let r = self.space;
                    //FIXME: draw auto handles as dashed lines
                    self.draw_control_handle(p0.to_screen(r), p1.to_screen(r));
                    self.draw_control_handle(p2.to_screen(r), p3.to_screen(r));
                    end_point = p3.to_screen(r);
                }
            }
        }

        if let Some(trailing) = path.trailing() {
            if path.should_draw_trailing() {
                self.draw_control_handle(end_point, trailing.to_screen(self.space));
            }
        }
    }

    fn draw_control_handle(&mut self, p1: Point, p2: Point) {
        let handle_color = self.env.get(theme::OFF_CURVE_HANDLE_COLOR);
        let l = Line::new(p1, p2);
        self.stroke(l, &handle_color, 1.0);
    }

    fn draw_point(&mut self, point: PointStyle, env: &Env) {
        let PointStyle {
            style,
            point,
            selected,
        } = point;
        match style {
            Style::Open(seg) => self.draw_open_path_terminal(&seg, selected, env),
            Style::Close(seg) => self.draw_open_path_terminal(&seg, selected, env),
            Style::OffCurve => self.draw_off_curve_point(point, selected, env),
            Style::OffCurveAuto => self.draw_auto_point(point, selected, env),
            Style::Smooth => self.draw_smooth_point(point, selected, env),
            Style::Corner => self.draw_corner_point(point, selected, env),
        }
    }

    fn draw_open_path_terminal(&mut self, seg: &kurbo::PathSeg, selected: bool, env: &Env) {
        let cap = cap_line(seg.to_cubic(), 12.);
        if selected {
            let inner = cap_line(seg.to_cubic(), 8.);
            self.stroke(cap, &env.get(theme::SELECTED_POINT_OUTER_COLOR), 4.0);
            self.stroke(inner, &env.get(theme::SELECTED_POINT_INNER_COLOR), 2.0);
        } else {
            self.stroke(cap, &env.get(theme::OFF_CURVE_HANDLE_COLOR), 2.0);
        }
    }

    fn draw_smooth_point(&mut self, p: Point, selected: bool, env: &Env) {
        let radius = if selected {
            env.get(theme::SMOOTH_SELECTED_RADIUS)
        } else {
            env.get(theme::SMOOTH_RADIUS)
        };
        let circ = Circle::new(p, radius);
        if selected {
            self.fill(circ, &env.get(theme::SELECTED_POINT_INNER_COLOR));
            self.stroke(circ, &env.get(theme::SELECTED_POINT_OUTER_COLOR), 2.0);
        } else {
            self.fill(circ, &env.get(theme::SMOOTH_POINT_INNER_COLOR));
            self.stroke(circ, &env.get(theme::SMOOTH_POINT_OUTER_COLOR), 2.0);
        }
    }

    fn draw_corner_point(&mut self, p: Point, selected: bool, env: &Env) {
        let radius = if selected {
            env.get(theme::CORNER_SELECTED_RADIUS)
        } else {
            env.get(theme::CORNER_RADIUS)
        };
        let rect = Rect::new(p.x - radius, p.y - radius, p.x + radius, p.y + radius);
        if selected {
            self.fill(rect, &env.get(theme::SELECTED_POINT_INNER_COLOR));
            self.stroke(rect, &env.get(theme::SELECTED_POINT_OUTER_COLOR), 2.0);
        } else {
            self.fill(rect, &env.get(theme::CORNER_POINT_INNER_COLOR));
            self.stroke(rect, &env.get(theme::CORNER_POINT_OUTER_COLOR), 2.0);
        }
    }

    fn draw_off_curve_point(&mut self, p: Point, selected: bool, env: &Env) {
        let radius = if selected {
            env.get(theme::OFF_CURVE_SELECTED_RADIUS)
        } else {
            env.get(theme::OFF_CURVE_RADIUS)
        };
        let circ = Circle::new(p, radius);
        if selected {
            self.fill(circ, &env.get(theme::SELECTED_POINT_INNER_COLOR));
            self.stroke(circ, &env.get(theme::SELECTED_POINT_OUTER_COLOR), 2.0);
        } else {
            self.fill(circ, &env.get(theme::OFF_CURVE_POINT_INNER_COLOR));
            self.stroke(circ, &env.get(theme::OFF_CURVE_POINT_OUTER_COLOR), 2.0);
        }
    }

    fn draw_auto_point(&mut self, p: Point, selected: bool, env: &Env) {
        let radius = if selected {
            env.get(theme::OFF_CURVE_SELECTED_RADIUS)
        } else {
            env.get(theme::OFF_CURVE_RADIUS)
        };
        let rect = Rect::new(p.x - radius, p.y - radius, p.x + radius, p.y + radius);
        let line1 = Line::new(rect.origin(), (rect.x1, rect.y1));
        let line2 = Line::new((rect.x1, rect.y0), (rect.x0, rect.y1));
        if selected {
            self.stroke(line1, &env.get(theme::SELECTED_POINT_OUTER_COLOR), 4.0);
            self.stroke(line2, &env.get(theme::SELECTED_POINT_OUTER_COLOR), 4.0);
            self.stroke(line1, &env.get(theme::SELECTED_POINT_INNER_COLOR), 2.0);
            self.stroke(line2, &env.get(theme::SELECTED_POINT_INNER_COLOR), 2.0);
        } else {
            self.stroke(line1, &env.get(theme::OFF_CURVE_HANDLE_COLOR), 1.0);
            self.stroke(line2, &env.get(theme::OFF_CURVE_HANDLE_COLOR), 1.0);
        }
    }

    fn draw_direction_indicator(&mut self, path: &BezPath, env: &Env) {
        let first_seg = match path.segments().next().as_ref().map(|seg| seg.to_cubic()) {
            None => return,
            Some(cubic) => cubic,
        };

        let tangent = tangent_vector(0.05, first_seg).normalize();
        let angle = Vec2::new(tangent.y, -tangent.x);
        let rotate = Affine::rotate(angle.atan2());
        let translate = Affine::translate(first_seg.p0.to_vec2() + tangent * 8.0);
        let mut arrow = make_arrow();
        arrow.apply_affine(rotate);
        arrow.apply_affine(translate);
        self.fill(arrow, &env.get(theme::DIRECTION_ARROW_COLOR));
    }

    fn draw_component(&mut self, component: &Component, font: &Workspace, color: &Color) {
        if let Some(mut bez) = font.get_bezier(&component.base) {
            let bez = Arc::make_mut(&mut bez);
            bez.apply_affine(component.transform);
            bez.apply_affine(self.space.affine());
            self.fill(&*bez, color);
        }
    }
}

struct PointStyle {
    point: Point,
    style: Style,
    selected: bool,
}

#[derive(Debug, Clone)]
enum Style {
    Open(kurbo::PathSeg),
    Close(kurbo::PathSeg),
    Corner,
    Smooth,
    OffCurve,
    OffCurveAuto,
}

struct PointIter<'a> {
    idx: usize,
    vport: ViewPort,
    path: &'a Path,
    bez: &'a BezPath,
    sels: &'a Selection,
}

impl<'a> PointIter<'a> {
    fn new(path: &'a Path, vport: ViewPort, bez: &'a BezPath, sels: &'a Selection) -> Self {
        PointIter {
            idx: 0,
            vport,
            bez,
            path,
            sels,
        }
    }

    fn next_style(&self) -> Style {
        let len = self.path.points().len();
        if len == 1 {
            return Style::Corner;
        }

        let this = self.path.points()[self.idx];
        if this.is_on_curve() && !self.path.is_closed() {
            if self.idx == 0 {
                return Style::Open(self.bez.segments().next().unwrap());
            } else if self.idx == len - 1 {
                return Style::Close(self.bez.segments().last().unwrap().reverse());
            }
        }

        match this.typ {
            PointType::OffCurve { auto: true } if self.path.is_hyper() => Style::OffCurveAuto,
            PointType::OffCurve { .. } => Style::OffCurve,
            PointType::OnCurve { smooth: false } => Style::Corner,
            PointType::OnCurve { smooth: true } => Style::Smooth,
        }
    }
}

impl<'a> std::iter::Iterator for PointIter<'a> {
    type Item = PointStyle;
    fn next(&mut self) -> Option<PointStyle> {
        let point = self.path.points().get(self.idx)?;
        let style = self.next_style();
        let selected = self.sels.contains(&point.id);
        let point = point.to_screen(self.vport);
        self.idx += 1;
        Some(PointStyle {
            point,
            style,
            selected,
        })
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_session(
    ctx: &mut PaintCtx,
    env: &Env,
    space: ViewPort,
    visible_rect: Rect,
    metrics: &FontMetrics,
    session: &EditSession,
    font: &Workspace,
    is_preview: bool,
) {
    let mut draw_ctx = DrawCtx::new(&mut ctx.render_ctx, env, space, visible_rect);

    if is_preview {
        draw_ctx.draw_filled(session, font);
        return;
    }

    draw_ctx.draw_grid();
    draw_ctx.draw_metrics(&session.glyph, metrics, env);
    draw_ctx.draw_guides(&session.guides, &session.selection, env);

    for path in session.paths.iter() {
        if session.selection.len() > 1 {
            // for a segment to be selected at least two points must be selected
            draw_ctx.draw_selected_segments(path, &session.selection);
        }
        let bez = space.affine() * path.bezier();
        draw_ctx.draw_path(&bez);
        draw_ctx.draw_control_point_lines(path);
        draw_ctx.draw_direction_indicator(&bez, env);

        for point in PointIter::new(path, space, &bez, &session.selection) {
            draw_ctx.draw_point(point, env)
        }

        if let Some(pt) = path.trailing() {
            if path.should_draw_trailing() {
                draw_ctx.draw_auto_point(pt.to_screen(space), false, env);
            }
        }
    }

    for component in session.components.iter() {
        draw_ctx.draw_component(component, font, &env.get(theme::COMPONENT_FILL_COLOR));
    }
}

/// Return the tangent of the cubic bezier `cb`, at time `t`, as a vector
/// relative to the path's start point.
fn tangent_vector(t: f64, cb: CubicBez) -> Vec2 {
    debug_assert!((0.0..=1.0).contains(&t));
    let CubicBez { p0, p1, p2, p3 } = cb;
    let one_minus_t = 1.0 - t;
    3.0 * one_minus_t.powi(2) * (p1 - p0)
        + 6.0 * t * one_minus_t * (p2 - p1)
        + 3.0 * t.powi(2) * (p3 - p2)
}

/// Create a line of length `len` perpendicular to the tangent of the cubic
/// bezier `cb`, centered on the bezier's start point.
fn cap_line(cb: CubicBez, len: f64) -> Line {
    let tan_vec = tangent_vector(0.01, cb);
    let end = cb.p0 + tan_vec;
    perp(cb.p0, end, len)
}

/// Create a line perpendicular to the line `(p1, p2)`, centered on `p1`.
fn perp(p0: Point, p1: Point, len: f64) -> Line {
    let perp_vec = Vec2::new(p0.y - p1.y, p1.x - p0.x);
    let norm_perp = perp_vec / perp_vec.hypot();
    let p2 = p0 + (len * -0.5) * norm_perp;
    let p3 = p0 + (len * 0.5) * norm_perp;
    Line::new(p2, p3)
}

fn make_arrow() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((0., 18.));
    bez.line_to((-12., 0.));
    bez.line_to((12., 0.));
    bez.close_path();
    bez
}
