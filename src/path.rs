use std::collections::HashSet;
use std::sync::Arc;

use super::design_space::{DPoint, DVec2, ViewPort};
use druid::kurbo::{BezPath, Point, Vec2};
use druid::Data;

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

#[derive(Debug, Clone, Copy, Data)]
pub struct PathPoint {
    pub id: EntityId,
    pub point: DPoint,
    pub typ: PointType,
}

#[derive(Debug, Data, Clone)]
pub struct Path {
    id: usize,
    points: std::sync::Arc<Vec<PathPoint>>,
    trailing: Option<DPoint>,
    closed: bool,
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
            points: std::sync::Arc::new(vec![start]),
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
        Path {
            id,
            points: Arc::new(points),
            trailing,
            closed,
        }
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

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn points(&self) -> &[PathPoint] {
        self.points.as_slice()
    }

    fn points_mut(&mut self) -> &mut Vec<PathPoint> {
        std::sync::Arc::make_mut(&mut self.points)
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
}
