use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use super::design_space::{DPoint, DVec2, ViewPort};
use druid::kurbo::{BezPath, CubicBez, Line, ParamCurve, PathSeg as KurboPathSeg, Point, Vec2};
use druid::Data;

#[cfg(test)]
use druid::kurbo::PathEl;

const RESERVED_ID_COUNT: usize = 5;
const GUIDE_TYPE_ID: usize = 1;

/// We give paths & points unique integer identifiers.
pub fn next_id() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT_ID: AtomicUsize = AtomicUsize::new(RESERVED_ID_COUNT);
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, Data, PartialEq, PartialOrd, Hash, Eq, Ord)]
pub struct EntityId {
    pub(crate) parent: usize,
    pub(crate) point: usize,
}

#[derive(Debug, Clone, Copy, Data, PartialEq)]
pub enum PointType {
    OnCurve,
    OnCurveSmooth,
    OffCurve,
}

#[derive(Debug, Clone, Copy, Data, PartialEq)]
pub struct PathPoint {
    pub id: EntityId,
    pub point: DPoint,
    pub typ: PointType,
}

#[derive(Debug, Data, Clone)]
pub struct Path {
    id: usize,
    points: Arc<Vec<PathPoint>>,
    trailing: Option<DPoint>,
    closed: bool,
}

/// Questionable.
///
/// We use this in the knife tool. Really all we need is the kurbo PathSeg and
/// the start/end EntityIds, but we needed more at an earlier iteration.
#[derive(Clone, Copy, PartialEq)]
pub enum PathSeg {
    Line(PathPoint, PathPoint),
    Cubic(PathPoint, PathPoint, PathPoint, PathPoint),
}

struct PathSegIter {
    points: Arc<Vec<PathPoint>>,
    prev_pt: PathPoint,
    idx: usize,
}

pub struct SegPointIter {
    seg: PathSeg,
    idx: usize,
}

impl EntityId {
    pub fn new_with_parent(parent: usize) -> Self {
        EntityId {
            parent,
            point: next_id(),
        }
    }

    #[inline]
    pub fn new_for_guide() -> Self {
        EntityId::new_with_parent(GUIDE_TYPE_ID)
    }

    pub fn is_guide(self) -> bool {
        self.parent == GUIDE_TYPE_ID
    }
}

impl std::cmp::PartialEq<Path> for EntityId {
    fn eq(&self, other: &Path) -> bool {
        self.parent == other.id
    }
}

impl std::cmp::PartialEq<EntityId> for Path {
    fn eq(&self, other: &EntityId) -> bool {
        self.id == other.parent
    }
}

impl PointType {
    pub fn is_on_curve(self) -> bool {
        match self {
            PointType::OnCurve | PointType::OnCurveSmooth => true,
            PointType::OffCurve => false,
        }
    }
}

impl PathPoint {
    pub fn off_curve(path: usize, point: DPoint) -> PathPoint {
        let id = EntityId {
            parent: path,
            point: next_id(),
        };
        PathPoint {
            id,
            point,
            typ: PointType::OffCurve,
        }
    }

    pub fn on_curve(path: usize, point: DPoint) -> PathPoint {
        let id = EntityId {
            parent: path,
            point: next_id(),
        };
        PathPoint {
            id,
            point,
            typ: PointType::OnCurve,
        }
    }

    pub fn is_on_curve(&self) -> bool {
        self.typ.is_on_curve()
    }

    /// The distance, in screen space, from this `PathPoint` to `point`, a point
    /// in screen space.
    pub fn screen_dist(&self, vport: ViewPort, point: Point) -> f64 {
        self.point.to_screen(vport).distance(point)
    }

    /// Convert this point to point in screen space.
    pub fn to_screen(&self, vport: ViewPort) -> Point {
        self.point.to_screen(vport)
    }
}

impl Path {
    pub fn new(point: DPoint) -> Path {
        let id = next_id();
        let start = PathPoint::on_curve(id, point);

        Path {
            id,
            points: Arc::new(vec![start]),
            closed: false,
            trailing: None,
        }
    }

    pub fn from_raw_parts(
        id: usize,
        points: Vec<PathPoint>,
        trailing: Option<DPoint>,
        closed: bool,
    ) -> Self {
        assert!(!points.is_empty(), "path may not be empty");
        assert!(points.iter().all(|pt| pt.id.parent == id), "{:#?}", points);
        Path {
            id,
            points: Arc::new(points),
            trailing,
            closed,
        }
    }

