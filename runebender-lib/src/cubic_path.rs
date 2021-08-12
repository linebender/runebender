use std::collections::HashMap;

use super::design_space::DPoint;
use super::point::{EntityId, PathPoint, PointType};
use super::point_list::{PathPoints, RawSegment};
use druid::kurbo::{BezPath, PathEl};
use druid::Data;

/// A single bezier path.
///
/// This does not contain subpaths, but a glyph can contain multiple paths.
/// UFO calls this a [contour][].
///
/// # Notes
///
/// As UFO does not support the idea of a 'start point' for closed glyphs,
/// and defines the points in a path as a cycle, we adopt the convention that
/// the 'first point' in a closed path is always the last point in the vec.
///
/// A path that is 'open' must both begin and end with on-curve points.
///
/// [contour]: https://unifiedfontobject.org/versions/ufo3/glyphs/glif/#contour
#[derive(Debug, Data, Clone)]
pub struct CubicPath {
    points: PathPoints,
}

impl CubicPath {
    pub(crate) fn new(point: DPoint) -> CubicPath {
        CubicPath {
            points: PathPoints::new(point),
        }
    }

    pub(crate) fn from_raw_parts(
        id: EntityId,
        points: Vec<PathPoint>,
        trailing: Option<DPoint>,
        closed: bool,
    ) -> Self {
        CubicPath {
            points: PathPoints::from_raw_parts(id, points, None, trailing, closed),
        }
    }

    /// Construct a new CubicPath from the provided `PathPoints`.
    ///
    /// The caller is responsible for ensuring that the points have valid
    /// and unique identifiers, and are otherwise well-formed.
    pub(crate) fn from_path_points_unchecked(points: PathPoints) -> Self {
        CubicPath { points }
    }

    /// Attempt to create a `Path` from a BezPath.
    ///
    /// - on the first 'segment' of the bezier will be used.
    /// - we don't currently support quadratics.
    pub(crate) fn from_bezpath(
        path: impl IntoIterator<Item = PathEl>,
    ) -> Result<Self, &'static str> {
        let path_id = EntityId::next();
        let mut els = path.into_iter();
        let mut points = Vec::new();
        let mut explicit_close = false;

        let start_point = match els.next() {
            Some(PathEl::MoveTo(pt)) => pt,
            _ => return Err("missing initial moveto"),
        };

        points.push(PathPoint::on_curve(path_id, DPoint::from_raw(start_point)));

        for el in els {
            match el {
                // we only take the first path segment
                PathEl::MoveTo(_) => break,
                PathEl::LineTo(pt) => {
                    points.push(PathPoint::on_curve(path_id, DPoint::from_raw(pt)));
                }
                PathEl::CurveTo(p0, p1, p2) => {
                    points.push(PathPoint::off_curve(path_id, DPoint::from_raw(p0)));
                    points.push(PathPoint::off_curve(path_id, DPoint::from_raw(p1)));
                    points.push(PathPoint::on_curve(path_id, DPoint::from_raw(p2)));
                }
                PathEl::QuadTo(..) => return Err("quads not currently supported"),
                PathEl::ClosePath => {
                    explicit_close = true;
                    break;
                }
            }
        }

        let closed = if points.len() > 1
            && points.first().map(|p| p.point) == points.last().map(|p| p.point)
        {
            points.pop();
            true
        } else {
            explicit_close
        };

        crate::path::mark_tangent_handles(&mut points);

        if closed {
            points.rotate_left(1);
        }

