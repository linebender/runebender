use super::design_space::{DPoint, DVec2, ViewPort};
use super::point::{EntityId, PathPoint, PointType};
use super::point_list::{PathPoints, Segment};
use druid::kurbo::{Affine, BezPath, ParamCurveNearest, PathEl, Point, Vec2};
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
pub struct Path {
    points: PathPoints,
}

impl Path {
    pub fn new(point: DPoint) -> Path {
        Path {
            points: PathPoints::new(point),
        }
    }

    pub fn from_raw_parts(
        id: EntityId,
        points: Vec<PathPoint>,
        trailing: Option<DPoint>,
        closed: bool,
    ) -> Self {
        Path {
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
        Path::from_raw_parts(new_parent, points, None, closed)
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

        mark_tangent_handles(&mut points);

        if closed {
            points.rotate_left(1);
        }

        Ok(Self::from_raw_parts(path_id, points, None, closed))
    }

    pub fn from_norad(src: &norad::glyph::Contour) -> Path {
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

        Path::from_raw_parts(path_id, points, None, closed)
    }

    pub fn to_norad(&self) -> norad::glyph::Contour {
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
            //let smooth = p.typ == PointType::OnCurveSmooth;
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

    pub fn id(&self) -> EntityId {
        self.points.id()
    }

    // this will return `true` if passed an entity that does not actually
    // exist in this path but has the same parent id, such as for a point
    // that has been deleted. I don't think this is an issue in practice?
    pub(crate) fn contains(&self, id: &EntityId) -> bool {
        id.is_child_of(self.id())
    }

    pub fn points(&self) -> &[PathPoint] {
        self.points.as_slice()
    }

    pub fn trailing(&self) -> Option<&DPoint> {
        self.points.trailing()
    }

    pub fn clear_trailing(&mut self) {
        self.points.clear_trailing()
    }

    /// Whether we should draw the 'trailing' control point & handle.
    /// We always do this for the first point, if it exists; otherwise
    /// we do it for curve points only.
    pub fn should_draw_trailing(&self) -> bool {
        self.points.len() == 1 || self.points.last_segment_is_curve()
    }

    /// Returns the start point of the path.
    pub fn start_point(&self) -> &PathPoint {
        self.points.start_point()
    }

    fn end_point(&self) -> &PathPoint {
        self.points.end_point()
    }

    pub fn bezier(&self) -> BezPath {
        let mut bez = BezPath::new();
        self.append_to_bezier(&mut bez);
        bez
    }

    pub fn delete_points(&mut self, points: &[EntityId]) {
        self.points.delete_points(points);
    }

    /// Given the selection, return the paths generated by the points in this
    /// path, that are in the selection.
    pub(crate) fn paths_for_selection(&self, selection: &Selection) -> Vec<Path> {
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
                            let path = Path::from_points_ignoring_parent(pts, false);
                            result.push(path);
                            single_point
                        }
                    };
                    if single_point_path {
                        result.push(Path::new(seg.start().point));
                    }
                }
                (true, true) => match current.take() {
                    None => current = Some(seg.points().collect()),
                    Some(mut pts) => {
                        if pts.last() != Some(&seg.start()) {
                            let path = Path::from_points_ignoring_parent(pts, false);
                            result.push(path);
                            current = Some(seg.points().collect());
                        } else {
                            pts.extend(seg.points().skip(1));
                            current = Some(pts);
                        }
                    }
                },
                (false, true) if seg.end() == *self.end_point() && self.points.closed() => {
                    result.push(Path::new(seg.end().point));
                }
                // we can can just continue, nothing to add
                (false, true) => (),
                (false, false) => (),
            }
        }

        if let Some(pts) = current.take() {
            let path = Path::from_points_ignoring_parent(pts, false);
            result.push(path);
        }

        if result.len() < 2 {
            return result;
        }

        // cleanup: if the last selected segment joins the first, combine them
        if result.first().unwrap().start_point().point == result.last().unwrap().end_point().point
            && self.points.closed()
        {
            let first = result.remove(0);
            let last = result.pop().unwrap();
            let points = last
                .points
                .iter_points()
                .chain(first.points.iter_points().skip(1))
                .collect();
            result.push(Path::from_points_ignoring_parent(points, false));
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

    pub fn screen_dist(&self, vport: ViewPort, point: Point) -> f64 {
        let screen_bez = vport.affine() * self.bezier();
        screen_bez
            .segments()
            .map(|seg| seg.nearest(point, 0.1).1)
            .fold(f64::MAX, |a, b| a.min(b))
    }

    /// Appends a point. Called when the user clicks. This point is always a corner;
    /// if the user drags it will be converted to a curve then.
    ///
    /// Returns the id of the newly added point.
    pub fn append_point(&mut self, point: DPoint) -> EntityId {
        self.points.push_on_curve(point)
    }

    /// Scale the selection.
    ///
    /// `scale` is the new scale, as a ratio.
    /// `anchor` is a point on the screen that should remain fixed.
    pub(crate) fn scale_points(&mut self, points: &[EntityId], scale: Vec2, anchor: DPoint) {
        let scale_xform = Affine::scale_non_uniform(scale.x, scale.y);
        self.points.transform_points(points, scale_xform, anchor);
    }

    pub(crate) fn nudge_points(&mut self, points: &[EntityId], v: DVec2) {
        let affine = Affine::translate(v.to_raw());
        self.points.transform_points(points, affine, DPoint::ZERO);
    }

    pub(crate) fn nudge_all_points(&mut self, v: DVec2) {
        let affine = Affine::translate(v.to_raw());
        self.points.transform_all(affine, DPoint::ZERO);
    }

    /// update an off-curve point in response to a drag.
    ///
    /// `is_locked` corresponds to the shift key being down.
    pub fn update_handle(&mut self, point: EntityId, dpt: DPoint, is_locked: bool) {
        self.points.update_handle(point, dpt, is_locked)
    }

    /// Called when the user drags (modifying the bezier control points)
    /// after clicking.
    pub fn update_for_drag(&mut self, handle: DPoint) {
        if !self.points.last_segment_is_curve() {
            self.convert_last_to_curve(handle);
        } else {
            self.update_trailing(handle);
        }
    }

    pub fn last_segment_is_curve(&self) -> bool {
        self.points.last_segment_is_curve()
    }

    pub fn toggle_on_curve_point_type(&mut self, id: EntityId) {
        let mut cursor = self.points.cursor(Some(id)).unwrap();
        let has_ctrl = cursor
            .prev()
            .map(PathPoint::is_off_curve)
            .or(cursor.next().map(PathPoint::is_off_curve))
            .unwrap_or(false);
        if cursor.point().is_smooth() || cursor.point().is_on_curve() && has_ctrl {
            cursor.point_mut().toggle_type()
        }
    }

    /// If the user drags after mousedown, we convert the last point to a curve.
    ///
    /// TODO: So this doesn't quite match the Glyphs logic. There, the "trailing"
    /// state is not stored in the path, but seems to be transitory to the pen tool
    /// (selecting a different tool and back to pen seems to clear it). And, it
    /// seems to be used only when the active point is not smooth (which includes
    /// the start point). Otherwise, if the point before `prev` is off-curve, `p1`
    /// is synthesized from mirroring that point. Then the 1/3 lerp is used as a
    /// fallback.
    ///
    /// This is pretty similar to the current behavior though.
    fn convert_last_to_curve(&mut self, handle: DPoint) {
        if self.points.len() > 1 {
            let mut prev = self.points.points_mut().pop().unwrap();
            assert!(prev.is_on_curve() && !prev.is_smooth());
            prev.toggle_type();
            let p1 = self.points.trailing().copied().unwrap_or_else(|| {
                self.points
                    .as_slice()
                    .last()
                    .unwrap()
                    .point
                    .lerp(prev.point, 1.0 / 3.0)
            });
            let p2 = prev.point - (handle - prev.point);
            let pts = &[
                PathPoint::off_curve(self.points.id(), p1),
                PathPoint::off_curve(self.points.id(), p2),
                prev,
            ];
            self.points.points_mut().extend(pts);
        }
        self.points.set_trailing(handle);
    }

    /// Update the curve while the user drags a new control point.
    fn update_trailing(&mut self, handle: DPoint) {
        if self.points.len() > 1 {
            let mut cursor = self.points.cursor(None).unwrap();
            cursor.move_to_end();
            assert!(cursor.point().is_on_curve());
            assert!(cursor.prev().map(PathPoint::is_off_curve).unwrap_or(false));
            let on_curve_pt = cursor.point().point;
            let new_p = on_curve_pt - (handle - on_curve_pt);
            cursor.prev_mut().unwrap().point = new_p;
        }
        self.points.set_trailing(handle);
    }

    /// Set one of a given point's axes to a new value; used when aligning a set
    /// of points.
    pub(crate) fn align_point(&mut self, point: EntityId, val: f64, set_x: bool) {
        let mut cursor = self.points.cursor(Some(point)).unwrap();
        if set_x {
            cursor.point_mut().point.x = val;
        } else {
            cursor.point_mut().point.y = val;
        }
    }

    // in an open path, the first point is essentially a `move_to` command.
    // 'closing' the path means moving this point to the end of the list.
    pub fn close(&mut self) -> EntityId {
        self.points.close()
    }

    pub fn is_closed(&self) -> bool {
        self.points.closed()
    }

    pub fn reverse_contour(&mut self) {
        self.points.reverse_contour()
    }

    pub(crate) fn path_point_for_id(&self, point: EntityId) -> Option<PathPoint> {
        self.points.path_point_for_id(point)
    }

    pub(crate) fn prev_point(&self, point: EntityId) -> PathPoint {
        self.points.prev_point(point)
    }

    pub(crate) fn next_point(&self, point: EntityId) -> PathPoint {
        self.points.next_point(point)
    }

    /// Iterate the segments of this path where both the start and end
    /// of the segment are in the selection.
    pub(crate) fn segments_for_points<'a>(
        &'a self,
        points: &'a Selection,
    ) -> impl Iterator<Item = Segment> + 'a {
        self.points
            .iter_segments()
            .filter(move |seg| points.contains(&seg.start_id()) && points.contains(&seg.end_id()))
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = Segment> {
        self.points.iter_segments()
    }

    pub(crate) fn split_segment_at_point(&mut self, seg: Segment, t: f64) {
        let (existing_control_pts, points_to_insert) = match seg {
            Segment::Line(..) => (0, 1),
            Segment::Cubic(..) => (2, 5),
        };

        let mut pre_seg = seg.subsegment(0.0..t);
        if let Segment::Cubic(_, _, _, p3) = &mut pre_seg {
            p3.typ = PointType::OnCurve { smooth: true };
        }
        let post_seg = seg.subsegment(t..1.0);
        let insert_idx = self
            .points
            .cursor(Some(seg.start_id()))
            .and_then(|c| c.next_idx())
            .unwrap(); // the first point of a segment always has a next point

        let iter = pre_seg
            .into_iter()
            .skip(1)
            .chain(post_seg.into_iter().skip(1));
        let self_id = self.id();
        let points = self.points.points_mut();
        points.splice(
            insert_idx..insert_idx + existing_control_pts,
            iter.take(points_to_insert).map(|mut next_pt| {
                next_pt.reparent(self_id);
                next_pt
            }),
        );
    }

    /// Upgrade a line segment to a cubic bezier.
    pub(crate) fn upgrade_line_seg(&mut self, seg: Segment) {
        let cursor = bail!(
            self.points.cursor(Some(seg.start_id())),
            "segment expected to exist"
        );
        let p0 = cursor.point().point;
        let p3 = bail!(cursor.next(), "segment has correct number of points").point;
        let p1 = p0.lerp(p3, 1.0 / 3.0);
        let p2 = p0.lerp(p3, 2.0 / 3.0);
        let path = seg.start_id().parent();
        let insert_idx = cursor.next_idx().unwrap();
        self.points.points_mut().splice(
            insert_idx..insert_idx,
            [p1, p2].iter().map(|p| PathPoint::off_curve(path, *p)),
        );
    }
}

