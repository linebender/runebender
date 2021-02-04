use std::collections::HashMap;
use std::sync::Arc;

use druid::kurbo::{BezPath, ParamCurve, ParamCurveNearest, PathEl, PathSeg, Point};
use druid::Data;
use spline::{Element, Segment as SplineSegment, SplineSpec};

use norad::glyph::{Contour, ContourPoint, PointType};
use norad::{Identifier, Plist};

use super::design_space::DPoint;
use super::point::{EntityId, PathPoint};
use super::point_list::{PathPoints, RawSegment};

pub(crate) static HYPERBEZ_LIB_VERSION_KEY: &str = "org.linebender.hyperbezier-version";
pub(crate) static HYPERBEZ_IS_POINT_KEY: &str = "org.linebender.hyperbezier-point";
pub(crate) static HYPERBEZ_CONTROL_POINTS: &str = "org.linebender.hyperbezier-control";

const HYPERBEZ_UFO_VERSION: u32 = 1;

#[derive(Debug, Data, Clone)]
pub struct HyperPath {
    points: PathPoints,
    #[data(ignore)]
    solver: SplineSpec,
    bezier: Arc<BezPath>,
}

#[derive(Debug, Clone)]
pub struct HyperSegment {
    pub(crate) path_seg: RawSegment,
    pub(crate) spline_seg: SplineSegment,
}

impl HyperPath {
    pub(crate) fn new(point: DPoint) -> HyperPath {
        HyperPath {
            points: PathPoints::new(point),
            solver: SplineSpec::new(),
            bezier: Arc::new(BezPath::new()),
        }
    }

    /// Construct a new CubicPath from the provided `PathPoints`.
    ///
    /// The caller is responsible for ensuring that the points have valid
    /// and unique identifiers, and are otherwise well-formed.
    pub(crate) fn from_path_points_unchecked(points: PathPoints) -> Self {
        let mut this = HyperPath {
            points,
            solver: SplineSpec::new(),
            bezier: Arc::new(BezPath::new()),
        };
        this.after_change();
        this
    }

    pub(crate) fn path_points(&self) -> &PathPoints {
        &self.points
    }

    pub(crate) fn path_points_mut(&mut self) -> &mut PathPoints {
        &mut self.points
    }

    pub(crate) fn hyper_segments(&self) -> Option<&[SplineSegment]> {
        self.solver.segments()
    }

    pub(crate) fn from_norad(src: &norad::glyph::Contour) -> Self {
        let mut points = Vec::new();
        let mut identifier_map = HashMap::new();
        let path_id = EntityId::next();
        if let Some(id) = src.identifier() {
            identifier_map.insert(path_id, id.clone());
        }

        let mut add_id = |n_pt: &ContourPoint, pp: &PathPoint| {
            if let Some(id) = n_pt.identifier() {
                identifier_map.insert(pp.id, id.clone());
            }
        };

        let mut closed = true;
        for point in src.points.iter() {
            if matches!(point.typ, norad::PointType::Move) {
                closed = false;
                let start = PathPoint::on_curve(
                    path_id,
                    DPoint::from_raw((point.x as f64, point.y as f64)),
                );
                add_id(point, &start);

                points.push(start);
                continue;
            }
            let lib = match point.lib() {
                Some(lib) => lib,
                None => continue,
            };
            if !lib
                .get(HYPERBEZ_IS_POINT_KEY)
                .and_then(|val| val.as_boolean())
                .unwrap_or(false)
            {
                continue;
            }

            if let Some(offcurves) = lib
                .get(HYPERBEZ_CONTROL_POINTS)
                .and_then(|val| val.as_dictionary())
            {
                let x1 = offcurves.get("x1").unwrap().as_real().unwrap();
                let x2 = offcurves.get("x2").unwrap().as_real().unwrap();
                let y1 = offcurves.get("y1").unwrap().as_real().unwrap();
                let y2 = offcurves.get("y2").unwrap().as_real().unwrap();
                let auto1 = offcurves.get("auto1").unwrap().as_boolean().unwrap();
                let auto2 = offcurves.get("auto2").unwrap().as_boolean().unwrap();
                let p1 = PathPoint::hyper_off_curve(path_id, DPoint::from_raw((x1, y1)), auto1);
                let p2 = PathPoint::hyper_off_curve(path_id, DPoint::from_raw((x2, y2)), auto2);
                points.push(p1);
                points.push(p2);
            }
            let mut end =
                PathPoint::on_curve(path_id, DPoint::from_raw((point.x as f64, point.y as f64)));
            add_id(point, &end);
            if point.smooth {
                end.toggle_type();
            }
            points.push(end);
        }
        let points =
            PathPoints::from_raw_parts(path_id, points, Some(identifier_map), None, closed);

        let mut this = Self {
            points,
            solver: SplineSpec::new(),
            bezier: Arc::new(BezPath::new()),
        };
        this.after_change();
        this
    }

