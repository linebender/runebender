//! The knife tool

use druid::kurbo::{Line, LineIntersection, ParamCurve, ParamCurveArclen};
use druid::piet::StrokeStyle;
use druid::{Env, EventCtx, KbKey, KeyEvent, MouseEvent, PaintCtx, Point, RenderContext};

use crate::cubic_path::CubicPath;
use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::Path;
use crate::point::{EntityId, PathPoint};
use crate::point_list::RawSegment;
use crate::theme;
use crate::tools::{EditType, Tool};

const MAX_RECURSE: usize = 16;

/// The state of the rectangle tool.
#[derive(Debug, Clone)]
pub struct Knife {
    gesture: GestureState,
    shift_locked: bool,
    stroke_style: StrokeStyle,
    /// during a drag, the places where we intersect a path; we just hold
    /// on to this so we don't always need to reallocate.
    intersections: Vec<DPoint>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GestureState {
    Ready,
    /// these points are in design space but may be fractional, hence not DPoint
    Begun {
        start: Point,
        current: Point,
    },
    Finished,
}

#[derive(Clone, Copy)]
struct Hit {
    intersection: LineIntersection,
    point: Point,
    seg: RawSegment,
}

impl Default for Knife {
    fn default() -> Self {
        let mut stroke_style = StrokeStyle::new();
        stroke_style.set_dash(vec![4.0, 2.0], 0.0);
        Knife {
            gesture: Default::default(),
            shift_locked: false,
            stroke_style,
            intersections: Vec::new(),
        }
    }
}

impl Default for GestureState {
    fn default() -> Self {
        GestureState::Ready
    }
}

impl Knife {
    fn current_points(&self) -> Option<(Point, Point)> {
        if let GestureState::Begun { start, current } = self.gesture {
            let mut current = current;
            if self.shift_locked {
                let delta = current - start;
                if delta.x.abs() > delta.y.abs() {
                    current.y = start.y;
                } else {
                    current.x = start.x;
                }
            }
            Some((start, current))
        } else {
            None
        }
    }

    fn current_line_in_dspace(&self) -> Option<Line> {
        self.current_points().map(|(p1, p2)| Line::new(p1, p2))
    }

    fn current_line_in_screen_space(&self, data: &EditSession) -> Option<Line> {
        let xform = data.viewport.affine();
        self.current_points()
            .map(|(p1, p2)| Line::new(xform * p1, xform * p2))
    }

    fn update_intersections(&mut self, data: &EditSession) {
        let line = match self.current_line_in_dspace() {
            Some(line) => line,
            None => return,
        };

        self.intersections.clear();

        let iter = data
            .paths
            .iter()
            .flat_map(Path::iter_segments)
            .flat_map(|seg| {
                seg.intersect_line(line)
                    .into_iter()
                    .map(move |hit| DPoint::from_raw(line.eval(hit.line_t)))
            });
        self.intersections.extend(iter);
    }
}

impl Tool for Knife {
    fn name(&self) -> &'static str {
        "Knife"
    }

    fn cancel(
        &mut self,
        mouse: &mut Mouse,
        _ctx: &mut EventCtx,
        data: &mut EditSession,
    ) -> Option<EditType> {
        mouse.cancel(data, self);
        None
    }

    fn key_down(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        if key.key == KbKey::Shift {
            self.shift_locked = true;
            self.update_intersections(data);
            ctx.request_paint();
        }
        None
    }

    fn key_up(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        if key.key == KbKey::Shift {
            self.shift_locked = false;
            self.update_intersections(data);
            ctx.request_paint();
        }
        None
    }
    fn init_mouse(&mut self, mouse: &mut Mouse) {
        mouse.min_drag_distance = 2.0;
    }

