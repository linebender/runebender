//! The knife tool

use std::cmp::{Ord, Ordering, PartialEq, PartialOrd};

use druid::kurbo::{
    CubicBez, Line, LineIntersection, ParamCurve, ParamCurveNearest, PathSeg as KurboPathSeg,
};
use druid::piet::StrokeStyle;
use druid::{Color, Env, EventCtx, KeyCode, KeyEvent, MouseEvent, PaintCtx, Point, RenderContext};

use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::mouse::{Drag, Mouse, MouseDelegate, TaggedEvent};
use crate::path::{Path, PathPoint, PathSeg};
use crate::tools::{EditType, Tool};

/// The state of the rectangle tool.
#[derive(Debug, Clone)]
pub struct Knife {
    gesture: GestureState,
    shift_locked: bool,
    stroke_style: StrokeStyle,
    /// during a drag, the places where we intersect a path; we just hold
    /// on to this so we don't always need to reallocate.
    intersections: Vec<Intersection>,
}

#[derive(Debug, Clone, Copy)]
struct Intersection {
    point: DPoint,
    seg: PathSeg,
    hit: LineIntersection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GestureState {
    Ready,
    Begun { start: DPoint, current: DPoint },
    Finished,
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

impl Knife {
    fn current_points(&self) -> Option<(DPoint, DPoint)> {
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
        self.current_points()
            .map(|(p1, p2)| Line::new(p1.to_raw(), p2.to_raw()))
    }

    fn current_line_in_screen_space(&self, data: &EditSession) -> Option<Line> {
        self.current_points()
            .map(|(p1, p2)| Line::new(data.viewport.to_screen(p1), data.viewport.to_screen(p2)))
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
                seg.to_kurbo()
                    .intersect_line(line)
                    .into_iter()
                    .map(move |hit| Intersection {
                        point: DPoint::from_raw(line.eval(hit.line_t)),
                        seg,
                        hit,
                    })
            });
        self.intersections.extend(iter);
        self.intersections.sort();
    }
}

impl Tool for Knife {
    fn name(&self) -> &'static str {
        "Knife"
    }

    fn key_down(
        &mut self,
        key: &KeyEvent,
        ctx: &mut EventCtx,
        data: &mut EditSession,
        _: &Env,
    ) -> Option<EditType> {
        if key.key_code == KeyCode::LeftShift || key.key_code == KeyCode::RightShift {
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
        if key.key_code == KeyCode::LeftShift || key.key_code == KeyCode::RightShift {
            self.shift_locked = false;
            self.update_intersections(data);
            ctx.request_paint();
        }
        None
    }
    fn init_mouse(&mut self, mouse: &mut Mouse) {
        mouse.min_drag_distance = 0.0;
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

            ctx.stroke_styled(line, &Color::BLACK, 1.0, &self.stroke_style);

            for hit in &self.intersections {
                let point = data.viewport.to_screen(hit.point);
                let cut_mark_start = point - (unit_vec * 4.0);
                let cut_mark_end = point + (unit_vec * 4.0);
                let cut_mark = Line::new(cut_mark_start, cut_mark_end);
                ctx.stroke(cut_mark, &Color::rgb(0.7, 0., 0.), 2.0);

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
            let pt = data.viewport.from_screen(event.pos);
            self.gesture = GestureState::Begun {
                start: pt,
                current: pt,
            };
            self.shift_locked = event.mods.shift;
        }
    }

    fn left_drag_ended(&mut self, drag: Drag, data: &mut EditSession) {
        if let GestureState::Begun { current, .. } = &mut self.gesture {
            let now = data.viewport.from_screen(drag.current.pos);
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

        //if !self.intersections.is_empty() {
        //let new_paths = slice_paths(&data.paths, &self.intersections);
        //}
        self.gesture = GestureState::Finished;
    }

    //fn left_drag_began(&mut self, event: Drag, data: &mut EditSession) {
    //if let GestureState::Down(start) = self.gesture {
    //let current = data.viewport.from_screen(event.current.pos);
    //self.gesture = GestureState::Begun { start, current };
    //}
    //}

    fn left_drag_changed(&mut self, drag: Drag, data: &mut EditSession) {
        if let GestureState::Begun { current, .. } = &mut self.gesture {
            *current = data.viewport.from_screen(drag.current.pos);
            self.update_intersections(data);
        }
    }
}

impl Default for GestureState {
    fn default() -> Self {
        GestureState::Ready
    }
}

impl PartialEq for Intersection {
    fn eq(&self, other: &Self) -> bool {
        self.point == other.point
            && self.seg == other.seg
            && self.hit.line_t == other.hit.line_t
            && self.hit.segment_t == other.hit.segment_t
    }
}

impl Eq for Intersection {}

impl Ord for Intersection {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.seg.start_id().parent, self.hit.line_t)
            .partial_cmp(&(other.seg.start_id().parent, self.hit.line_t))
            .unwrap()
    }
}