    /// Attempt to create a `Path` from a BezPath.
    ///
    /// - on the first 'segment' of the bezier will be used.
    /// - we don't currently support quadratics.
    #[cfg(test)]
    pub(crate) fn from_bezpath(
        path: impl IntoIterator<Item = PathEl>,
    ) -> Result<Self, &'static str> {
        let path_id = next_id();
        let mut els = path.into_iter();
        let mut points = Vec::new();
        let mut closed = false;

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
                    closed = true;
                    break;
                }
            }
        }

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
        let closed = if let NoradPType::Move = src.points[0].typ {
            false
        } else {
            true
        };

        let path_id = next_id();

        let mut points: Vec<PathPoint> = src
            .points
            .iter()
            .map(|src_point| {
                //eprintln!("({}, {}): {:?}{}", src_point.x, src_point.y, src_point.typ, if src_point.smooth { " smooth" } else { "" });
                let point = DPoint::new(src_point.x.round() as f64, src_point.y.round() as f64);
                let typ = match &src_point.typ {
                    NoradPType::OffCurve => PointType::OffCurve,
                    NoradPType::QCurve => panic!(
                        "quadratics unsupported, we should have \
                         validated input before now"
                    ),
                    NoradPType::Move | NoradPType::Line | NoradPType::Curve if src_point.smooth => {
                        PointType::OnCurveSmooth
                    }
                    _other => PointType::OnCurve,
                };
                let id = EntityId {
                    parent: path_id,
                    point: next_id(),
                };
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
        let mut prev_off_curve = self.points.last().map(|p| p.typ) == Some(PointType::OffCurve);
        for p in self.points.iter() {
            let typ = match p.typ {
                PointType::OnCurve | PointType::OnCurveSmooth
                    if points.is_empty() && !self.closed =>
                {
                    NoradPType::Move
                }
                PointType::OffCurve => NoradPType::OffCurve,
                PointType::OnCurve | PointType::OnCurveSmooth if prev_off_curve => {
                    NoradPType::Curve
                }
                _ => NoradPType::Line,
            };
            let smooth = p.typ == PointType::OnCurveSmooth;
            let x = p.point.x as f32;
            let y = p.point.y as f32;
            points.push(ContourPoint {
                x,
                y,
                typ,
                smooth,
                identifier: None,
                name: None,
            });
            prev_off_curve = p.typ == PointType::OffCurve;
        }

        if self.closed {
            points.rotate_right(1);
        }
        Contour {
            points,
            identifier: None,
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn points(&self) -> &[PathPoint] {
        self.points.as_slice()
    }

    fn points_mut(&mut self) -> &mut Vec<PathPoint> {
        Arc::make_mut(&mut self.points)
    }

    pub fn iter_segments(&self) -> impl Iterator<Item = PathSeg> {
        let prev_pt = *self.start_point();
        let idx = if self.closed { 0 } else { 1 };
        PathSegIter {
            points: self.points.clone(),
            prev_pt,
            idx,
        }
    }

    pub fn trailing(&self) -> Option<&DPoint> {
        self.trailing.as_ref()
    }

    pub fn clear_trailing(&mut self) {
        self.trailing = None;
    }

    /// Whether we should draw the 'trailing' control point & handle.
    /// We always do this for the first point, if it exists; otherwise
    /// we do it for curve points only.
    pub fn should_draw_trailing(&self) -> bool {
        self.points.len() == 1 || self.last_segment_is_curve()
    }

    /// Returns the start point of the path.
    pub fn start_point(&self) -> &PathPoint {
        assert!(!self.points.is_empty(), "empty path is not constructable");
        if self.closed {
            self.points.last().unwrap()
        } else {
            self.points.first().unwrap()
        }
    }

    pub fn bezier(&self) -> BezPath {
        let mut bez = BezPath::new();
        self.append_to_bezier(&mut bez);
        bez
    }

    pub(crate) fn append_to_bezier(&self, bez: &mut BezPath) {
        bez.move_to(self.start_point().point.to_raw());
        let mut i = if self.closed { 0 } else { 1 };
        //self.debug_print_points();

        while i < self.points.len() {
            if self.points[i].is_on_curve() {
                bez.line_to(self.points[i].point.to_raw());
                i += 1;
            } else {
                bez.curve_to(
                    self.points[i].point.to_raw(),
                    self.points[i + 1].point.to_raw(),
                    self.points[self.next_idx(i + 1)].point.to_raw(),
                );
                i += 3;
            }
        }
        if self.closed {
            bez.close_path();
        }
    }

    pub fn screen_dist(&self, vport: ViewPort, point: Point) -> f64 {
        let screen_bez = vport.affine() * self.bezier();
        let (_, x, y) = screen_bez.nearest(point, 0.1);
        Vec2::new(x, y).hypot()
    }

    /// Appends a point. Called when the user clicks. This point is always a corner;
    /// if the user drags it will be converted to a curve then.
    ///
    /// Returns the id of the newly added point, or the start/end point if this
    /// closes the path.
    pub fn append_point(&mut self, point: DPoint) -> EntityId {
        if !self.closed && point == self.points[0].point {
            return self.close();
        }
        let new = PathPoint::on_curve(self.id, point);
        self.points_mut().push(new);
        new.id
    }

    pub fn nudge_points(&mut self, points: &[EntityId], v: DVec2) {
        let mut to_nudge = HashSet::new();
        for point in points {
            let idx = match self.points.iter().position(|p| p.id == *point) {
                Some(idx) => idx,
                None => continue,
            };
            to_nudge.insert(idx);
            if self.points[idx].is_on_curve() {
                let prev = self.prev_idx(idx);
                let next = self.next_idx(idx);
                if !self.points[prev].is_on_curve() {
                    to_nudge.insert(prev);
                }
                if !self.points[next].is_on_curve() {
                    to_nudge.insert(next);
                }
            }
        }

        for idx in &to_nudge {
            self.nudge_point(*idx, v);
            if !self.points[*idx].is_on_curve() {
                if let Some((on_curve, handle)) = self.tangent_handle(*idx) {
                    if !to_nudge.contains(&handle) {
                        self.adjust_handle_angle(*idx, on_curve, handle);
                    }
                }
            }
        }
    }

    fn nudge_point(&mut self, idx: usize, v: DVec2) {
        self.points_mut()[idx].point.x += v.x;
        self.points_mut()[idx].point.y += v.y;
    }

    /// Returns the index for the on_curve point and the 'other' handle
    /// for an offcurve point, if it exists.
    fn tangent_handle(&self, idx: usize) -> Option<(usize, usize)> {
        assert!(!self.points[idx].is_on_curve());
        let prev = self.prev_idx(idx);
        let next = self.next_idx(idx);
        if self.points[prev].typ == PointType::OnCurveSmooth {
            let prev2 = self.prev_idx(prev);
            if !self.points[prev2].is_on_curve() {
                return Some((prev, prev2));
            }
        } else if self.points[next].typ == PointType::OnCurveSmooth {
            let next2 = self.next_idx(next);
            if !self.points[next2].is_on_curve() {
                return Some((next, next2));
            }
        }
        None
    }

    /// Update a tangent handle in response to the movement of the partner handle.
    /// `bcp1` is the handle that has moved, and `bcp2` is the handle that needs
    /// to be adjusted.
    fn adjust_handle_angle(&mut self, bcp1: usize, on_curve: usize, bcp2: usize) {
        let raw_angle = (self.points[bcp1].point - self.points[on_curve].point).to_raw();
        if raw_angle.hypot() == 0.0 {
            return;
        }

        // that angle is in the opposite direction, so flip it
        let norm_angle = raw_angle.normalize() * -1.0;
        let handle_len = (self.points[bcp2].point - self.points[on_curve].point)
            .hypot()
            .abs();

        let new_handle_offset = DVec2::from_raw(norm_angle * handle_len);
        let new_pos = self.points[on_curve].point + new_handle_offset;
        self.points_mut()[bcp2].point = new_pos;
    }

    pub fn debug_print_points(&self) {
        eprintln!(
            "path {}, len {} closed {}",
            self.id,
            self.points.len(),
            self.closed
        );
        for point in self.points.iter() {
            eprintln!(
                "[{}, {}]: {:?} {:?}",
                point.id.parent, point.id.point, point.point, point.typ
            );
        }
    }

    pub fn delete_points(&mut self, points: &[EntityId]) {
        eprintln!("deleting {:?}", points);
        for point in points {
            self.delete_point(*point)
        }
    }

    //FIXME: this is currently buggy :(
    fn delete_point(&mut self, point_id: EntityId) {
        let idx = match self.points.iter().position(|p| p.id == point_id) {
            Some(idx) => idx,
            None => return,
        };

        let prev_idx = self.prev_idx(idx);
        let next_idx = self.next_idx(idx);

        eprintln!("deleting {:?}", idx);
        self.debug_print_points();

        match self.points[idx].typ {
            PointType::OffCurve => {
                // delete both of the off curve points for this segment
                let other_id = if self.points[prev_idx].typ == PointType::OffCurve {
                    self.points[prev_idx].id
                } else {
                    assert!(self.points[next_idx].typ == PointType::OffCurve);
                    self.points[next_idx].id
                };
                self.points_mut()
                    .retain(|p| p.id != point_id && p.id != other_id);
            }
            _on_curve if self.points.len() == 1 => {
                self.points_mut().clear();
            }
            // with less than 4 points they must all be on curve
            _on_curve if self.points.len() == 4 => {
                self.points_mut()
                    .retain(|p| p.is_on_curve() && p.id != point_id);
            }

            _on_curve if self.points[prev_idx].is_on_curve() => {
                // this is a line segment
                self.points_mut().remove(idx);
            }
            _on_curve if self.points[next_idx].is_on_curve() => {
                // if we neighbour a corner point, leave handles (neighbour becomes curve)
                self.points_mut().remove(idx);
            }
            _ => {
                assert!(self.points.len() > 4);
                let prev = self.points[prev_idx];
                let next = self.points[next_idx];
                assert!(!prev.is_on_curve() && !next.is_on_curve());
                let to_del = [prev.id, next.id, point_id];
                self.points_mut().retain(|p| !to_del.contains(&p.id));
                if self.points.len() == 3 {
                    self.points_mut().retain(|p| p.is_on_curve());
                }
            }
        }

        // check if any points are smooth that should now just be corners
        for idx in 0..self.points.len() {
            let prev_idx = match idx {
                0 => self.points.len() - 1,
                other => other - 1,
            };
            let next_idx = (idx + 1) % self.points.len();

            if self.points[idx].typ == PointType::OnCurveSmooth
                && self.points[prev_idx].is_on_curve()
                && self.points[next_idx].is_on_curve()
            {
                self.points_mut()[idx].typ = PointType::OnCurve;
            }
        }

        // normalize our representation
        let len = self.points.len();
        if len > 2 && !self.points[0].is_on_curve() && !self.points[len - 1].is_on_curve() {
            self.points_mut().rotate_left(1);
        }

        // if we have fewer than three on_curve points we are open.
        if self.points.len() < 3 {
            self.closed = false;
        }
    }

    /// Called when the user drags (modifying the bezier control points) after clicking.
    pub fn update_for_drag(&mut self, handle: DPoint) {
        assert!(!self.points.is_empty());
        if !self.last_segment_is_curve() {
            self.convert_last_to_curve(handle);
        } else {
            self.update_trailing(handle);
        }
    }

    pub fn last_segment_is_curve(&self) -> bool {
        let len = self.points.len();
        len > 2 && !self.points[len - 2].is_on_curve()
    }

    pub fn toggle_on_curve_point_type(&mut self, id: EntityId) {
        let idx = self.idx_for_point(id).unwrap();
        let has_ctrl = !self.points[self.prev_idx(idx)].is_on_curve()
            || !self.points[self.next_idx(idx)].is_on_curve();
        let point = &mut self.points_mut()[idx];
        point.typ = match point.typ {
            PointType::OnCurve if has_ctrl => PointType::OnCurveSmooth,
            PointType::OnCurveSmooth => PointType::OnCurve,
            other => other,
        }
    }

    /// If the user drags after mousedown, we convert the last point to a curve.
    fn convert_last_to_curve(&mut self, handle: DPoint) {
        assert!(!self.points.is_empty());
        if self.points.len() > 1 {
            let mut prev = self.points_mut().pop().unwrap();
            prev.typ = PointType::OnCurveSmooth;
            let p1 = self
                .trailing
                .take()
                .unwrap_or(self.points.last().unwrap().point);
            let p2 = prev.point - (handle - prev.point);
            let pts = &[
                PathPoint::off_curve(self.id, p1),
                PathPoint::off_curve(self.id, p2),
                prev,
            ];
            self.points_mut().extend(pts);
        }
        self.trailing = Some(handle);
    }

    /// Update the curve while the user drags a new control point.
    fn update_trailing(&mut self, handle: DPoint) {
        if self.points.len() > 1 {
            let len = self.points.len();
            assert!(self.points[len - 1].typ != PointType::OffCurve);
            assert!(self.points[len - 2].typ == PointType::OffCurve);
            let on_curve_pt = self.points[len - 1].point;
            let new_p = on_curve_pt - (handle - on_curve_pt);
            self.points_mut()[len - 2].point = new_p;
        }
        self.trailing = Some(handle);
    }

    // in an open path, the first point is essentially a `move_to` command.
    // 'closing' the path means moving this point to the end of the list.
    fn close(&mut self) -> EntityId {
        assert!(!self.closed);
        self.points_mut().rotate_left(1);
        self.closed = true;
        self.points.last().unwrap().id
    }

    #[inline]
    fn prev_idx(&self, idx: usize) -> usize {
        if idx == 0 {
            self.points.len() - 1
        } else {
            idx - 1
        }
    }

    #[inline]
    fn next_idx(&self, idx: usize) -> usize {
        (idx + 1) % self.points.len()
    }

    fn idx_for_point(&self, point: EntityId) -> Option<usize> {
        self.points.iter().position(|p| p.id == point)
    }

    pub(crate) fn path_point_for_id(&self, point: EntityId) -> Option<PathPoint> {
        assert!(point.parent == self.id);
        self.idx_for_point(point).map(|idx| self.points[idx])
    }

    pub(crate) fn prev_point(&self, point: EntityId) -> PathPoint {
        assert!(point.parent == self.id);
        let idx = self.idx_for_point(point).expect("bad input to prev_point");
        let idx = self.prev_idx(idx);
        self.points[idx]
    }

    pub(crate) fn next_point(&self, point: EntityId) -> PathPoint {
        assert!(point.parent == self.id);
        let idx = self.idx_for_point(point).expect("bad input to next_point");
        let idx = self.next_idx(idx);
        self.points[idx]
    }

    pub(crate) fn split_segment_at_point(&mut self, seg: PathSeg, t: f64) {
        let (existing_control_pts, points_to_insert) = match seg {
            PathSeg::Line(..) => (0, 1),
            PathSeg::Cubic(..) => (2, 5),
        };

        let pre_seg = seg.subsegment(0.0..t);
        let post_seg = seg.subsegment(t..1.0);
        let mut insert_idx = self
            .points
            .iter()
            .position(|p| p.id == seg.start_id())
            .unwrap();
        insert_idx = self.next_idx(insert_idx);
        //let mut to_replace = points_to_insert;
        let mut iter = pre_seg
            .into_iter()
            .skip(1)
            .chain(post_seg.into_iter().skip(1));
        let self_id = self.id();
        let points = self.points_mut();
        for i in 0..points_to_insert {
            let mut next_pt = iter.next().unwrap();
            next_pt.id.parent = self_id;
            if i < existing_control_pts {
                points[insert_idx] = next_pt;
            } else {
                points.insert(insert_idx, next_pt);
            }
            insert_idx += 1;
        }
        mark_tangent_handles(points);
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
                    pt.typ = PointType::OnCurveSmooth;
                }
            }
        }
        //pt.id.parent = parent_id;
        points[idx] = pt;
        idx += 1;
    }
}

