use super::cubic_path::CubicPath;
use super::design_space::{DPoint, DVec2, ViewPort};
use super::hyper_path::{HyperPath, HyperSegment, HYPERBEZ_LIB_VERSION_KEY};
use super::point::{EntityId, PathPoint};
use super::point_list::{PathPoints, RawSegment};
use druid::kurbo::{
    Affine, BezPath, Line, LineIntersection, ParamCurve, ParamCurveNearest, PathSeg, Point, Vec2,
};
use druid::Data;

use crate::selection::Selection;

#[derive(Debug, Clone, Data)]
pub enum Path {
    Cubic(CubicPath),
    Hyper(HyperPath),
}

#[derive(Debug, Clone)]
pub enum Segment {
    Cubic(RawSegment),
    Hyper(HyperSegment),
}

impl Path {
    pub fn new(point: DPoint) -> Path {
        CubicPath::new(point).into()
    }

    pub fn new_hyper(point: DPoint) -> Path {
        Path::Hyper(HyperPath::new(point))
    }

    pub fn from_norad(src: &norad::glyph::Contour) -> Path {
        if src
            .lib()
            .map(|lib| lib.contains_key(HYPERBEZ_LIB_VERSION_KEY))
            .unwrap_or(false)
        {
            HyperPath::from_norad(src).into()
        } else {
            CubicPath::from_norad(src).into()
        }
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

    pub(crate) fn is_hyper(&self) -> bool {
        matches!(self, Path::Hyper(_))
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

    pub fn iter_segments(&self) -> impl Iterator<Item = Segment> + '_ {
        //NOTE:
        // in order to return `impl Iterator` we need to have a single concrete return type;
        // we can't branch on the type of the path and return a different iterator for each.
        // In order to make this work we have a slightly awkward implmenetation here.
        let hyper_segments = match self {
            Path::Cubic(_) => None,
            Path::Hyper(path) => path.hyper_segments(),
        };

        self.path_points()
            .iter_segments()
            .enumerate()
            .map(move |(i, path_seg)| {
                if let Some(spline_seg) = hyper_segments.and_then(|segs| segs.get(i).cloned()) {
                    Segment::Hyper(HyperSegment {
                        path_seg,
                        spline_seg,
                    })
                } else {
                    Segment::Cubic(path_seg)
                }
            })
    }

    pub(crate) fn paths_for_selection(&self, selection: &Selection) -> Vec<Path> {
        let paths = self.path_points().paths_for_selection(selection);
        paths
            .into_iter()
            .map(|pts| {
                if self.is_hyper() {
                    HyperPath::from(pts).into()
                } else {
                    CubicPath::from(pts).into()
                }
            })
            .collect()
    }

    /// Scale the selection.
    ///
    /// `scale` is the new scale, as a ratio.
    /// `anchor` is a point on the screen that should remain fixed.
    pub(crate) fn scale_points(&mut self, points: &[EntityId], scale: Vec2, anchor: DPoint) {
        let scale_xform = Affine::scale_non_uniform(scale.x, scale.y);
        self.path_points_mut()
            .transform_points(points, scale_xform, anchor);
        self.after_change();
    }

    pub(crate) fn nudge_points(&mut self, points: &[EntityId], v: DVec2) {
        let affine = Affine::translate(v.to_raw());
        let transformed = self
            .path_points_mut()
            .transform_points(points, affine, DPoint::ZERO);
        if self.is_hyper() {
            for point in points {
                // if this is an off-curve, and its neighbouring on-curve or
                // its tangent off-curve are not also nudged, we set it to non-auto
                if let Some((on_curve, other_handle)) =
                    self.path_points_mut().tangent_handle_opt(*point)
                {
                    let others_selected = transformed.contains(&on_curve)
                        && other_handle
                            .map(|p| transformed.contains(&p))
                            .unwrap_or(true);
                    if !others_selected {
                        self.path_points_mut().with_point_mut(*point, |pp| {
                            if pp.is_auto() {
                                pp.toggle_type()
                            }
                        })
                    }
                }
            }
        }
        self.after_change();
    }

    pub(crate) fn nudge_all_points(&mut self, v: DVec2) {
        let affine = Affine::translate(v.to_raw());
        self.path_points_mut().transform_all(affine, DPoint::ZERO);
        self.after_change();
    }

    pub fn trailing(&self) -> Option<DPoint> {
        self.path_points().trailing()
    }

    pub fn clear_trailing(&mut self) {
        self.path_points_mut().take_trailing();
    }

    pub fn start_point(&self) -> &PathPoint {
        self.path_points().start_point()
    }

    /// Delete a collection of points from this path.
    ///
    /// If only a single point was deleted from this path, we will return
    /// the id of a point in the path that is suitable for setting as the
    /// new selection.
    pub fn delete_points(&mut self, points: &[EntityId]) -> Option<EntityId> {
        let result = self.path_points_mut().delete_points(points);
        self.after_change();
        result.filter(|_| points.len() == 1)
    }

    pub fn close(&mut self, smooth: bool) -> EntityId {
        match self {
            Path::Cubic(path) => path.path_points_mut().close(),
            Path::Hyper(path) => {
                let id = path.close(smooth);
                self.after_change();
                id
            }
        }
    }

    pub fn reverse_contour(&mut self) {
        self.path_points_mut().reverse_contour();
        self.after_change();
    }

    fn after_change(&mut self) {
        if let Path::Hyper(path) = self {
            path.after_change();
        }
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
        self.iter_segments()
            .filter(move |seg| points.contains(&seg.start_id()) && points.contains(&seg.end_id()))
    }

    pub(crate) fn split_segment_at_point(&mut self, seg: Segment, t: f64) {
        match self {
            Path::Cubic(path) => {
                if let Segment::Cubic(seg) = seg {
                    path.split_segment_at_point(seg, t);
                }
            }
            Path::Hyper(path) => {
                if let Segment::Hyper(seg) = seg {
                    path.split_segment_at_point(seg, t);
                }
            }
        }
        self.after_change();
    }

    /// Upgrade a line segment to a cubic bezier.
    ///
    /// If 'use trailing' is true, this will use the trailing point to populate
    /// the first handle.
    pub(crate) fn upgrade_line_seg(&mut self, seg: &Segment, use_trailing: bool) {
        let cursor = self.path_points_mut().cursor(Some(seg.start_id()));
        let p0 = *bail!(cursor.point());
        let p3 = *bail!(cursor.peek_next(), "segment has correct number of points");
        let p1 = p0.point.lerp(p3.point, 1.0 / 3.0);
        let p1 = if use_trailing {
            self.path_points_mut().take_trailing().unwrap_or(p1)
        } else {
            p1
        };
        let p2 = p0.point.lerp(p3.point, 2.0 / 3.0);
        let path = seg.start_id().parent();
        let (p1, p2) = if self.is_hyper() {
            (
                PathPoint::hyper_off_curve(path, p1, true),
                PathPoint::hyper_off_curve(path, p2, true),
            )
        } else {
            (
                PathPoint::off_curve(path, p1),
                PathPoint::off_curve(path, p2),
            )
        };
        self.path_points_mut()
            .upgrade_line_seg(seg.start_id(), p1, p2);
        self.after_change();
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
    pub fn line_to(&mut self, point: DPoint, smooth: bool) -> EntityId {
        match self {
            Path::Cubic(path) => path.path_points_mut().push_on_curve(point),
            Path::Hyper(path) => {
                let id = if !smooth {
                    path.path_points_mut().push_on_curve(point)
                } else {
                    path.spline_to(point, smooth);
                    path.path_points().last_on_curve_point().id
                };
                self.after_change();
                id
            }
        }
    }

    /// Update the curve while the user drags a new control point.
    pub(crate) fn update_trailing(&mut self, point: EntityId, handle: DPoint) {
        self.path_points_mut().set_trailing(handle);
        if self.points().len() > 1 {
            let is_hyper = self.is_hyper();
            let mut cursor = self.path_points_mut().cursor(Some(point));
            assert!(cursor
                .peek_prev()
                .map(PathPoint::is_off_curve)
                .unwrap_or(false));
            let on_curve_pt = bail!(cursor.point()).point;
            let new_p = on_curve_pt - (handle - on_curve_pt);
            cursor.move_prev();
            if let Some(prev) = cursor.point_mut() {
                prev.point = new_p;
                if prev.is_auto() && is_hyper {
                    prev.toggle_type();
                }
            }
            self.after_change();
        }
    }

    /// Set one of a given point's axes to a new value; used when aligning a set
    /// of points.
    pub(crate) fn align_point(&mut self, point: EntityId, val: f64, set_x: bool) {
        self.path_points_mut().with_point_mut(point, |pp| {
            if set_x {
                pp.point.x = val
            } else {
                pp.point.y = val
            }
        });
        self.after_change();
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

    pub fn toggle_point_type(&mut self, id: EntityId) {
        self.path_points_mut()
            .with_point_mut(id, |pp| pp.toggle_type());
        self.after_change();
    }

    /// Only toggles the point type if it is on-curve, and only makes it
    /// smooth if it has a neighbouring off-curve
    pub fn toggle_on_curve_point_type(&mut self, id: EntityId) {
        let mut cursor = self.path_points_mut().cursor(Some(id));
        let has_ctrl = cursor
            .peek_prev()
            .map(PathPoint::is_off_curve)
            .or_else(|| cursor.peek_next().map(PathPoint::is_off_curve))
            .unwrap_or(false);
        if let Some(pt) = cursor.point_mut() {
            if pt.is_smooth() || pt.is_on_curve() && has_ctrl {
                pt.toggle_type();
            }
        }
        self.after_change();
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

impl Segment {
    pub(crate) fn start(&self) -> PathPoint {
        match self {
            Self::Cubic(seg) => seg.start(),
            Self::Hyper(seg) => seg.path_seg.start(),
        }
    }

    pub(crate) fn end(&self) -> PathPoint {
        match self {
            Self::Cubic(seg) => seg.end(),
            Self::Hyper(seg) => seg.path_seg.end(),
        }
    }

    pub(crate) fn start_id(&self) -> EntityId {
        self.start().id
    }

    pub(crate) fn end_id(&self) -> EntityId {
        self.end().id
    }

    pub(crate) fn raw_segment(&self) -> &RawSegment {
        match self {
            Self::Cubic(seg) => seg,
            Self::Hyper(seg) => &seg.path_seg,
        }
    }

    pub(crate) fn is_line(&self) -> bool {
        matches!(self.raw_segment(), RawSegment::Line(..))
    }

    pub(crate) fn nearest(&self, point: DPoint) -> (f64, f64) {
        match self {
            Self::Cubic(seg) => seg
                .to_kurbo()
                .nearest(point.to_raw(), druid::kurbo::DEFAULT_ACCURACY),
            Self::Hyper(seg) => seg.nearest(point),
        }
    }

    /// This returns a point and not a DPoint because the location may
    /// not fall on the grid.
    pub(crate) fn nearest_point(&self, point: DPoint) -> Point {
        let (t, _) = self.nearest(point);
        self.eval(t)
    }

    pub(crate) fn intersect_line(&self, line: Line) -> Vec<LineIntersection> {
        match self {
            Self::Cubic(seg) => seg.to_kurbo().intersect_line(line).into_iter().collect(),
            Self::Hyper(seg) if self.is_line() => seg
                .path_seg
                .to_kurbo()
                .intersect_line(line)
                .into_iter()
                .collect(),
            Self::Hyper(seg) => seg
                .kurbo_segments()
                .enumerate()
                .flat_map(|(i, kseg)| {
                    let hits = kseg.intersect_line(line);
                    hits.into_iter().map(move |mut hit| {
                        hit.segment_t = -hit.segment_t - i as f64;
                        hit
                    })
                })
                .collect(),
        }
    }

    /// The position on the segment corresponding to some param,
    /// generally in the range [0.0, 1.0].
    pub(crate) fn eval(&self, param: f64) -> Point {
        match self {
            Self::Cubic(seg) => seg.to_kurbo().eval(param),
            Self::Hyper(seg) => seg.eval(param),
        }
    }

    pub(crate) fn kurbo_segments(&self) -> impl Iterator<Item = PathSeg> + '_ {
        let (one_iter, two_iter) = match self {
            Self::Cubic(seg) => (Some(seg.to_kurbo()), None),
            Self::Hyper(seg) => (None, Some(seg.kurbo_segments())),
        };
        one_iter.into_iter().chain(two_iter.into_iter().flatten())
    }

    //pub(crate) fn subsegment(self, range: Range<f64>) -> Self {
    //match &self {
    //Self::Cubic(seg) => Self::Cubic(seg.subsegment(range)),
    //Self::Hyper(_seg) => {
    //eprintln!("HyperBezier subsegment unimplemented");
    //self
    //}
    //}
    //}
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum SerializePath {
    Cubic(PathPoints),
    Hyper(PathPoints),
}

use serde::{Deserialize, Deserializer, Serialize, Serializer};

impl Serialize for Path {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let to_ser = match self {
            Path::Cubic(path) => SerializePath::Cubic(path.path_points().to_owned()),
            Path::Hyper(path) => SerializePath::Hyper(path.path_points().to_owned()),
        };
        to_ser.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Path {
    fn deserialize<D>(deserializer: D) -> Result<Path, D::Error>
    where
        D: Deserializer<'de>,
    {
        let path: SerializePath = Deserialize::deserialize(deserializer)?;
        match path {
            SerializePath::Cubic(path) => Ok(CubicPath::from_path_points_unchecked(path).into()),
            SerializePath::Hyper(path) => Ok(HyperPath::from_path_points_unchecked(path).into()),
        }
    }
}

impl From<CubicPath> for Path {
    fn from(src: CubicPath) -> Path {
        Path::Cubic(src)
    }
}

impl From<HyperPath> for Path {
    fn from(src: HyperPath) -> Path {
        Path::Hyper(src)
    }
}

impl From<RawSegment> for Segment {
    fn from(src: RawSegment) -> Segment {
        Segment::Cubic(src)
    }
}

impl From<HyperSegment> for Segment {
    fn from(src: HyperSegment) -> Segment {
        Segment::Hyper(src)
    }
}