impl PartialOrd for Intersection {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
        eprintln!("#####original path {} ######", path.id());
        slice_path(path, line, &mut out, 0);
        eprintln!("##### {} new paths #####", out.len());
    }
    out
}

#[derive(Clone, Copy)]
struct Hit {
    intersection: LineIntersection,
    point: Point,
    seg: PathSeg,
}

impl Hit {
    fn new(line: Line, intersection: LineIntersection, seg: PathSeg) -> Self {
        let point = line.eval(intersection.line_t);
        Hit {
            intersection,
            point,
            seg,
        }
    }
}

const MAX_RECURSE: usize = 2;
// an amount of `t` we insert between slice 'segments', so that after finishing
// a first slice we don't accidentally count the end of the previous slice as
// the start of a new one.
const SLICE_EP: f64 = 1e-9;
/// Slice a path with a line.
///
/// Resulting paths are pushed to the `acc` vec.
///
/// If no modifications are made, the source `path` should still be pushed to `acc`.
fn slice_path(path: &Path, line: Line, acc: &mut Vec<Path>, recurse: usize) {
    eprintln!("slicing path {}", path.id());
    let mut intersections = path
        .iter_segments()
        .flat_map(|seg| {
            seg.to_kurbo()
                .intersect_line(line)
                .into_iter()
                .map(move |hit| Hit::new(line, hit, seg))
        })
        .collect::<Vec<_>>();

    if intersections.is_empty() || recurse == MAX_RECURSE {
        acc.push(path.to_owned());
        return;
    }

    intersections.sort_by(|a, b| {
        a.intersection
            .line_t
            .partial_cmp(&b.intersection.line_t)
            .unwrap()
    });
    // we now have a non-zero number of intersections.
    let start = intersections[0];
    let end = match intersections.get(1) {
        Some(thing) => *thing,
        //TODO: in case of only one intersection, add a point to the line
        None => {
            acc.push(path.to_owned());
            return;
        }
    };

    eprintln!(
        "{} intersections, slicing {:.1}..{:.1}",
        intersections.len(),
        start.point,
        end.point
    );

    let next_line_start_t = end.intersection.line_t + SLICE_EP;

    eprintln!("preorder {} {}", start.seg.start_id(), end.seg.start_id());
    let (start, end) = order_points(path, start, end);
    eprintln!("{} {}", start.seg.start_id(), end.seg.start_id());
    let path_one = slice_first_path(path, start, end);
    let path_two = slice_second_path(path, start, end);
    eprintln!("new paths {} and {}", path_one.id(), path_two.id());
    let line = line.subsegment(next_line_start_t..1.0);
    slice_path(&path_one, line, acc, recurse + 1);
    slice_path(&path_two, line, acc, recurse + 1);
}

/// The 'first' path includes the paths original start point, and may be open.
fn slice_first_path(path: &Path, start: Hit, end: Hit) -> Path {
    let mut points = Vec::new();
    let mut iter = path.iter_segments();
    let mut start_seg = None;

    //eprintln!("slicing {} to {}", start.seg.start_id(), end.seg.start_id());
    for seg in &mut iter {
        eprintln!("seg {:?}", seg);
        // just copy over all points up to our first cut
        if seg.start_id() != start.seg.start_id() {
            append_all_points(&mut points, seg);
            eprintln!("appending all, len {}", points.len());
        } else {
            let (cut_t, _dst) = seg.to_kurbo().nearest(start.point, 0.1);
            append_clipped_segment(&mut points, path.id(), seg, 0.0, cut_t);
            eprintln!("appending truncated, len {}", points.len());
            assert!(_dst <= 0.1, "total sanity check");
            start_seg = Some(seg);
            break;
        }
    }

    let mut iter = start_seg.iter().copied().chain(iter);
    for seg in &mut iter {
        eprintln!("seg {:?} {}", seg, points.len());
        if seg.start_id() == end.seg.start_id() {
            let (cut_t, _dst) = seg.to_kurbo().nearest(end.point, 0.1);
            append_clipped_segment(&mut points, path.id(), seg, cut_t, 1.0);
            eprintln!("appending clipped, len {}", points.len());
            break;
        }
    }

    // and finally append all remaining segments:
    iter.inspect(|seg| eprintln!("appending {:?}, seg", seg))
        .for_each(|seg| append_all_points(&mut points, seg));
    points.iter_mut().for_each(|p| p.id.parent = path.id());

    if points.first().map(|p| p.point) == points.last().map(|p| p.point) {
        points.pop();
    }
    if path.is_closed() {
        points.rotate_left(1);
    }
    Path::from_raw_parts(path.id(), points, None, path.is_closed())
}

