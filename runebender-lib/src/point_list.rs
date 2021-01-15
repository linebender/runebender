/// Raw storage for the points that make up a glyph contour
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use super::design_space::{DPoint, DVec2};
use super::point::{EntityId, PathPoint};

use druid::kurbo::{Affine, CubicBez, Line, ParamCurve, PathSeg};
use druid::Data;

#[derive(Debug, Clone, Data)]
pub struct PathPoints {
    path_id: EntityId,
    points: Arc<Vec<PathPoint>>,
    trailing: Option<DPoint>,
    closed: bool,
}

pub struct Cursor<'a> {
    idx: usize,
    inner: &'a mut PathPoints,
}

/// A segment in a one-or-two parameter spline.
///
/// That is: this can be either part of a cubic bezier, or part of a hyperbezier.
///
/// We do not currently support quadratics.
#[derive(Clone, Copy, PartialEq)]
pub enum Segment {
    Line(PathPoint, PathPoint),
    Cubic(PathPoint, PathPoint, PathPoint, PathPoint),
}

impl Cursor<'_> {
    pub fn point(&self) -> &PathPoint {
        &self.inner.points[self.idx]
    }

    pub fn point_mut(&mut self) -> &mut PathPoint {
        &mut Arc::make_mut(&mut self.inner.points)[self.idx]
    }

    pub fn next(&self) -> Option<&PathPoint> {
        self.next_idx().map(|idx| &self.inner.points[idx])
    }

    pub fn next_mut(&mut self) -> Option<&mut PathPoint> {
        let idx = self.next_idx()?;
        Some(&mut Arc::make_mut(&mut self.inner.points)[idx])
    }

    pub fn prev(&self) -> Option<&PathPoint> {
        self.prev_idx().map(|idx| &self.inner.points[idx])
    }

    pub fn prev_mut(&mut self) -> Option<&mut PathPoint> {
        let idx = self.prev_idx()?;
        Some(&mut Arc::make_mut(&mut self.inner.points)[idx])
    }

    pub fn move_to_start(&mut self) {
        self.idx = if self.inner.closed {
            self.inner.len() - 1
        } else {
            0
        };
    }

    pub fn move_to_end(&mut self) {
        self.idx = self.inner.len() - 1
    }

    #[inline]
    fn prev_idx(&self) -> Option<usize> {
        if self.inner.closed() {
            Some(((self.inner.len() + self.idx) - 1) % self.inner.len())
        } else {
            self.idx.checked_sub(1)
        }
    }

    #[inline]
    pub(crate) fn next_idx(&self) -> Option<usize> {
        if self.inner.closed() {
            Some((self.idx + 1) % self.inner.len())
        } else if self.idx < self.inner.len() - 1 {
            Some(self.idx + 1)
        } else {
            None
        }
    }
}

impl PathPoints {
    pub fn new(start_point: DPoint) -> Self {
        let path_id = EntityId::next();
        let start = PathPoint::on_curve(path_id, start_point);
        PathPoints {
            path_id,
            points: Arc::new(vec![start]),
            closed: false,
            trailing: None,
        }
    }

    pub fn from_raw_parts(
        path_id: EntityId,
        mut points: Vec<PathPoint>,
        trailing: Option<DPoint>,
        closed: bool,
    ) -> Self {
        assert!(!points.is_empty(), "path may not be empty");
        assert!(
            points.iter().all(|pt| pt.id.is_child_of(path_id)),
            "{:#?}",
            points
        );
        if !closed {
            assert!(points.first().unwrap().is_on_curve());
        }
        // normalize incoming representation
        // if the path is closed, the last point should be an on-curve point,
        // and is considered the start of the path.
        if closed && !points.last().unwrap().is_on_curve() {
            // we assume there is at least one on-curve point. One day,
            // we will find out one day that this assumption was wrong.
            let rotate_distance = points.iter().position(|p| p.is_on_curve()).unwrap() + 1;

            points.rotate_left(rotate_distance);
        }

        PathPoints {
            path_id,
            points: Arc::new(points),
            trailing,
            closed,
        }
    }

    pub fn len(&self) -> usize {
        self.points.len()
    }

    pub fn closed(&self) -> bool {
        self.closed
    }

    pub fn id(&self) -> EntityId {
        self.path_id
    }

    pub fn trailing(&self) -> Option<&DPoint> {
        self.trailing.as_ref()
    }

    fn trailing_mut(&mut self) -> Option<&mut DPoint> {
        self.trailing.as_mut()
    }

    pub fn clear_trailing(&mut self) {
        self.trailing = None;
    }

    pub fn set_trailing(&mut self, trailing: DPoint) {
        self.trailing = Some(trailing);
    }