    pub(crate) fn to_norad(&self) -> norad::glyph::Contour {
        fn norad_point(
            pt: Point,
            typ: PointType,
            smooth: bool,
            ident: Option<norad::Identifier>,
        ) -> ContourPoint {
            ContourPoint::new(pt.x as f32, pt.y as f32, typ, smooth, None, ident, None)
        }

        fn extend_from_bezpath(points: &mut Vec<ContourPoint>, path: &[PathEl]) {
            for el in path.iter() {
                match el {
                    PathEl::LineTo(pt) => {
                        points.push(norad_point(*pt, PointType::Curve, false, None))
                    }
                    PathEl::CurveTo(p1, p2, p3) => {
                        points.push(norad_point(*p1, PointType::OffCurve, false, None));
                        points.push(norad_point(*p2, PointType::OffCurve, false, None));
                        points.push(norad_point(*p3, PointType::Curve, false, None));
                    }
                    _ => (),
                }
            }
        }
        let mut points = Vec::new();
        if !self.path_points().closed() {
            let start = self.path_points().start_point();
            let ident = self.path_points().norad_id_for_id(start.id);
            points.push(norad_point(
                start.point.to_raw(),
                PointType::Move,
                false,
                ident,
            ));
        }

        if self.path_points().len() > 1 {
            for (segment, spline_segment) in self
                .path_points()
                .iter_segments()
                .zip(self.solver.segments().unwrap())
            {
                match segment {
                    RawSegment::Line(_, end) => {
                        extend_from_bezpath(&mut points, &[PathEl::LineTo(end.point.to_raw())]);
                        let mut last = points.last_mut().unwrap();
                        last.smooth = end.is_smooth();
                        let mut lib = Plist::new();
                        lib.insert(HYPERBEZ_IS_POINT_KEY.into(), true.into());
                        last.replace_lib(lib);
                        if let Some(ident) = self.path_points().norad_id_for_id(end.id) {
                            last.replace_identifier(ident);
                        }
                    }
                    RawSegment::Cubic(_, p1, p2, p3) => {
                        let mut segment_bez = BezPath::new();
                        spline_segment.render(&mut segment_bez);
                        extend_from_bezpath(&mut points, segment_bez.elements());
                        let last = points.last_mut().unwrap();
                        last.smooth = p3.is_smooth();
                        let mut offcurves = Plist::new();
                        offcurves.insert("x1".into(), p1.point.x.into());
                        offcurves.insert("x2".into(), p2.point.x.into());
                        offcurves.insert("y1".into(), p1.point.y.into());
                        offcurves.insert("y2".into(), p2.point.y.into());
                        offcurves.insert("auto1".into(), p1.is_auto().into());
                        offcurves.insert("auto2".into(), p2.is_auto().into());
                        let mut lib = Plist::new();
                        lib.insert(HYPERBEZ_CONTROL_POINTS.into(), offcurves.into());
                        lib.insert(HYPERBEZ_IS_POINT_KEY.into(), true.into());
                        last.replace_lib(lib);
                        if let Some(ident) = self.path_points().norad_id_for_id(p3.id) {
                            last.replace_identifier(ident);
                        }
                    }
                }
            }
        }

        let mut lib = Plist::new();
        lib.insert(HYPERBEZ_LIB_VERSION_KEY.into(), HYPERBEZ_UFO_VERSION.into());
        let ident = self
            .path_points()
            .norad_id_for_id(self.path_points().id())
            .unwrap_or_else(Identifier::from_uuidv4);
        Contour::new(points, Some(ident), Some(lib))
    }

    pub(crate) fn append_to_bezier(&self, bez: &mut BezPath) {
        bez.extend(self.bezier.elements().iter().cloned());
    }

