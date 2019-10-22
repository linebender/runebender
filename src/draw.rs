//! Drawing algorithms and helpers

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::component::Component;
use crate::data::FontMetrics;
use crate::design_space::ViewPort;
use crate::edit_session::EditSession;
use crate::guides::{Guide, GuideLine};
use crate::path::{EntityId, Path, PointType};
//use super::{Tool, ViewPort};
use druid::kurbo::{Affine, BezPath, Circle, CubicBez, Line, PathSeg, Point, Rect, Size, Vec2};
use druid::piet::{Color, Piet, RenderContext};
use druid::PaintCtx;

use norad::{Glyph, Ufo};

const PATH_COLOR: Color = Color::rgb8(0x00, 0x00, 0x00);
const METRICS_COLOR: Color = Color::rgb8(0xA0, 0xA0, 0xA0);
const GUIDE_COLOR: Color = Color::rgb8(0xFC, 0x54, 0x93);
const SELECTED_GUIDE_COLOR: Color = Color::rgb8(0xFE, 0xED, 0xED);
const SELECTION_RECT_BG_COLOR: Color = Color::rgba8(0xDD, 0xDD, 0xDD, 0x55);
const SELECTION_RECT_STROKE_COLOR: Color = Color::rgb8(0x53, 0x8B, 0xBB);
const SMOOTH_POINT_COLOR: Color = Color::rgb8(0x_41, 0x8E, 0x22);
const CORNER_POINT_COLOR: Color = Color::rgb8(0x0b, 0x2b, 0xdb);
const OFF_CURVE_POINT_COLOR: Color = Color::rgb8(0xbb, 0xbb, 0xbb);
const OFF_CURVE_HANDLE_COLOR: Color = Color::rgb8(0xbb, 0xbb, 0xbb);
const DIRECTION_ARROW_COLOR: Color = Color::rgba8(0x00, 0x00, 0x00, 0x44);
const COMPONENT_FILL_COLOR: Color = Color::rgba8(0, 0, 0, 0x44);

const SMOOTH_RADIUS: f64 = 3.5;
const SMOOTH_SELECTED_RADIUS: f64 = 4.;
const OFF_CURVE_RADIUS: f64 = 2.;
const OFF_CURVE_SELECTED_RADIUS: f64 = 2.5;

/// A context for drawing that maps between screen space and design space.
struct DrawCtx<'a, 'b: 'a> {
    ctx: &'a mut Piet<'b>,
    space: ViewPort,
}

impl<'a, 'b> std::ops::Deref for DrawCtx<'a, 'b> {
    type Target = Piet<'b>;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl<'a, 'b> std::ops::DerefMut for DrawCtx<'a, 'b> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ctx
    }
}