    pub fn as_slice(&self) -> &[PathPoint] {
        &self.points
    }

    pub(crate) fn points_mut(&mut self) -> &mut Vec<PathPoint> {
        Arc::make_mut(&mut self.points)
    }

    /// Iterates points in order.
    //TODO: can we combine this with the one above?
    pub(crate) fn iter_points(&self) -> impl Iterator<Item = PathPoint> + '_ {
        let (first, remaining_n) = if self.closed {
            (self.points.last().copied(), self.points.len() - 1)
        } else {
            (None, self.points.len())
        };

        first
            .into_iter()
            .chain(self.points.iter().take(remaining_n).copied())
    }

    pub fn iter_segments(&self) -> Segments {
        let prev_pt = *self.start_point();
        let idx = if self.closed() { 0 } else { 1 };
        Segments {
            points: self.points.clone(),
            prev_pt,
            idx,
        }
    }

    /// Returns a cursor useful for modifying the path.
    ///
    /// If you pass a point id, the cursor will start at that point; if not
    /// it will start at the first point.
    pub fn cursor(&mut self, id: Option<EntityId>) -> Option<Cursor> {
        let idx = id
            .and_then(|id| self.idx_for_point(id))
            .unwrap_or_else(|| self.first_idx());
        Some(Cursor { idx, inner: self })
    }

    pub fn close(&mut self) -> EntityId {
        assert!(!self.closed);
        self.points_mut().rotate_left(1);
        self.closed = true;
        self.points.last().unwrap().id
    }

    pub(crate) fn reverse_contour(&mut self) {
        let last = self.last_idx();
        self.points_mut()[..last].reverse();
    }

    fn first_idx(&self) -> usize {
        if self.closed {
            self.len() - 1
        } else {
            0
        }
    }

    fn last_idx(&self) -> usize {
        if self.closed {
            self.points.len() - 1
        } else {
            self.points.len()
        }
    }

    /// Push a new on-curve point onto the end of the point list.
    ///
    /// The points must not be closed.
    pub fn push_on_curve(&mut self, point: DPoint) -> EntityId {
        assert!(!self.closed);
        let point = PathPoint::on_curve(self.path_id, point);
        self.points_mut().push(point);
        point.id
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
        assert!(point.is_child_of(self.path_id));
        self.idx_for_point(point).map(|idx| self.points[idx])
    }

    pub(crate) fn prev_point(&self, point: EntityId) -> PathPoint {
        assert!(point.is_child_of(self.path_id));
        let idx = self.idx_for_point(point).expect("bad input to prev_point");
        let idx = self.prev_idx(idx);
        self.points[idx]
    }

    pub(crate) fn next_point(&self, point: EntityId) -> PathPoint {
        assert!(point.is_child_of(self.path_id));
        let idx = self.idx_for_point(point).expect("bad input to next_point");
        let idx = self.next_idx(idx);
        self.points[idx]
    }

    pub fn start_point(&self) -> &PathPoint {
        assert!(!self.points.is_empty(), "empty path is not constructable");
        self.points.get(self.first_idx()).unwrap()
    }

    //FIXME: this logic feels weird. What *is* an end point? in a closed path,
    //it should be the start point. Do we mean something else here,
    // like "last on-curve point, not including the start?"
    pub fn end_point(&self) -> &PathPoint {
        assert!(!self.points.is_empty(), "empty path is not constructable");
        let idx = if self.closed {
            self.points.len().saturating_sub(2)
        } else {
            self.points.len() - 1
        };
        &self.points[idx]
    }

    pub fn transform_all(&mut self, affine: Affine, anchor: DPoint) {
        let anchor = anchor.to_dvec2();
        for idx in 0..self.points.len() {
            self.transform_point(idx, affine, anchor);
        }

        if let Some(trailing) = self.trailing_mut() {
            let new_trailing = affine * trailing.to_raw();
            *trailing = DPoint::from_raw(new_trailing);
        }
    }

    /// Apply the provided transform to all selected points, updating handles as
    /// appropriate.
    ///
    /// The `anchor` argument is a point that should be treated as the origin
    /// when applying the transform, which is used for things like scaling from
    /// a fixed point.
    pub fn transform_points(&mut self, points: &[EntityId], affine: Affine, anchor: DPoint) {
        let to_xform = self.points_for_points(points);
        let anchor = anchor.to_dvec2();
        for idx in &to_xform {
            self.transform_point(*idx, affine, anchor);
            if !self.points[*idx].is_on_curve() {
                if let Some((on_curve, handle)) = self.tangent_handle(*idx) {
                    if !to_xform.contains(&handle) {
                        self.adjust_handle_angle(*idx, on_curve, handle);
                    }
                }
            }
        }
    }

    fn transform_point(&mut self, idx: usize, affine: Affine, anchor: DVec2) {
        let anchor = anchor.to_raw();
        let point = self.points[idx].point.to_raw() - anchor;
        let point = affine * point + anchor;
        self.points_mut()[idx].point = DPoint::from_raw(point);
    }

    /// For a list of points, returns a set of indices for those points, including
    /// any associated off-curve points.
    fn points_for_points(&self, points: &[EntityId]) -> HashSet<usize> {
        let mut to_xform = HashSet::new();
        for point in points {
            let idx = match self.points.iter().position(|p| p.id == *point) {
                Some(idx) => idx,
                None => continue,
            };
            to_xform.insert(idx);
            if self.points[idx].is_on_curve() {
                let prev = self.prev_idx(idx);
                let next = self.next_idx(idx);
                if !self.points[prev].is_on_curve() {
                    to_xform.insert(prev);
                }
                if !self.points[next].is_on_curve() {
                    to_xform.insert(next);
                }
            }
        }
        to_xform
    }

    pub fn update_handle(&mut self, point: EntityId, mut dpt: DPoint, is_locked: bool) {
        if let Some(bcp1) = self.idx_for_point(point) {
            if let Some((on_curve, bcp2)) = self.tangent_handle_opt(bcp1) {
                if is_locked {
                    dpt = dpt.axis_locked_to(self.points[on_curve].point);
                }
                self.points_mut()[bcp1].point = dpt;
                if let Some(bcp2) = bcp2 {
                    self.adjust_handle_angle(bcp1, on_curve, bcp2);
                }
            }
        }
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
        let handle_len = (self.points[bcp2].point - self.points[on_curve].point).hypot();

        let new_handle_offset = DVec2::from_raw(norm_angle * handle_len);
        let new_pos = self.points[on_curve].point + new_handle_offset;
        self.points_mut()[bcp2].point = new_pos;
    }

    /// Given the idx of an off-curve point, check if that point has a tangent
    /// handle; that is, if the nearest on-curve point's other neighbour is
    /// also an off-curve point, and the on-curve point is smooth.
    ///
    /// Returns the index for the on_curve point and the 'other' handle
    /// for an offcurve point, if it exists.
    fn tangent_handle(&self, idx: usize) -> Option<(usize, usize)> {
        if let Some((on_curve, Some(bcp2))) = self.tangent_handle_opt(idx) {
            Some((on_curve, bcp2))
        } else {
            None
        }
    }

    /// Given the idx of an off-curve point, return its neighbouring on-curve
    /// point; if that point is smooth and its other neighbour is also an
    /// off-curve, it returns that as well.
    fn tangent_handle_opt(&self, idx: usize) -> Option<(usize, Option<usize>)> {
        assert!(!self.points[idx].is_on_curve());
        let prev = self.prev_idx(idx);
        let next = self.next_idx(idx);
        if self.points[prev].typ.is_on_curve() {
            let prev2 = self.prev_idx(prev);
            if self.points[prev].is_smooth() && !self.points[prev2].is_on_curve() {
                return Some((prev, Some(prev2)));
            } else {
                return Some((prev, None));
            }
        } else if self.points[next].is_on_curve() {
            let next2 = self.next_idx(next);
            if self.points[next].is_smooth() && !self.points[next2].is_on_curve() {
                return Some((next, Some(next2)));
            } else {
                return Some((next, None));
            }
        }
        None
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
        //self.debug_print_points();

        match self.points[idx].typ {
            p if p.is_off_curve() => {
                // delete both of the off curve points for this segment
                let other_id = if self.points[prev_idx].is_off_curve() {
                    self.points[prev_idx].id
                } else {
                    assert!(self.points[next_idx].is_off_curve());
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

            if self.points[idx].is_smooth()
                && self.points[prev_idx].is_on_curve()
                && self.points[next_idx].is_on_curve()
            {
                self.points_mut()[idx].toggle_type();
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

    pub fn last_segment_is_curve(&self) -> bool {
        let len = self.points.len();
        len > 2 && !self.points[len - 2].is_on_curve()
    }
}

impl Segment {
    pub(crate) fn start(&self) -> PathPoint {
        match self {
            Segment::Line(p1, _) => *p1,
            Segment::Cubic(p1, ..) => *p1,
        }
    }

    pub(crate) fn end(&self) -> PathPoint {
        match self {
            Segment::Line(_, p2) => *p2,
            Segment::Cubic(.., p2) => *p2,
        }
    }

    pub(crate) fn start_id(&self) -> EntityId {
        self.start().id
    }

    pub(crate) fn end_id(&self) -> EntityId {
        self.end().id
    }

    pub(crate) fn points(&self) -> impl Iterator<Item = PathPoint> {
        let mut idx = 0;
        let seg = *self;
        std::iter::from_fn(move || {
            idx += 1;
            match (&seg, idx) {
                (_, 1) => Some(seg.start()),
                (Segment::Line(_, p2), 2) => Some(*p2),
                (Segment::Cubic(_, p2, _, _), 2) => Some(*p2),
                (Segment::Cubic(_, _, p3, _), 3) => Some(*p3),
                (Segment::Cubic(_, _, _, p4), 4) => Some(*p4),
                _ => None,
            }
        })
    }

    // FIXME: why a vec? was I just lazy?
    pub(crate) fn ids(&self) -> Vec<EntityId> {
        match self {
            Segment::Line(p1, p2) => vec![p1.id, p2.id],
            Segment::Cubic(p1, p2, p3, p4) => vec![p1.id, p2.id, p3.id, p4.id],
        }
    }

    pub(crate) fn to_kurbo(self) -> PathSeg {
        match self {
            Segment::Line(p1, p2) => PathSeg::Line(Line::new(p1.point.to_raw(), p2.point.to_raw())),
            Segment::Cubic(p1, p2, p3, p4) => PathSeg::Cubic(CubicBez::new(
                p1.point.to_raw(),
                p2.point.to_raw(),
                p3.point.to_raw(),
                p4.point.to_raw(),
            )),
        }
    }

    pub(crate) fn subsegment(self, range: Range<f64>) -> Self {
        let subseg = self.to_kurbo().subsegment(range);
        let path_id = self.start_id().parent();
        match subseg {
            PathSeg::Line(Line { p0, p1 }) => Segment::Line(
                PathPoint::on_curve(path_id, DPoint::from_raw(p0)),
                PathPoint::on_curve(path_id, DPoint::from_raw(p1)),
            ),
            PathSeg::Cubic(CubicBez { p0, p1, p2, p3 }) => {
                let p0 = PathPoint::on_curve(path_id, DPoint::from_raw(p0));
                let p1 = PathPoint::off_curve(path_id, DPoint::from_raw(p1));
                let p2 = PathPoint::off_curve(path_id, DPoint::from_raw(p2));
                let p3 = PathPoint::on_curve(path_id, DPoint::from_raw(p3));
                Segment::Cubic(p0, p1, p2, p3)
            }
            PathSeg::Quad(_) => panic!("quads are not supported"),
        }
    }
}

/// An iterator over the segments in a path list.
pub struct Segments {
    points: Arc<Vec<PathPoint>>,
    prev_pt: PathPoint,
    idx: usize,
}

impl Iterator for Segments {
    type Item = Segment;

    fn next(&mut self) -> Option<Segment> {
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
            Segment::Cubic(seg_start, p1, p2, self.prev_pt)
        } else {
            self.prev_pt = self.points[self.idx];
            self.idx += 1;
            Segment::Line(seg_start, self.prev_pt)
        };
        Some(seg)
    }
}

/// An iterator over the points in a segment
pub struct SegmentPointIter {
    seg: Segment,
    idx: usize,
}

impl std::iter::IntoIterator for Segment {
    type Item = PathPoint;
    type IntoIter = SegmentPointIter;
    fn into_iter(self) -> Self::IntoIter {
        SegmentPointIter { seg: self, idx: 0 }
    }
}

impl Iterator for SegmentPointIter {
    type Item = PathPoint;

    fn next(&mut self) -> Option<PathPoint> {
        self.idx += 1;
        match (self.idx, self.seg) {
            (1, Segment::Line(p1, _)) => Some(p1),
            (2, Segment::Line(_, p2)) => Some(p2),
            (1, Segment::Cubic(p1, ..)) => Some(p1),
            (2, Segment::Cubic(_, p2, ..)) => Some(p2),
            (3, Segment::Cubic(_, _, p3, ..)) => Some(p3),
            (4, Segment::Cubic(_, _, _, p4)) => Some(p4),
            _ => None,
        }
    }
}

impl std::fmt::Debug for Segment {
    #[allow(clippy::many_single_char_names)]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Segment::Line(one, two) => write!(
                f,
                "({}->{}) Line({:?}, {:?})",
                self.start_id(),
                self.end_id(),
                one.point,
                two.point
            ),
            Segment::Cubic(a, b, c, d) => write!(
                f,
                "Cubic({:?}, {:?}, {:?}, {:?})",
                a.point, b.point, c.point, d.point
            ),
        }
    }
}