    /// If smooth is true, adds a spline-to, else adds a line-to
    pub(crate) fn close(&mut self, smooth: bool) -> EntityId {
        assert!(!self.points.closed());
        let start = self.points.as_slice()[0].point;
        if smooth {
            self.spline_to(start, smooth);
            self.path_points_mut().close();
            self.path_points_mut().points_mut().pop();
            self.points.as_slice().last().unwrap().id
        } else {
            self.path_points_mut().close()
        }
    }

    pub(crate) fn spline_to(&mut self, p3: DPoint, smooth: bool) {
        let prev = self.points.as_slice().last().cloned().unwrap().point;
        let path_id = self.points.id();
        let p1 = prev.lerp(p3, 1.0 / 3.0);
        let p2 = prev.lerp(p3, 2.0 / 3.0);
        self.points
            .points_mut()
            .push(PathPoint::hyper_off_curve(path_id, p1, true));
        self.points
            .points_mut()
            .push(PathPoint::hyper_off_curve(path_id, p2, true));
        self.points
            .points_mut()
            .push(PathPoint::on_curve(path_id, p3));
        if smooth {
            self.points.points_mut().last_mut().unwrap().toggle_type();
        }
    }

    pub(crate) fn split_segment_at_point(&mut self, seg: HyperSegment, t: f64) {
        let pt = DPoint::from_raw(seg.eval(t));
        let path_id = seg.path_seg.start_id().parent();
        let (pre, post) = match &seg.path_seg {
            RawSegment::Line(p1, p2) => {
                let pt = PathPoint::on_curve(path_id, pt);
                (RawSegment::Line(*p1, pt), RawSegment::Line(pt, *p2))
            }
            RawSegment::Cubic(p1, p2, p3, p4) => {
                let pt = PathPoint::on_curve_smooth(path_id, pt);
                (
                    RawSegment::Cubic(
                        *p1,
                        *p2,
                        PathPoint::hyper_off_curve(path_id, p2.point, true),
                        pt,
                    ),
                    RawSegment::Cubic(
                        pt,
                        PathPoint::hyper_off_curve(path_id, p3.point, true),
                        *p3,
                        *p4,
                    ),
                )
            }
        };
        self.points.split_segment(seg.path_seg, pre, post);
    }

    pub(crate) fn after_change(&mut self) {
        if self.points.len() > 1 {
            self.rebuild_solver();
            self.rebuild_spline()
        } else {
            self.bezier = Arc::new(BezPath::default());
        }
    }

    /// rebuilds the solver from scratch, which is easier than trying to
    /// incrementally update it for some operations.
    fn rebuild_solver(&mut self) {
        let mut solver = SplineSpec::new();
        *solver.elements_mut() = self.iter_spline_elements().collect();
        if self.points.closed() {
            solver.close();
        }
        self.solver = solver;
    }

    /// Takes the current solver and updates the position of auto points based
    /// on their position in the resolved spline.
    fn rebuild_spline(&mut self) {
        let HyperPath { solver, points, .. } = self;
        let spline = solver.solve();

        // debugging some issues in the solver:
        if spline
            .segments()
            .iter()
            .any(|seg| !seg.p1.x.is_normal() || !seg.p2.x.is_normal())
        {
            std::mem::drop(spline);
            eprintln!("spline problems: {:?}", solver.elements());
            return;
        }
        let mut ix = if points.closed() { 0 } else { 1 };
        let points = points.points_mut();
        for segment in spline.segments() {
            if segment.is_line() {
                let p1 = segment.p0.lerp(segment.p3, 1.0 / 3.0);
                let p2 = segment.p0.lerp(segment.p3, 2.0 / 3.0);
                // I think we do no touchup, here?
                let is_on_curve = points.get(ix).unwrap().is_on_curve();
                if is_on_curve {
                    ix += 1;
                } else {
                    assert!(points.get(ix + 1).unwrap().is_off_curve());
                    points.get_mut(ix).unwrap().point = DPoint::from_raw(p1);
                    points.get_mut(ix + 1).unwrap().point = DPoint::from_raw(p2);
                    ix += 3;
                }
            } else {
                let p1 = points.get_mut(ix).unwrap();
                if p1.is_auto() {
                    p1.point = DPoint::from_raw(segment.p1);
                }
                let p2 = points.get_mut(ix + 1).unwrap();
                if p2.is_auto() {
                    p2.point = DPoint::from_raw(segment.p2);
                }
                ix += 3;
            }
        }

        self.bezier = Arc::new(spline.render());

        // and then we want to actually update our stored points:
    }