        Ok(Self::from_raw_parts(path_id, points, None, closed))
    }

    pub(crate) fn from_norad(src: &norad::glyph::Contour) -> CubicPath {
        use norad::glyph::PointType as NoradPType;
        assert!(
            !src.points.is_empty(),
            "non empty points list should already be checked"
        );
        let closed = !matches!(src.points[0].typ, NoradPType::Move);

        let path_id = EntityId::next();
        let mut points = Vec::new();
        let mut idents = HashMap::new();

        if let Some(id) = src.identifier() {
            idents.insert(path_id, id.clone());
        }

        for n_pt in &src.points {
            let point = DPoint::new(n_pt.x.round() as f64, n_pt.y.round() as f64);
            let typ = PointType::from_norad(n_pt);
            let id = EntityId::new_with_parent(path_id);
            if let Some(ident) = n_pt.identifier() {
                idents.insert(id, ident.to_owned());
            }
            points.push(PathPoint { id, point, typ });
        }

        //FIXME: this looks confused and is probably a bug? we should normalize
        //points we get from norad, but not arbitrarily change the order?
        if closed {
            points.rotate_left(1);
        }

        let points = PathPoints::from_raw_parts(path_id, points, Some(idents), None, closed);
        CubicPath { points }
    }

    pub(crate) fn to_norad(&self) -> norad::glyph::Contour {
        use norad::glyph::{Contour, ContourPoint, PointType as NoradPType};
        let mut points = Vec::new();
        let mut prev_off_curve = self
            .points
            .as_slice()
            .last()
            .map(|p| !p.is_on_curve())
            .unwrap_or(false);
        for p in self.points.as_slice() {
            let needs_move = points.is_empty() && !self.points.closed();
            let (typ, smooth) = match p.typ {
                PointType::OnCurve { smooth } if needs_move => (NoradPType::Move, smooth),
                PointType::OffCurve { .. } => (NoradPType::OffCurve, false),
                PointType::OnCurve { smooth } if prev_off_curve => (NoradPType::Curve, smooth),
                PointType::OnCurve { smooth } => (NoradPType::Line, smooth),
            };
            let x = p.point.x as f32;
            let y = p.point.y as f32;
            let npoint = ContourPoint::new(x, y, typ, smooth, None, None, None);
            points.push(npoint);
            prev_off_curve = p.is_off_curve();
        }

        if self.points.closed() {
            points.rotate_right(1);
        }
        let ident = self.path_points().norad_id_for_id(self.path_points().id());
        Contour::new(points, ident, None)
    }

    pub(crate) fn iter_segments(&self) -> impl Iterator<Item = RawSegment> {
        self.path_points().iter_segments()
    }

    pub(crate) fn path_points(&self) -> &PathPoints {
        &self.points
    }

    pub(crate) fn path_points_mut(&mut self) -> &mut PathPoints {
        &mut self.points
    }

    pub(crate) fn append_to_bezier(&self, bez: &mut BezPath) {
        bez.move_to(self.points.start_point().point.to_raw());
        for segment in self.points.iter_segments() {
            match segment {
                RawSegment::Line(_, p1) => bez.line_to(p1.point.to_raw()),
                RawSegment::Cubic(_, p1, p2, p3) => {
                    bez.curve_to(p1.to_kurbo(), p2.to_kurbo(), p3.to_kurbo())
                }
            }
        }
        if self.points.closed() {
            bez.close_path();
        }
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.points.closed()
    }

    pub(crate) fn split_segment_at_point(&mut self, seg: RawSegment, t: f64) {
        let mut pre_seg = seg.subsegment(0.0..t);
        if let RawSegment::Cubic(_, _, _, p3) = &mut pre_seg {
            p3.typ = PointType::OnCurve { smooth: true };
        }
        let post_seg = seg.subsegment(t..1.0);
        self.points.split_segment(seg, pre_seg, post_seg);
    }
}

impl From<PathPoints> for CubicPath {
    fn from(points: PathPoints) -> CubicPath {
        CubicPath { points }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use druid::kurbo::{Line, PathSeg, Point, Rect, Shape};

    #[test]
    fn from_bezpath() {
        let rect = Rect::from_origin_size((0., 0.), (10., 10.));
        let path = CubicPath::from_bezpath(rect.to_path(0.1)).unwrap();
        assert!(path.is_closed());
        assert_eq!(path.points.len(), 4);
        assert_eq!(path.points.start_point().point.to_raw(), Point::ORIGIN);
    }

    #[test]
    fn iter_rect_segs() {
        let rect = Rect::new(0., 0., 10., 10.);
        let path = CubicPath::from_bezpath(rect.to_path(0.1)).unwrap();

        let mut seg_iter = path.iter_segments();
        assert!(matches!(seg_iter.next().unwrap(), RawSegment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), RawSegment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), RawSegment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), RawSegment::Line(..)));
    }

    #[test]
    fn iter_line_sects() {
        let mut path = CubicPath::new(DPoint::new(0., 0.));
        path.path_points_mut().push_on_curve(DPoint::new(10., 10.));

        let mut seg_iter = path.iter_segments();
        let seg = seg_iter.next().unwrap();
        let line = match seg.to_kurbo() {
            PathSeg::Line(line) => line,
            other => panic!("expected line found {:?}", other),
        };

        assert!(seg_iter.next().is_none());
        assert!(!path.is_closed());
        assert_eq!(line.p0, Point::ORIGIN);
        assert_eq!(line.p1, Point::new(10., 10.));
    }

    #[test]
    fn iter_triangle_sects() {
        let mut bez = BezPath::new();
        bez.move_to((10., 10.));
        bez.line_to((0., 0.));
        bez.line_to((20., 0.));
        bez.close_path();

        let path = CubicPath::from_bezpath(bez).unwrap();

        assert!(path.points.closed());
        assert_eq!(path.points.len(), 3);

        let mut iter = path.points.iter_segments().map(RawSegment::to_kurbo);
        assert_eq!(iter.next(), Some(Line::new((10., 10.), (0., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((0., 0.), (20., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((20., 0.), (10., 10.)).into()));
    }
}