    fn mouse_event(
        &mut self,
        event: TaggedEvent,
        mouse: &mut Mouse,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        let pre_state = self.gesture;
        mouse.mouse_event(event, data, self);
        if pre_state != self.gesture {
            ctx.request_paint();
        }

        if self.gesture == GestureState::Finished {
            self.gesture = GestureState::Ready;
            Some(EditType::Normal)
        } else {
            None
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditSession, _env: &Env) {
        if let Some(line) = self.current_line_in_screen_space(data) {
            let unit_vec = (line.end() - line.start()).normalize();
            //let perp = druid::kurbo::Vec2::new(-unit_vec.y, unit_vec.x);

            ctx.stroke_styled(line, &_env.get(theme::KNIFE_GUIDE), 2.0, &self.stroke_style);

            for point in &self.intersections {
                let point = data.viewport.to_screen(*point);
                let cut_mark_start = point - (unit_vec * 4.0);
                let cut_mark_end = point + (unit_vec * 4.0);
                let cut_mark = Line::new(cut_mark_start, cut_mark_end);
                ctx.stroke(cut_mark, &_env.get(theme::KNIFE_GUIDE_INTERSECTION), 8.0);

                //let cms1 = cut_mark_start + perp * 2.0;
                //let cme1 = cut_mark_end + perp * 2.0;

                //let cms2 = cut_mark_start - perp * 2.0;
                //let cme2 = cut_mark_end - perp * 2.0;
                //ctx.stroke(Line::new(cms1, cme1), &Color::BLACK, 1.0);
                //ctx.stroke(Line::new(cms2, cme2), &Color::BLACK, 1.0);
            }
        }
    }
}

impl MouseDelegate<EditSession> for Knife {
    fn cancel(&mut self, _data: &mut EditSession) {
        self.gesture = GestureState::Ready;
    }

    fn left_down(&mut self, event: &MouseEvent, data: &mut EditSession) {
        if event.count == 1 {
            let pt = data.viewport.inverse_affine() * event.pos;
            self.gesture = GestureState::Begun {
                start: pt,
                current: pt,
            };
            self.shift_locked = event.mods.shift();
        }
    }

    fn left_drag_ended(&mut self, drag: Drag, data: &mut EditSession) {
        if let GestureState::Begun { current, .. } = &mut self.gesture {
            let now = data.viewport.inverse_affine() * drag.current.pos;
            if now != *current {
                *current = now;
                self.update_intersections(data);
            }
        }

        if let Some(line) = self.current_line_in_dspace() {
            if !self.intersections.is_empty() {
                let new_paths = slice_paths(&data.paths, line);
                data.paths = new_paths.into();
            }
        }

        self.gesture = GestureState::Finished;
    }

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        if let GestureState::Begun { current, .. } = &mut self.gesture {
            *current = data.viewport.inverse_affine() * drag.current.pos;
            self.update_intersections(data);
        }
    }
}

impl Hit {
    fn new(line: Line, intersection: LineIntersection, seg: RawSegment) -> Self {
        let point = line.eval(intersection.line_t);
        Hit {
            intersection,
            point,
            seg,
        }
    }

    fn seg_t(&self) -> f64 {
        self.intersection.segment_t
    }
}

/// What the knife tool does.
///
/// Checks for intersection with all paths, modifying old and adding
/// new paths as necessary.
///
/// The algorithm is pretty straight forward, and operates individually
/// on each path
///
/// - for each path, check if there are any intersections.
///
/// for paths with intersections:
/// - take the first two hits (sorted by t on the line) on the path
/// - split the path at those two points
///     - for each new path, insert a new line segment between the two cut points
/// - modify the line so that it now starts at the last of those hit points
/// - recursively try to cut each new path with the modified line
fn slice_paths(paths: &[Path], line: Line) -> Vec<Path> {
    let mut out = Vec::new();
    for path in paths {
        if !path.is_hyper() {
            slice_path(path, line, &mut out);
        } else {
            log::info!("slice not implemented for hyperbezier paths");
            out.push(path.clone());
        }
    }
    out
}