    fn iter_spline_elements(&self) -> impl Iterator<Item = Element> {
        let start = Some(self.points.start_point().point.to_raw());
        SplineElementIter {
            start,
            segments: self.points.iter_segments(),
        }
    }
}

impl HyperSegment {
    /// Find the nearest position in this segment to the provided point.
    ///
    /// # Hack
    ///
    /// If the param is calculated piecewise over the rendered bezier,
    /// it will be returned as a negative number, the integer component
    /// of which will correspond to the rendered segment and the fractional
    /// component will correspond to the param within that segment.
    pub(crate) fn nearest(&self, point: DPoint) -> (f64, f64) {
        let point = point.to_raw();
        const ACC: f64 = druid::kurbo::DEFAULT_ACCURACY;
        if self.spline_seg.is_line() {
            self.path_seg.to_kurbo().nearest(point, ACC)
        } else {
            self.kurbo_segments()
                .enumerate()
                .fold((-0.0, f64::MAX), |acc, (i, seg)| {
                    let (t, dist) = seg.nearest(point, ACC);
                    if acc.1 < dist {
                        acc
                    } else {
                        (-t - i as f64, dist)
                    }
                })
        }
    }

    /// The position in the segment corresponding to some param.
    ///
    /// # Hack
    ///
    /// If the provided param is positive, it will be treated as belonging
    /// in the range [0.0, 1.0] over the whole curve.
    ///
    /// If it is *negative* it will be interpreted piecewise over the rendered
    /// bezier segments. This behaviour will be removed once we have implemented
    /// thie ParamCurve traits for the hyperbezier.
    pub(crate) fn eval(&self, param: f64) -> Point {
        if self.spline_seg.is_line() {
            self.path_seg.to_kurbo().eval(param)
        } else {
            let (to_skip, seg_param) = if param.is_sign_negative() {
                (param.abs().trunc() as usize, param.abs().fract())
            } else {
                //TODO: when we get `eval` on the spline segment we can get rid of this
                let segment_count = self.spline_seg.hb.render_subdivisions();
                assert_eq!(self.kurbo_segments().count(), segment_count);
                let param_scale = 1.0 / segment_count as f64;
                let to_skip = (param / param_scale) as usize;
                let seg_param = param - (to_skip as f64 * param_scale);
                (to_skip, seg_param)
            };
            match self
                .kurbo_segments()
                .nth(to_skip)
                .map(|seg| seg.eval(seg_param))
            {
                Some(pt) => pt,
                None => {
                    let seg_count =
                        druid::kurbo::segments(self.spline_seg.render_elements()).count();
                    eprintln!(
                        "HyperBez::eval failed: skipped {} of {} segments",
                        to_skip, seg_count
                    );
                    self.path_seg.start().point.to_raw()
                }
            }
        }
    }

    pub(crate) fn kurbo_segments(&self) -> impl Iterator<Item = PathSeg> + '_ {
        let move_t = PathEl::MoveTo(self.path_seg.start().point.to_raw());
        let iter = std::iter::once(move_t).chain(self.spline_seg.render_elements());
        druid::kurbo::segments(iter)
    }
}

impl From<PathPoints> for HyperPath {
    fn from(points: PathPoints) -> HyperPath {
        HyperPath {
            points,
            solver: SplineSpec::new(),
            bezier: Arc::new(BezPath::new()),
        }
    }
}

struct SplineElementIter<I> {
    segments: I,
    start: Option<Point>,
}

impl<I> Iterator for SplineElementIter<I>
where
    I: Iterator<Item = RawSegment>,
{
    type Item = Element;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(start) = self.start.take() {
            return Some(Element::MoveTo(start));
        }
        match self.segments.next()? {
            RawSegment::Line(_, p1) => Some(Element::LineTo(p1.point.to_raw(), p1.is_smooth())),
            RawSegment::Cubic(_, p1, p2, p3) => {
                let p1 = if p1.is_auto() {
                    None
                } else {
                    Some(p1.point.to_raw())
                };
                let p2 = if p2.is_auto() {
                    None
                } else {
                    Some(p2.point.to_raw())
                };
                Some(Element::SplineTo(p1, p2, p3.point.to_raw(), p3.is_smooth()))
            }
        }
    }
}