impl PathSeg {
    pub(crate) fn start_id(&self) -> EntityId {
        match self {
            PathSeg::Line(p1, _) => p1.id,
            PathSeg::Cubic(p1, ..) => p1.id,
        }
    }

    pub(crate) fn end_id(&self) -> EntityId {
        match self {
            PathSeg::Line(_, p2) => p2.id,
            PathSeg::Cubic(.., p2) => p2.id,
        }
    }

    pub(crate) fn to_kurbo(self) -> KurboPathSeg {
        match self {
            PathSeg::Line(p1, p2) => {
                KurboPathSeg::Line(Line::new(p1.point.to_raw(), p2.point.to_raw()))
            }
            PathSeg::Cubic(p1, p2, p3, p4) => KurboPathSeg::Cubic(CubicBez::new(
                p1.point.to_raw(),
                p2.point.to_raw(),
                p3.point.to_raw(),
                p4.point.to_raw(),
            )),
        }
    }

    pub(crate) fn subsegment(self, range: Range<f64>) -> Self {
        let subseg = self.to_kurbo().subsegment(range);
        let path_id = self.start_id().parent;
        match subseg {
            KurboPathSeg::Line(Line { p0, p1 }) => PathSeg::Line(
                PathPoint::on_curve(path_id, DPoint::from_raw(p0)),
                PathPoint::on_curve(path_id, DPoint::from_raw(p1)),
            ),
            KurboPathSeg::Cubic(CubicBez { p0, p1, p2, p3 }) => {
                let p0 = PathPoint::on_curve(path_id, DPoint::from_raw(p0));
                let p1 = PathPoint::off_curve(path_id, DPoint::from_raw(p1));
                let p2 = PathPoint::off_curve(path_id, DPoint::from_raw(p2));
                let p3 = PathPoint::on_curve(path_id, DPoint::from_raw(p3));
                PathSeg::Cubic(p0, p1, p2, p3)
            }
            KurboPathSeg::Quad(_) => panic!("quads are not supported"),
        }
    }
}