impl<'a, 'b: 'a> DrawCtx<'a, 'b> {
    fn new(ctx: &'a mut Piet<'b>, space: ViewPort) -> Self {
        DrawCtx { ctx, space }
    }

    fn draw_metrics(&mut self, glyph: &Glyph, metrics: &FontMetrics) {
        let upm = metrics.units_per_em;
        //let cap_height = metrics.cap_height.unwrap_or((upm * 0.7).round());
        let ascender = metrics.ascender.unwrap_or((upm * 0.8).round());
        let descender = metrics.descender.unwrap_or(-(upm * 0.2).round());
        let hadvance = glyph
            .advance
            .as_ref()
            .map(|a| a.width as f64)
            .unwrap_or((upm * 0.5).round());
        let bounds = Rect::from_points((0., descender), (hadvance, ascender));
        self.stroke(bounds, &METRICS_COLOR, 1.0);
        let baseline = Line::new((0.0, 0.0), (hadvance, 0.0));
        self.stroke(baseline, &METRICS_COLOR, 1.0);
    }

    fn draw_grid(&mut self) {
        if self.space.zoom >= 8.0 {
            let grid_fade = ((self.space.zoom - 8.) / 10.).min(1.0).max(0.01);
            let gray_val = 0xFF - (0x44 as f64 * grid_fade) as u8;
            //let gray = gray_val << 16 | gray_val << 8 | gray_val;
            //let brush = self.solid_brush(Color::rgb24(gray));
            let brush = Color::rgb8(gray_val, gray_val, gray_val);

            // TODO: use view size
            // TODO: more efficient maybe to just save the grid as a bezier,
            // then just transform and draw?
            let visible_pixels = 2000 / self.space.zoom as usize;
            let view_origin = self.space.transform().inverse() * Point::new(0., 0.);
            let Point { x, y } = view_origin.round();
            let x1 = x - 1.;
            let y1 = y - 1.;
            for i in 0..=visible_pixels {
                let off = i as f64;
                let len = visible_pixels as f64;
                let xmin = self.space.to_screen((x1 + off, y1));
                let xmax = self.space.to_screen((x1 + off, y1 + len));
                let ymin = self.space.to_screen((x1, y1 + off));
                let ymax = self.space.to_screen((x1 + len, y1 + off));
                self.stroke(Line::new(xmin, xmax), &brush, 1.0);
                self.stroke(Line::new(ymin, ymax), &brush, 1.0);
            }
        }
    }

    fn draw_guides(&mut self, guides: &[Guide], sels: &BTreeSet<EntityId>) {
        //eprintln!("drawing {} guides", guides.len());
        //let view_origin = self.space.transform().inverse() * Point::new(0., 0.);
        //let Point { x, y } = view_origin.round();
        //let visible_pixels = 2000. / self.space.zoom;
        //let bounds = Rect::from_points((x, y), (x + visible_pixels, y + visible_pixels));

        let brush = self.solid_brush(GUIDE_COLOR);
        let sel_brush = self.solid_brush(SELECTED_GUIDE_COLOR);
        for guide in guides {
            let line = self.line_for_guide(guide);
            //if intersects(line, bounds) {
            //eprintln!("drawing {:?}", line);
            if sels.contains(&guide.id) {
                self.stroke(line, &sel_brush, 8.0);
            }
            self.stroke(line, &brush, 0.5);
            //} else {
            //eprintln!("skipping {:?}", guide);
            //}
        }
    }

    fn line_for_guide(&self, guide: &Guide) -> Line {
        let view_origin = self.space.transform().inverse() * Point::new(0., 0.);
        let Point { x, y } = view_origin.round();
        let visible_pixels = 2000. / self.space.zoom;
        match guide.guide {
            GuideLine::Horiz(p) => {
                let p1 = self.space.to_screen((x, p.y));
                let p2 = self.space.to_screen((x + visible_pixels, p.y));
                Line::new(p1, p2)
            }
            GuideLine::Vertical(p) => {
                let p1 = self.space.to_screen((p.x, y));
                let p2 = self.space.to_screen((p.x, y + visible_pixels));
                Line::new(p1, p2)
            }
            GuideLine::Angle { p1, p2 } => {
                let p1 = p1.to_screen(self.space);
                let p2 = p2.to_screen(self.space);
                let vec = (p2 - p1).normalize();
                let p1 = p2 - vec * 5000.; // an arbitrary number
                let p2 = p2 + vec * 5000.;
                Line::new(p1, p2)
            } //Line::new(Point::ZERO, Point::ZERO),
        }
    }

    fn draw_path(&mut self, bez: &BezPath) {
        let path_brush = self.solid_brush(PATH_COLOR);
        self.stroke(bez, &path_brush, 1.0);
    }

    fn draw_filled_paths(&mut self, paths: &[Path]) {
        for p in paths {
            let bez = self.space.transform() * p.bezier().clone();
            self.fill(bez, &Color::BLACK);
        }
    }

    fn draw_control_point_lines(&mut self, path: &Path) {
        let mut prev_point = path.start_point().to_screen(self.space);
        let mut idx = 0;
        while idx < path.points().len() {
            match path.points()[idx] {
                p if p.is_on_curve() => prev_point = p.to_screen(self.space),
                p => {
                    self.draw_control_handle(prev_point, p.to_screen(self.space));
                    let p1 = path.points()[idx + 1].to_screen(self.space);
                    let p2 = path.points()[idx + 2].to_screen(self.space);
                    self.draw_control_handle(p1, p2);
                    idx += 2;
                    prev_point = p2;
                }
            }
            idx += 1;
        }

        if let Some(trailing) = path.trailing() {
            if path.should_draw_trailing() {
                self.draw_control_handle(prev_point, trailing.to_screen(self.space));
            }
        }
    }

    fn draw_control_handle(&mut self, p1: Point, p2: Point) {
        let l = Line::new(p1, p2);
        self.stroke(l, &OFF_CURVE_HANDLE_COLOR, 1.0);
    }

    fn draw_point(&mut self, point: PointStyle) {
        let PointStyle {
            style,
            point,
            selected,
        } = point;
        match style {
            Style::Open(seg) => self.draw_open_path_terminal(&seg, selected),
            Style::Close(seg) => self.draw_open_path_terminal(&seg, selected),
            Style::OffCurve => self.draw_off_curve_point(point, selected),
            Style::Smooth => self.draw_smooth_point(point, selected),
            Style::Tangent => self.draw_smooth_point(point, selected),
            Style::Corner => self.draw_corner_point(point, selected),
        }
    }

    fn draw_open_path_terminal(&mut self, seg: &PathSeg, selected: bool) {
        let cap = cap_line(seg.to_cubic(), 12.);
        if selected {
            self.stroke(cap, &OFF_CURVE_HANDLE_COLOR, 3.0);
        } else {
            self.stroke(cap, &OFF_CURVE_HANDLE_COLOR, 2.0);
        }
    }

    fn draw_smooth_point(&mut self, p: Point, selected: bool) {
        let radius = if selected {
            SMOOTH_SELECTED_RADIUS
        } else {
            SMOOTH_RADIUS
        };
        let circ = Circle::new(p, radius);
        if selected {
            self.fill(circ, &SMOOTH_POINT_COLOR);
        } else {
            self.stroke(circ, &SMOOTH_POINT_COLOR, 1.0);
        }
    }

    fn draw_corner_point(&mut self, p: Point, selected: bool) {
        let radius = if selected {
            SMOOTH_SELECTED_RADIUS
        } else {
            SMOOTH_RADIUS
        };
        let rect = Rect::new(p.x - radius, p.y - radius, p.x + radius, p.y + radius);
        if selected {
            self.fill(rect, &CORNER_POINT_COLOR);
        } else {
            self.stroke(rect, &CORNER_POINT_COLOR, 1.0);
        }
    }

    fn draw_off_curve_point(&mut self, p: Point, selected: bool) {
        let radius = if selected {
            OFF_CURVE_SELECTED_RADIUS
        } else {
            OFF_CURVE_RADIUS
        };
        let brush = self.solid_brush(OFF_CURVE_POINT_COLOR);
        let circ = Circle::new(p, radius);
        if selected {
            self.fill(circ, &OFF_CURVE_POINT_COLOR);
        } else {
            self.stroke(circ, &brush, 1.0);
        }
    }

    fn draw_selection_rect(&mut self, rect: Rect) {
        let bg_brush = self.solid_brush(SELECTION_RECT_BG_COLOR);
        let stroke_brush = self.solid_brush(SELECTION_RECT_STROKE_COLOR);
        self.fill(rect, &SELECTION_RECT_BG_COLOR);
        self.stroke(rect, &SELECTION_RECT_STROKE_COLOR, 1.0);
    }

    fn draw_direction_indicator(&mut self, path: &BezPath) {
        let first_seg = match path.segments().next().as_ref().map(PathSeg::to_cubic) {
            None => return,
            Some(cubic) => cubic,
        };

        let tangent = tangent_vector(0.05, first_seg).normalize();
        let angle = Vec2::new(tangent.y, -tangent.x);
        let rotate = Affine::rotate(angle.atan2());
        let translate = Affine::translate(first_seg.p0.to_vec2() + tangent * 4.0);
        let mut arrow = make_arrow();
        arrow.apply_affine(rotate);
        arrow.apply_affine(translate);
        self.fill(arrow, &DIRECTION_ARROW_COLOR);
    }

    fn draw_component(&mut self, component: &Component, ufo: &Ufo) {
        if let Some(bez) = crate::data::get_bezier(&component.base, ufo, None) {
            self.fill(
                component.transform * Arc::try_unwrap(bez).unwrap(),
                &COMPONENT_FILL_COLOR,
            );
        }
    }
}