/// Walk the points in a list and mark those that look like tangent points
/// as being tangent points (OnCurveSmooth).
pub(crate) fn mark_tangent_handles(points: &mut [PathPoint]) {
    let len = points.len();

    // a closure for calculating indices
    let prev_and_next_idx = |idx: usize| {
        let prev = (idx + len).saturating_sub(1) % len;
        let next = (idx + 1) % len;
        (prev, next)
    };

    let mut idx = 0;
    while idx < len {
        let mut pt = points[idx];
        if pt.is_on_curve() {
            let (prev, next) = prev_and_next_idx(idx);
            let prev = points[prev];
            let next = points[next];
            if !prev.is_on_curve() && !next.is_on_curve() {
                let prev_angle = (prev.point.to_raw() - pt.point.to_raw()).atan2();
                let next_angle = (pt.point.to_raw() - next.point.to_raw()).atan2();
                let delta_angle = (prev_angle - next_angle).abs();
                // if the angle between the control points and the on-curve
                // point are within ~a degree of each other, consider it a tangent point.
                if delta_angle <= 0.018 {
                    pt.toggle_type()
                }
            }
        }
        //pt.id.parent = parent_id;
        points[idx] = pt;
        idx += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use druid::kurbo::{Line, PathSeg, Rect, Shape};

    #[test]
    fn from_bezpath() {
        let rect = Rect::from_origin_size((0., 0.), (10., 10.));
        let path = Path::from_bezpath(rect.to_path(0.1)).unwrap();
        assert!(path.is_closed());
        assert_eq!(path.points.len(), 4);
        assert_eq!(path.start_point().point.to_raw(), Point::ORIGIN);
    }

    #[test]
    fn iter_rect_segs() {
        let rect = Rect::new(0., 0., 10., 10.);
        let path = Path::from_bezpath(rect.to_path(0.1)).unwrap();

        let mut seg_iter = path.iter_segments();
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), Segment::Line(..)));
    }

    #[test]
    fn iter_line_sects() {
        let mut path = Path::new(DPoint::new(0., 0.));
        path.append_point(DPoint::new(10., 10.));

        let mut seg_iter = path.iter_segments();
        let seg = seg_iter.next().unwrap();
        let line = match seg.to_kurbo() {
            PathSeg::Line(line) => line,
            other => panic!("expected line found {:?}", other),
        };

        assert!(seg_iter.next().is_none());
        assert!(!path.points.closed());
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

        let path = Path::from_bezpath(bez).unwrap();

        assert!(path.points.closed());
        assert_eq!(path.points().len(), 3);

        let mut iter = path.points.iter_segments().map(Segment::to_kurbo);
        assert_eq!(iter.next(), Some(Line::new((10., 10.), (0., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((0., 0.), (20., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((20., 0.), (10., 10.)).into()));
    }
}
