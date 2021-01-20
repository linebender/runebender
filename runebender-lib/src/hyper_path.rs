use std::sync::Arc;

use super::design_space::DPoint;
use super::point::{EntityId, PathPoint, PointType};
use super::point_list::{PathPoints, Segment};
use druid::kurbo::{BezPath, PathEl, Point};
use druid::Data;
use spline::{Element, SplineSpec};

use crate::selection::Selection;

#[derive(Debug, Data, Clone)]
pub struct HyperPath {
    points: PathPoints,
    #[data(ignore)]
    solver: SplineSpec,
    bezier: Arc<BezPath>,
}

impl HyperPath {
    pub(crate) fn new(point: DPoint) -> HyperPath {
        HyperPath {
            points: PathPoints::new(point),
            solver: SplineSpec::new(),
            bezier: Arc::new(BezPath::new()),
        }
    }

    pub(crate) fn path_points(&self) -> &PathPoints {
        &self.points
    }

    pub(crate) fn path_points_mut(&mut self) -> &mut PathPoints {
        &mut self.points
    }

    pub(crate) fn to_norad(&self) -> norad::glyph::Contour {
        use norad::glyph::{Contour, ContourPoint, PointType as NoradPType};
        fn norad_point(pt: DPoint, typ: NoradPType, smooth: bool) -> ContourPoint {
            ContourPoint::new(pt.x as f32, pt.y as f32, typ, smooth, None, None, None)
        }
        let mut points = Vec::new();
        if self.path_points().closed() {
            let start = self.path_points().start_point().point;
            points.push(norad_point(start, NoradPType::Move, false));
        }

        for segment in self.path_points().iter_segments() {
            match segment {
                Segment::Line(_, end) => {
                    points.push(norad_point(end.point, NoradPType::Line, end.is_smooth()));
                }
                Segment::Cubic(_, p1, p2, p3) => {
                    points.push(norad_point(p1.point, NoradPType::OffCurve, false));
                    points.push(norad_point(p2.point, NoradPType::OffCurve, false));
                    points.push(norad_point(p3.point, NoradPType::Curve, p3.is_smooth()));
                }
            }
        }
        if self.points.closed() {
            points.rotate_right(1);
        }
        Contour::new(points, None, None)
    }

    pub(crate) fn append_to_bezier(&self, bez: &mut BezPath) {
        bez.extend(self.bezier.elements().iter().cloned());
    }

    pub(crate) fn spline_to(&mut self, p3: DPoint, smooth: bool) {
        let prev = self.points.as_slice().last().cloned().unwrap().point;
        let path_id = self.points.id();
        let p1 = prev.lerp(p3, 1.0 / 3.0);
        let p2 = prev.lerp(p3, 2.0 / 3.0);
        self.points.points_mut().push(PathPoint::auto(path_id, p1));
        self.points.points_mut().push(PathPoint::auto(path_id, p2));
        self.points
            .points_mut()
            .push(PathPoint::on_curve(path_id, p3));
        if smooth {
            self.points.points_mut().last_mut().unwrap().toggle_type();
        }
    }

    pub(crate) fn convert_last_to_curve(&mut self, _handle: DPoint) {
        if let Some(prev_point) = self.points.points_mut().pop() {
            assert!(self.points.trailing().is_none());
            self.spline_to(prev_point.point, true);
        }
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
        let start = if !self.points.closed() {
            Some(self.points.start_point().point.to_raw())
        } else {
            None
        };
        SplineElementIter {
            start,
            segments: self.points.iter_segments(),
        }
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
    //ix: usize,
}

impl<I> Iterator for SplineElementIter<I>
where
    I: Iterator<Item = Segment>,
{
    type Item = Element;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(start) = self.start.take() {
            return Some(Element::MoveTo(start));
        }
        match self.segments.next()? {
            Segment::Line(_, p1) => Some(Element::LineTo(p1.point.to_raw(), p1.is_smooth())),
            Segment::Cubic(_, p1, p2, p3) => {
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