/// Slice a path with a line.
///
/// Resulting paths are pushed to the `acc` vec.
///
/// If no modifications are made, the source `path` should still be pushed to `acc`.
fn slice_path(path: &Path, line: Line, acc: &mut Vec<Path>) {
    let mut hits = Vec::new();
    // we clone here; the impl is recursive and if this path isn't sliced the
    // clone will be returned in `acc`.
    if let Path::Cubic(path) = path {
        slice_path_impl(path.clone(), line, acc, &mut hits, 0)
    }
}

/// does the actual work
/// - we reuse a vector for calculating hits, because... why not
/// - we track recursions and bail at some limit, because I don't trust all the edge cases.
fn slice_path_impl(
    mut path: CubicPath,
    line: Line,
    acc: &mut Vec<Path>,
    hit_buf: &mut Vec<Hit>,
    recurse: usize,
) {
    hit_buf.clear();
    hit_buf.extend(path.iter_segments().flat_map(|seg| {
        seg.to_kurbo()
            .intersect_line(line)
            .into_iter()
            .map(move |hit| Hit::new(line, hit, seg))
    }));

    if hit_buf.len() <= 1 || recurse == MAX_RECURSE {
        if let Some(hit) = hit_buf.first() {
            path.split_segment_at_point(hit.seg, hit.intersection.segment_t);
        }
        if recurse == MAX_RECURSE {
            log::info!("slice_path hit recurse limit");
        }
        acc.push(path.into());
        return;
    }

    // we sort based on `t` on the line, that is, in the order of the "cut"
    hit_buf.sort_by(|a, b| {
        a.intersection
            .line_t
            .partial_cmp(&b.intersection.line_t)
            .unwrap()
    });

    // we just work with the first two intersections at a time.
    let start = hit_buf[0];
    let end = *hit_buf.get(1).expect("len already checked");

    // stash where on the line the last hit we're using is;
    // we will resume from here afterwards.
    //
    // we add an amount of `t` equal to a design-space unit between slice segments.
    // this hopefully means we won't cut a line, and then immediately cut one of the
    // new segments.
    let slice_ep = 1.0 / line.arclen(1e-6);
    let next_line_start_t = end.intersection.line_t + slice_ep;

    // order the points based on the order they appear in the source path;
    // this makes other logic easier (we will hit the start pt first when iterating).
    let (start, end) = order_points(&path, start, end);

    // generate the path on either side of the cut
    let (path_one, path_two) = split_path_at_intersections(&path, start, end);

    // calculate the cut line that remains to be processed
    let line = line.subsegment(next_line_start_t..1.0);
    // recurse on each of the new paths
    slice_path_impl(path_one, line, acc, hit_buf, recurse + 1);
    slice_path_impl(path_two, line, acc, hit_buf, recurse + 1);
}

/// Given a path and two points on that path, divide it in two.
///
/// The 'first' path includes the paths original start point, and may be open.
/// The 'second' path is the part that is 'sliced off', and it always closed.
fn split_path_at_intersections(path: &CubicPath, start: Hit, end: Hit) -> (CubicPath, CubicPath) {
    let one_id = path.path_points().id();
    let two_id = EntityId::next();
    let mut one_points = Vec::new();
    let mut two_points = Vec::new();
    let mut iter = path.iter_segments();
    let mut two_is_done = false;

    // the part leading up to the cut
    for seg in &mut iter {
        // just copy over all points up to our first cut
        if seg.start_id() != start.seg.start_id() {
            append_all_points(&mut one_points, seg);
        } else {
            let cut_t = start.seg_t();
            append_all_points(&mut one_points, seg.subsegment(0.0..cut_t));

            if seg.start_id() == end.seg.start_id() {
                // the special case where the cut starts and ends in the same segment:
                append_all_points(&mut one_points, seg.subsegment(end.seg_t()..1.));
                append_all_points(&mut two_points, seg.subsegment(cut_t..end.seg_t()));
                two_is_done = true;
            } else {
                append_all_points(&mut two_points, seg.subsegment(cut_t..1.));
            }

            if !path.is_closed() {
                // add the cut line
                two_points.push(PathPoint::on_curve(two_id, DPoint::from_raw(start.point)));
            }
            break;
        }
    }

    // the part between the two cuts
    for seg in &mut iter {
        if seg.start_id() == end.seg.start_id() {
            let cut_t = end.seg_t();
            append_all_points(&mut one_points, seg.subsegment(cut_t..1.0));
            if !two_is_done {
                append_all_points(&mut two_points, seg.subsegment(0.0..cut_t));
            }
            break;
        } else if !two_is_done {
            append_all_points(&mut two_points, seg);
        }
    }

    // the part after the cut
    iter.for_each(|seg| append_all_points(&mut one_points, seg));

    if one_points.first().map(|p| p.point) == one_points.last().map(|p| p.point) {
        one_points.pop();
    }

    let path1 = finalize_path(one_points, one_id, path.is_closed());
    let path2 = finalize_path(two_points, two_id, true);
    (path1, path2)
}