/// The 'second' path does not include the start point, and is always closed.
fn slice_second_path(path: &Path, start: Hit, end: Hit) -> Path {
    let path_id = crate::path::next_id();
    let mut points = Vec::new();
    let mut iter = path.iter_segments();
    let mut done = false;

    for seg in &mut iter {
        eprintln!("seg {:?}", seg);
        // ignore all points to the first cut
        if seg.start_id() != start.seg.start_id() {
            continue;
        } else {
            let (cut_t, _dst) = seg.to_kurbo().nearest(start.point, 0.1);
            assert!(_dst <= 0.1, "total sanity check");
            let end_t = if seg.start_id() == end.seg.start_id() {
                done = true;
                seg.to_kurbo().nearest(end.point, 0.1).0
            } else {
                1.0
            };
            append_clipped_segment(&mut points, path_id, seg, cut_t, end_t);
            if !path.is_closed() {
            // add the cut line
                points.push(PathPoint::on_curve(path_id, DPoint::from_raw(start.point)));
            }
            break;
        }
    }

    if !done {
        for seg in iter {
            if seg.start_id() != end.seg.start_id() {
                append_all_points(&mut points, seg);
            } else {
                let end_t = seg.to_kurbo().nearest(end.point, 0.1).0;
                append_clipped_segment(&mut points, path_id, seg, 0.0, end_t);
                break;
            }
        }
    }

    points.iter_mut().for_each(|p| p.id.parent = path_id);
    points.rotate_left(1);
    Path::from_raw_parts(path_id, points, None, true)
}

fn append_all_points(dest: &mut Vec<PathPoint>, seg: PathSeg) {
    match seg {
        PathSeg::Line(one, two) => {
            if dest.last().map(|p| p.point) != Some(one.point) {
                dest.push(one);
            }
            dest.push(two);
        }
        PathSeg::Cubic(a, b, c, d) => {
            if dest.last().map(|p| p.point) != Some(a.point) {
                dest.push(a);
            }
            dest.extend(&[b, c, d]);
        }
    }
}

// clip the segment from time `t1..t2`.
fn append_clipped_segment(
    dest: &mut Vec<PathPoint>,
    path_id: usize,
    seg: PathSeg,
    t1: f64,
    t2: f64,
) {
    match seg.to_kurbo().subsegment(t1..t2) {
        KurboPathSeg::Line(Line { p0, p1 }) => {
            if dest.last().map(|p| p.point.to_raw()) != Some(p0) {
                let one = PathPoint::on_curve(path_id, DPoint::from_raw(p0));
                dest.push(one);
            }
            let two = PathPoint::on_curve(path_id, DPoint::from_raw(p1));
            dest.push(two);
        }
        KurboPathSeg::Cubic(CubicBez { p0, p1, p2, p3 }) => {
            if dest.last().map(|p| p.point.to_raw()) != Some(p0) {
                let one = PathPoint::on_curve(path_id, DPoint::from_raw(p0));
                dest.push(one);
            }
            let b = PathPoint::off_curve(path_id, DPoint::from_raw(p1));
            let c = PathPoint::off_curve(path_id, DPoint::from_raw(p2));
            let d = PathPoint::on_curve(path_id, DPoint::from_raw(p3));
            dest.extend(&[b, c, d]);
        }
        _ => (),
    }
}

/// order our two cut points based on the order of points in the path.
///
/// this simplifies the two slice functions, since they can assume they will hit
/// the start point first while iterating.
fn order_points(path: &Path, start: Hit, end: Hit) -> (Hit, Hit) {
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
        path.append_point(DPoint::new(0., 0.));
        path.append_point(DPoint::new(20., 0.));
        path.append_point(DPoint::new(10., 10.0));

        let line = Line::new((3., 6.), (8., -2.));
        let mut out = Vec::new();
        slice_path(&path, line, &mut out, 0);

        assert_eq!(out.len(), 2);
        let one = &out[0];
        let two = &out[1];

        let one_segs = one
            .iter_segments()
            .map(PathSeg::to_kurbo)
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
            .map(PathSeg::to_kurbo)
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

        let path = Path::from_bezpath(bez).unwrap();


     // first try slicing a non-first segment
        let slice_line1 = Line::new((10., 20.), (25., 10.));
        let slice_line2 = Line::new((25., 10.), (10., 20.));

        let mut out = Vec::new();
        slice_path(&path, slice_line1, &mut out, 0);
        let first = out.clone();
        out.clear();
        slice_path(&path, slice_line2, &mut out, 0);
        let second = out;
        assert_eq!(first.len(), 2);
        assert_eq!(second.len(), 2);

        assert_equal_points!(first[0], second[0]);
        assert_equal_points!(first[1], second[1]);

        // then try slicing the first segment
        let slice_line1 = Line::new((0., 10.), (10., 0.));
        let slice_line2 = Line::new((10., 0.), (0., 10.));

        let mut out = Vec::new();
        slice_path(&path, slice_line1, &mut out, 0);
        eprintln!("\n$$$$\n");
        let first = out.clone();
        out.clear();
        slice_path(&path, slice_line2, &mut out, 0);
        //panic!("awww");
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

        let path = Path::from_bezpath(bez).unwrap();
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