impl Iterator for PathSegIter {
    type Item = PathSeg;

    fn next(&mut self) -> Option<PathSeg> {
        if self.idx >= self.points.len() {
            return None;
        }
        let seg_start = self.prev_pt;
        let seg = if !self.points[self.idx].is_on_curve() {
            let p1 = self.points[self.idx];
            let p2 = self.points[self.idx + 1];
            self.prev_pt = self.points[self.idx + 2];
            self.idx += 3;
            assert!(
                self.prev_pt.typ.is_on_curve(),
                "{:#?} idx{}",
                &self.points,
                self.idx
            );
            PathSeg::Cubic(seg_start, p1, p2, self.prev_pt)
        } else {
            self.prev_pt = self.points[self.idx];
            self.idx += 1;
            PathSeg::Line(seg_start, self.prev_pt)
        };
        Some(seg)
    }
}

impl std::iter::IntoIterator for PathSeg {
    type Item = PathPoint;
    type IntoIter = SegPointIter;
    fn into_iter(self) -> Self::IntoIter {
        SegPointIter { seg: self, idx: 0 }
    }
}

impl Iterator for SegPointIter {
    type Item = PathPoint;

    fn next(&mut self) -> Option<PathPoint> {
        self.idx += 1;
        match (self.idx, self.seg) {
            (1, PathSeg::Line(p1, _)) => Some(p1),
            (2, PathSeg::Line(_, p2)) => Some(p2),
            (1, PathSeg::Cubic(p1, ..)) => Some(p1),
            (2, PathSeg::Cubic(_, p2, ..)) => Some(p2),
            (3, PathSeg::Cubic(_, _, p3, ..)) => Some(p3),
            (4, PathSeg::Cubic(_, _, _, p4)) => Some(p4),
            _ => None,
        }
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "id{}.{}", self.parent, self.point)
    }
}

