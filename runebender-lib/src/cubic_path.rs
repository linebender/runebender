use super::design_space::DPoint;
use super::point::{EntityId, PathPoint, PointType};
use super::point_list::{PathPoints, Segment};
use druid::kurbo::{BezPath, PathEl};
use druid::Data;

use crate::selection::Selection;

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
            points: PathPoints::from_raw_parts(id, points, trailing, closed),
        }
    }

    /// For constructing a new path with points that are members of another
    /// path; we need to reset the parent ids.
    fn from_points_ignoring_parent(mut points: Vec<PathPoint>, closed: bool) -> Self {
        let new_parent = EntityId::next();
        for pt in &mut points {
            pt.id = EntityId::new_with_parent(new_parent);
        }
        CubicPath::from_raw_parts(new_parent, points, None, closed)
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

        let mut points: Vec<PathPoint> = src
            .points
            .iter()
            .map(|src_point| {
                //eprintln!("({}, {}): {:?}{}", src_point.x, src_point.y, src_point.typ, if src_point.smooth { " smooth" } else { "" });
                let point = DPoint::new(src_point.x.round() as f64, src_point.y.round() as f64);
                let typ = PointType::from_norad(&src_point.typ, src_point.smooth);
                let id = EntityId::new_with_parent(path_id);
                PathPoint { id, point, typ }
            })
            .collect();

        if closed {
            points.rotate_left(1);
        }

        CubicPath::from_raw_parts(path_id, points, None, closed)
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
        Contour::new(points, None, None)
    }

    pub(crate) fn path_points(&self) -> &PathPoints {
        &self.points
    }

    pub(crate) fn path_points_mut(&mut self) -> &mut PathPoints {
        &mut self.points
    }

    /// Given the selection, return the paths generated by the points in this
    /// path, that are in the selection.
    pub(crate) fn paths_for_selection(&self, selection: &Selection) -> Vec<CubicPath> {
        let (on_curve_count, selected_count) =
            self.points
                .iter_points()
                .fold((0, 0), |(total, selected), point| {
                    let sel = if selection.contains(&point.id) { 1 } else { 0 };
                    let on_curve = if point.is_on_curve() { 1 } else { 0 };
                    (total + on_curve, selected + sel)
                });

        // all our on-curves are selected, so just return us.
        if selected_count == on_curve_count {
            // sanity check
            assert!(on_curve_count > 0);
            return vec![self.clone()];
        // no selection, return nothing
        } else if selected_count == 0 {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut current: Option<Vec<PathPoint>> = None;

        for seg in self.points.iter_segments() {
            let has_start = selection.contains(&seg.start_id());
            let has_end = selection.contains(&seg.end_id());
            match (has_start, has_end) {
                (true, false) => {
                    let single_point_path = match current.take() {
                        None => true,
                        // our start point is the same as the end point;
                        // just append the current path
                        Some(pts) => {
                            let single_point = pts.last() != Some(&seg.start());
                            let path = CubicPath::from_points_ignoring_parent(pts, false);
                            result.push(path);
                            single_point
                        }
                    };
                    if single_point_path {
                        result.push(CubicPath::new(seg.start().point));
                    }
                }
                (true, true) => match current.take() {
                    None => current = Some(seg.points().collect()),
                    Some(mut pts) => {
                        if pts.last() != Some(&seg.start()) {
                            let path = CubicPath::from_points_ignoring_parent(pts, false);
                            result.push(path);
                            current = Some(seg.points().collect());
                        } else {
                            pts.extend(seg.points().skip(1));
                            current = Some(pts);
                        }
                    }
                },
                (false, true)
                    if seg.end() == self.points.last_on_curve_point() && self.points.closed() =>
                {
                    result.push(CubicPath::new(seg.end().point));
                }
                // we can can just continue, nothing to add
                (false, true) => (),
                (false, false) => (),
            }
        }

        if let Some(pts) = current.take() {
            let path = CubicPath::from_points_ignoring_parent(pts, false);
            result.push(path);
        }

        if result.len() < 2 {
            return result;
        }

        // cleanup: if the last selected segment joins the first, combine them
        if result.first().unwrap().points.start_point().point
            == result.last().unwrap().points.last_on_curve_point().point
            && self.points.closed()
        {
            let first = result.remove(0);
            let last = result.pop().unwrap();
            let points = last
                .points
                .iter_points()
                .chain(first.points.iter_points().skip(1))
                .collect();
            result.push(CubicPath::from_points_ignoring_parent(points, false));
        }

        result
    }

    pub(crate) fn append_to_bezier(&self, bez: &mut BezPath) {
        bez.move_to(self.points.start_point().point.to_raw());
        for segment in self.points.iter_segments() {
            match segment {
                Segment::Line(_, p1) => bez.line_to(p1.point.to_raw()),
                Segment::Cubic(_, p1, p2, p3) => {
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

    pub(crate) fn split_segment_at_point(&mut self, seg: Segment, t: f64) {
        let mut pre_seg = seg.subsegment(0.0..t);
        if let Segment::Cubic(_, _, _, p3) = &mut pre_seg {
            p3.typ = PointType::OnCurve { smooth: true };
        }
        let post_seg = seg.subsegment(t..1.0);
        self.points.split_segment(seg, pre_seg, post_seg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::Path;
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
        let path: Path = CubicPath::from_bezpath(rect.to_path(0.1)).unwrap().into();

        let mut seg_iter = path.iter_segments();
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
    }

    #[test]
    fn iter_line_sects() {
        let mut path = CubicPath::new(DPoint::new(0., 0.));
        path.path_points_mut().push_on_curve(DPoint::new(10., 10.));
        let path: Path = path.into();

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

        let mut iter = path.points.iter_segments().map(Segment::to_kurbo);
        assert_eq!(iter.next(), Some(Line::new((10., 10.), (0., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((0., 0.), (20., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((20., 0.), (10., 10.)).into()));
    }
}