struct PointStyle {
    point: Point,
    style: Style,
    selected: bool,
}

enum Style {
    Open(PathSeg),
    Close(PathSeg),
    Corner,
    Smooth,
    Tangent,
    OffCurve,
}

struct PointIter<'a> {
    idx: usize,
    vport: ViewPort,
    path: &'a Path,
    bez: &'a BezPath,
    sels: &'a BTreeSet<EntityId>,
}

impl<'a> PointIter<'a> {
    fn new(
        path: &'a Path,
        vport: ViewPort,
        bez: &'a BezPath,
        sels: &'a BTreeSet<EntityId>,
    ) -> Self {
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
            PointType::OnCurve => Style::Corner,
            PointType::OffCurve => Style::OffCurve,
            PointType::OnCurveSmooth => {
                let prev = self.path.prev_point(this.id);
                let next = self.path.next_point(this.id);
                match (prev.is_on_curve(), next.is_on_curve()) {
                    (false, false) => Style::Smooth,
                    (true, false) | (false, true) => Style::Tangent,
                    _ => unreachable!(),
                }
            }
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

pub(crate) fn draw_session(
    ctx: &mut PaintCtx,
    space: ViewPort,
    canvas_size: Size,
    metrics: &FontMetrics,
    session: &EditSession,
    ufo: &Ufo,
) {
    ctx.clear(Color::WHITE);
    // kind of a hack? the glyph coordinate space has (0, 0) at the baseline with y up;
    // the piet coordinate space has (0, 0) in the top left, with y down.
    let affine = Affine::new([
        0.8,
        0.0,
        0.0,
        -0.8,
        canvas_size.width * 0.25,
        canvas_size.height * 0.75,
    ]);
    if let Err(e) = ctx.save() {
        log::warn!("failed to save context {:?}", e);
    }

    ctx.transform(affine);
    let mut draw_ctx = DrawCtx::new(&mut ctx.render_ctx, space);
    draw_ctx.draw_metrics(&session.glyph, metrics);
    draw_ctx.draw_guides(&session.guides, &session.selection);

    for path in session.paths.iter() {
        let bez = space.transform() * path.bezier().clone();
        draw_ctx.draw_path(&bez);
        draw_ctx.draw_control_point_lines(path);
        draw_ctx.draw_direction_indicator(&bez);

        for point in PointIter::new(path, space, &bez, &session.selection) {
            draw_ctx.draw_point(point)
        }

        if let Some(pt) = path.trailing() {
            if path.should_draw_trailing() {
                draw_ctx.draw_off_curve_point(pt.to_screen(space), true);
            }
        }
    }

    for component in session.components.iter() {
        draw_ctx.draw_component(component, ufo);
    }
    if let Err(e) = ctx.restore() {
        log::warn!("failed to restore context {:?}", e);
    }
}

//pub(crate) fn draw_paths(
//metrics: &FontMetrics,
//paths: &[Path],
//sels: &BTreeSet<EntityId>,
//guides: &[Guide],
////tool: &dyn Tool,
//space: ViewPort,
//canvas_size: Size,
//ctx: &mut PaintCtx,
//_mouse: Point,
//) {
//ctx.clear(Color::WHITE);
////if tool.name() == "preview" {
////draw_ctx.draw_filled_paths(paths);
////return;
////}

//let affine = Affine::new([
//0.8,
//0.0,
//0.0,
//-0.8,
//canvas_size.width * 0.75,
//canvas_size.height * 0.75,
//]);
//ctx.save();
//ctx.transform(affine);

//let mut draw_ctx = DrawCtx::new(&mut ctx.render_ctx, space);

//draw_ctx.draw_grid();
//draw_ctx.draw_guides(guides, sels);
//for path in paths {
////let bez = affine * (space.transform() * path.bezier().clone());
//let bez = (space.transform() * path.bezier().clone());
//draw_ctx.draw_path(&bez);
//draw_ctx.draw_control_point_lines(path);
//draw_ctx.draw_direction_indicator(&bez);

//for point in PointIter::new(path, space, &bez, sels) {
//draw_ctx.draw_point(point)
//}

//if let Some(pt) = path.trailing() {
//if path.should_draw_trailing() {
//draw_ctx.draw_off_curve_point(pt.to_screen(space), true);
//}
//}
//}
//if let Err(e) = ctx.restore() {
//log::warn!("failed to restore context {:?}", e);
//}

////if let Some(rect) = tool.selection_rect() {
////draw_ctx.draw_selection_rect(rect);
////}
//}

/// Return the tangent of the cubic bezier `cb`, at time `t`, as a vector
/// relative to the path's start point.
fn tangent_vector(t: f64, cb: CubicBez) -> Vec2 {
    debug_assert!(t >= 0.0 && t <= 1.0);
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
    //bez.move_to((-5., 0.));
    //bez.line_to((5., 0.));
    //bez.line_to((5., 11.));
    //bez.line_to((15., 11.));
    //bez.line_to((0., 32.));
    //bez.line_to((-15., 11.));
    //bez.line_to((-5., 11.));
    //bez.close_path();

    bez.move_to((0., 18.));
    bez.line_to((-12., 0.));
    bez.line_to((12., 0.));
    bez.close_path();
    bez
}

//fn intersects(line: Line, rect: Rect) -> bool {
//let linev = line.p1 - line.p0;
//let tl = rect.origin();
//let bl = Point::new(rect.x0, rect.y1);
//let tr = Point::new(rect.x1, rect.y0);
//let br = Point::new(rect.x1, rect.y1);
//let left = bl - tl;
//let top = tr - tl;
//let right = br - tr;
//let bottom = br - bl;
//let s: f64 = [left, top, right, bottom]
//.iter()
//.map(|v| linev.dot(*v).signum())
//.sum();

//s.abs() == 4.0
//}