/// set tangent handles, set correct parent ids, and construct the path
fn finalize_path(mut points: Vec<PathPoint>, parent_id: EntityId, closed: bool) -> CubicPath {
    crate::path::mark_tangent_handles(&mut points);
    points.iter_mut().for_each(|p| p.reparent(parent_id));
    if closed {
        points.rotate_left(1);
    }

    CubicPath::from_raw_parts(parent_id, points, None, closed)
}

fn append_all_points(dest: &mut Vec<PathPoint>, seg: RawSegment) {
    let mut iter = seg.into_iter();
    let first = iter.next().unwrap();
    // we skip the first point if it's the same as the current previous point
    if dest.last().map(|p| p.point) != Some(first.point) {
        dest.push(first);
    }
    dest.extend(iter);
}

/// order our two cut points based on the order of points in the path.
///
/// this simplifies the two slice functions, since they can assume they will hit
/// the start point first while iterating.
fn order_points(path: &CubicPath, start: Hit, end: Hit) -> (Hit, Hit) {
    for seg in path.iter_segments() {
        if seg.start_id() == start.seg.start_id() {
            // in the special case that we're slicing a single segment,
            // we want to order the slice points based on their `t` on that segment.
            if seg.start_id() == end.seg.start_id()
                && end.intersection.segment_t < start.intersection.segment_t
            {
                return (end, start);
            }
            return (start, end);
        } else if seg.start_id() == end.seg.start_id() {
            return (end, start);
        }
    }
    debug_assert!(false, "order points fell through?");
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cubic_path::CubicPath;

    use druid::kurbo::BezPath;

    #[must_use = "this should be unwrapped"]
    fn equal_points(one: &Path, two: &Path) -> Result<(), String> {
        let one_len = one.points().len();
        let two_len = two.points().len();
        if one_len != two_len {
            let mut out = format!("unequal lengths: {}/{}\n", one_len, two_len);
            let longer = one_len.max(two_len);
            (0..longer)
                .into_iter()
                .map(|i| {
                    let p1 = one
                        .points()
                        .get(i)
                        .map(|p| p.point.to_string())
                        .unwrap_or("None".into());
                    let p2 = two
                        .points()
                        .get(i)
                        .map(|p| p.point.to_string())
                        .unwrap_or("None".into());
                    format!("{:<10} {}\n", p1, p2)
                })
                .for_each(|line| out.push_str(&line));
            return Err(out);
        }
        for (i, (a, b)) in one
            .points()
            .into_iter()
            .zip(two.points().into_iter())
            .enumerate()
        {
            if a.point != b.point {
                return Err(format!("{} != {} (#{})", a.point, b.point, i));
            }
        }
        Ok(())
    }

    macro_rules! assert_equal_points {
        ($left:expr, $right:expr) => {
            match equal_points(&$left, &$right) {
                Ok(_) => (),
                Err(msg) => panic!("Unequal paths:\n{}", msg),
            }
        };
    }

    #[test]
    fn triangle() {
        let mut path = Path::new(DPoint::new(10., 10.));
        path.line_to(DPoint::new(0., 0.), false);
        path.line_to(DPoint::new(20., 0.), false);
        path.close(false);

        let line = Line::new((3., 6.), (8., -2.));
        let mut out = Vec::new();
        slice_path(&path, line, &mut out);

        assert_eq!(out.len(), 2);
        let one = &out[0];
        let two = &out[1];

        let one_segs = one
            .iter_segments()
            .map(|seg| seg.raw_segment().to_kurbo())
            .collect::<Vec<_>>();
        let exp = vec![
            Line::new((10., 10.), (4., 4.)).into(),
            Line::new((4., 4.), (7., 0.)).into(),
            Line::new((7.0, 0.), (20., 0.)).into(),
            Line::new((20., 0.), (10., 10.)).into(),
        ];
        assert_eq!(one_segs, exp, "{:#?}\n{:#?}", one_segs, exp);

        let two_segs = two
            .iter_segments()
            .map(|seg| seg.raw_segment().to_kurbo())
            .collect::<Vec<_>>();
        let exp = vec![
            Line::new((4., 4.), (0., 0.)).into(),
            Line::new((0.0, 0.), (7., 0.)).into(),
            Line::new((7., 0.), (4., 4.)).into(),
        ];
        assert_eq!(two_segs, exp, "{:#?}\n{:#?}", one_segs, exp);
    }

    // the same line sliced from different directions should produce
    // the same results
    #[test]
    fn slice_single_curve_segment() {
        let mut bez = BezPath::new();
        bez.move_to((0.0, 0.0));
        bez.curve_to((0.0, 0.0), (0.0, 10.0), (10.0, 10.0));
        bez.curve_to((15.0, 10.0), (15.0, 20.0), (20.0, 20.0));
        bez.curve_to((25.0, 20.0), (21.0, 5.0), (15.0, 5.0));
        bez.curve_to((9.0, 5.0), (15.0, 0.0), (0.0, 0.0));
        bez.close_path();

        let path: Path = CubicPath::from_bezpath(bez).unwrap().into();

        // first try slicing a non-first segment
        let slice_line1 = Line::new((10., 20.), (25., 10.));
        let slice_line2 = Line::new((25., 10.), (10., 20.));

        let mut out = Vec::new();
        slice_path(&path, slice_line1, &mut out);
        let first = out.clone();
        out.clear();
        slice_path(&path, slice_line2, &mut out);
        let second = out;
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);

        assert_equal_points!(first[0], second[0]);
        assert_equal_points!(first[1], second[1]);

        // then try slicing the first segment
        let slice_line1 = Line::new((0., 10.), (10., 0.));
        let slice_line2 = Line::new((10., 0.), (0., 10.));

        let mut out = Vec::new();
        slice_path(&path, slice_line1, &mut out);
        let first = out.clone();
        out.clear();
        slice_path(&path, slice_line2, &mut out);
        let second = out;
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);

        assert_equal_points!(first[0], second[0]);
        assert_equal_points!(first[1], second[1]);
    }

    #[test]
    fn open_single_segment_curve() {
        let mut bez = BezPath::new();
        bez.move_to((0.0, 0.0));
        bez.curve_to((0.0, 15.0), (10.0, 15.0), (10.0, 0.0));

        let path: Path = CubicPath::from_bezpath(bez).unwrap().into();
        let slice_line = Line::new((0., 8.), (10., 8.));
        let paths = slice_paths(&[path], slice_line);
        assert_eq!(paths.len(), 2);

        let path1 = paths.get(0).unwrap();
        let path2 = paths.get(1).unwrap();

        assert!(!path1.is_closed());
        assert_eq!(path1.points().len(), 8);

        assert!(path2.is_closed());
        assert_eq!(path2.points().len(), 5);
    }
}
