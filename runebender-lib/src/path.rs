use super::cubic_path::CubicPath;
use super::design_space::{DPoint, DVec2, ViewPort};
use super::point::{EntityId, PathPoint};
use super::point_list::{PathPoints, Segment};
use druid::kurbo::{Affine, BezPath, ParamCurveNearest, Point, Vec2};
use druid::Data;

use crate::selection::Selection;

#[derive(Debug, Clone, Data)]
pub enum Path {
    Cubic(CubicPath),
    Hyper(CubicPath),
}

impl Path {
    pub fn new(point: DPoint) -> Path {
        CubicPath::new(point).into()
    }

    pub fn from_norad(src: &norad::glyph::Contour) -> Path {
        CubicPath::from_norad(src).into()
    }

    pub fn from_raw_parts(
        id: EntityId,
        points: Vec<PathPoint>,
        trailing: Option<DPoint>,
        closed: bool,
    ) -> Self {
        CubicPath::from_raw_parts(id, points, trailing, closed).into()
    }

    pub fn to_norad(&self) -> norad::glyph::Contour {
        match self {
            Path::Cubic(path) => path.to_norad(),
            Path::Hyper(path) => path.to_norad(),
        }
    }

    fn path_points(&self) -> &PathPoints {
        match self {
            Path::Cubic(path) => path.path_points(),
            Path::Hyper(path) => path.path_points(),
        }
    }