impl std::fmt::Debug for PathSeg {
    #[allow(clippy::many_single_char_names)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PathSeg::Line(one, two) => write!(
                f,
                "({}->{}) Line({:?}, {:?})",
                self.start_id(),
                self.end_id(),
                one.point,
                two.point
            ),
            PathSeg::Cubic(a, b, c, d) => write!(
                f,
                "Cubic({:?}, {:?}, {:?}, {:?})",
                a.point, b.point, c.point, d.point
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use druid::kurbo::{Rect, Shape};

    #[test]
    fn from_bezpath() {
        let rect = Rect::from_origin_size((0., 0.), (10., 10.));
        let path = Path::from_bezpath(rect.to_bez_path(0.1)).unwrap();
        assert!(path.is_closed());
        assert_eq!(path.points.len(), 4);
        assert_eq!(path.start_point().point.to_raw(), Point::ORIGIN);
    }

    #[test]
    fn iter_rect_segs() {
        let rect = Rect::new(0., 0., 10., 10.);
        let path = Path::from_bezpath(rect.to_bez_path(0.1)).unwrap(); // make_rect_path(rect);

        let mut seg_iter = path.iter_segments();
        assert!(matches!(seg_iter.next().unwrap(), PathSeg::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), PathSeg::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), PathSeg::Line(..)));
        assert!(matches!(seg_iter.next().unwrap(), PathSeg::Line(..)));
    }

    #[test]
    fn iter_line_sects() {
        let mut path = Path::new(DPoint::new(0., 0.));
        path.append_point(DPoint::new(10., 10.));

        let mut seg_iter = path.iter_segments();
        let seg = seg_iter.next().unwrap();
        let line = match seg.to_kurbo() {
            KurboPathSeg::Line(line) => line,
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

        let path = Path::from_bezpath(bez).unwrap();

        assert!(path.is_closed());
        assert_eq!(path.points().len(), 3);

        let mut iter = path.iter_segments().map(PathSeg::to_kurbo);
        assert_eq!(iter.next(), Some(Line::new((10., 10.), (0., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((0., 0.), (20., 0.)).into()));
        assert_eq!(iter.next(), Some(Line::new((20., 0.), (10., 10.)).into()));
    }
}