    fn path_points_mut(&mut self) -> &mut PathPoints {
        match self {
            Path::Cubic(path) => path.path_points_mut(),
            Path::Hyper(path) => path.path_points_mut(),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.path_points().closed()
    }

    pub fn points(&self) -> &[PathPoint] {
        match self {
            Path::Cubic(path) => path.path_points().as_slice(),
            Path::Hyper(path) => path.path_points().as_slice(),
        }
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = Segment> {
        self.path_points().iter_segments()
    }

    pub(crate) fn paths_for_selection(&self, selection: &Selection) -> Vec<Path> {
        //FIXME: figure out how we're doing this bit
        match self {
            Path::Cubic(path) => path
                .paths_for_selection(selection)
                .into_iter()
                .map(Into::into)
                .collect(),
            Path::Hyper(path) => path
                .paths_for_selection(selection)
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }

    /// Scale the selection.
    ///
    /// `scale` is the new scale, as a ratio.
    /// `anchor` is a point on the screen that should remain fixed.
    pub(crate) fn scale_points(&mut self, points: &[EntityId], scale: Vec2, anchor: DPoint) {
        let scale_xform = Affine::scale_non_uniform(scale.x, scale.y);
        self.path_points_mut()
            .transform_points(points, scale_xform, anchor);
    }

    pub(crate) fn nudge_points(&mut self, points: &[EntityId], v: DVec2) {
        let affine = Affine::translate(v.to_raw());
        self.path_points_mut()
            .transform_points(points, affine, DPoint::ZERO);
    }

    pub(crate) fn nudge_all_points(&mut self, v: DVec2) {
        let affine = Affine::translate(v.to_raw());
        self.path_points_mut().transform_all(affine, DPoint::ZERO);
    }

    pub fn trailing(&self) -> Option<&DPoint> {
        self.path_points().trailing()
    }

    pub fn clear_trailing(&mut self) {
        self.path_points_mut().clear_trailing();
    }

    pub fn start_point(&self) -> &PathPoint {
        self.path_points().start_point()
    }

    pub fn delete_points(&mut self, points: &[EntityId]) {
        self.path_points_mut().delete_points(points);
    }

    pub fn close(&mut self) -> EntityId {
        self.path_points_mut().close()
    }

    pub fn reverse_contour(&mut self) {
        self.path_points_mut().reverse_contour()
    }

    /// Returns the distance from a point to any point on this path.
    pub fn screen_dist(&self, vport: ViewPort, point: Point) -> f64 {
        let screen_bez = vport.affine() * self.bezier();
        screen_bez
            .segments()
            .map(|seg| seg.nearest(point, 0.1).1)
            .fold(f64::MAX, |a, b| a.min(b))
    }

    /// Iterate the segments of this path where both the start and end
    /// of the segment are in the selection.
    pub(crate) fn segments_for_points<'a>(
        &'a self,
        points: &'a Selection,
    ) -> impl Iterator<Item = Segment> + 'a {
        self.path_points()
            .iter_segments()
            .filter(move |seg| points.contains(&seg.start_id()) && points.contains(&seg.end_id()))
    }

    pub(crate) fn split_segment_at_point(&mut self, seg: Segment, t: f64) {
        match self {
            Path::Cubic(path) => path.split_segment_at_point(seg, t),
            Path::Hyper(path) => path.split_segment_at_point(seg, t),
        }
    }

    /// Upgrade a line segment to a cubic bezier.
    pub(crate) fn upgrade_line_seg(&mut self, seg: Segment) {
        let cursor = self.path_points_mut().cursor(Some(seg.start_id()));
        let p0 = bail!(cursor.point());
        let p3 = bail!(cursor.peek_next(), "segment has correct number of points");
        let p1 = p0.point.lerp(p3.point, 1.0 / 3.0);
        let p2 = p0.point.lerp(p3.point, 2.0 / 3.0);
        let path = seg.start_id().parent();
        let new_seg = Segment::Cubic(
            *p0,
            PathPoint::off_curve(path, p1),
            PathPoint::off_curve(path, p2),
            *p3,
        );
        self.path_points_mut().replace_segment(seg, new_seg);
    }

    /// Upgrade a line segment to a cubic bezier.
    pub(crate) fn path_point_for_id(&self, point: EntityId) -> Option<PathPoint> {
        self.path_points().path_point_for_id(point)
    }

    pub(crate) fn prev_point(&self, point: EntityId) -> Option<PathPoint> {
        self.path_points().prev_point(point)
    }

    pub(crate) fn next_point(&self, point: EntityId) -> Option<PathPoint> {
        self.path_points().next_point(point)
    }

    pub fn id(&self) -> EntityId {
        self.path_points().id()
    }

    pub(crate) fn append_to_bezier(&self, bez: &mut BezPath) {
        match self {
            Path::Cubic(path) => path.append_to_bezier(bez),
            Path::Hyper(path) => path.append_to_bezier(bez),
        }
    }

    pub fn bezier(&self) -> BezPath {
        let mut bez = BezPath::new();
        self.append_to_bezier(&mut bez);
        bez
    }

    /// Add a new line segment at the end of the path.
    ///
    /// This is called when the user clicks with the pen tool.
    pub fn line_to(&mut self, point: DPoint) -> EntityId {
        self.path_points_mut().push_on_curve(point)
    }

    /// update an off-curve point in response to a drag.
    ///
    /// `is_locked` corresponds to the shift key being down.
    pub fn update_handle(&mut self, point: EntityId, dpt: DPoint, is_locked: bool) {
        self.path_points_mut().update_handle(point, dpt, is_locked)
    }

    /// Called when the user drags (modifying the bezier control points)
    /// after clicking.
    pub fn update_for_drag(&mut self, handle: DPoint) {
        if !self.path_points().last_segment_is_curve() {
            self.convert_last_to_curve(handle);
        } else {
            self.update_trailing(handle);
        }
    }

    /// Update the curve while the user drags a new control point.
    fn update_trailing(&mut self, handle: DPoint) {
        if let Some(last_point) = self.path_points().trailing_point_in_open_path().copied() {
            let mut cursor = self.path_points_mut().cursor(Some(last_point.id));
            assert!(last_point.is_on_curve());
            assert!(cursor
                .peek_prev()
                .map(PathPoint::is_off_curve)
                .unwrap_or(false));
            let on_curve_pt = last_point.point;
            let new_p = on_curve_pt - (handle - on_curve_pt);
            cursor.move_prev();
            cursor.point_mut().unwrap().point = new_p;
            self.path_points_mut().set_trailing(handle);
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
        if self.path_points().len() > 1 {
            let mut prev = self.path_points_mut().points_mut().pop().unwrap();
            assert!(prev.is_on_curve() && !prev.is_smooth());
            prev.toggle_type();
            let p1 = self.path_points().trailing().copied().unwrap_or_else(|| {
                self.path_points()
                    .as_slice()
                    .last()
                    .unwrap()
                    .point
                    .lerp(prev.point, 1.0 / 3.0)
            });
            let p2 = prev.point - (handle - prev.point);
            let pts = &[
                PathPoint::off_curve(self.id(), p1),
                PathPoint::off_curve(self.id(), p2),
                prev,
            ];
            self.path_points_mut().points_mut().extend(pts);
        }
        self.path_points_mut().set_trailing(handle);
    }

    /// Set one of a given point's axes to a new value; used when aligning a set
    /// of points.
    pub(crate) fn align_point(&mut self, point: EntityId, val: f64, set_x: bool) {
        let mut cursor = self.path_points_mut().cursor(Some(point));
        if let Some(pt) = cursor.point_mut() {
            if set_x {
                pt.point.x = val;
            } else {
                pt.point.y = val;
            }
        }
    }

    pub fn last_segment_is_curve(&self) -> bool {
        self.path_points().last_segment_is_curve()
    }

    /// Whether we should draw the 'trailing' control point & handle.
    /// We always do this for the first point, if it exists; otherwise
    /// we do it for curve points only.
    pub fn should_draw_trailing(&self) -> bool {
        self.path_points().len() == 1 || self.path_points().last_segment_is_curve()
    }

    // this will return `true` if passed an entity that does not actually
    // exist in this path but has the same parent id, such as for a point
    // that has been deleted. I don't think this is an issue in practice?
    pub(crate) fn contains(&self, id: &EntityId) -> bool {
        id.is_child_of(self.id())
    }

    pub fn toggle_on_curve_point_type(&mut self, id: EntityId) {
        let mut cursor = self.path_points_mut().cursor(Some(id));
        let has_ctrl = cursor
            .peek_prev()
            .map(PathPoint::is_off_curve)
            .or(cursor.peek_next().map(PathPoint::is_off_curve))
            .unwrap_or(false);
        if let Some(pt) = cursor.point_mut() {
            if pt.is_smooth() || pt.is_on_curve() && has_ctrl {
                pt.toggle_type();
            }
        }
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
impl From<CubicPath> for Path {
    fn from(src: CubicPath) -> Path {
        Path::Cubic(src)
    }
}
