//! encoding and decoding paths for use with the clipboard.

use std::collections::HashMap;
use std::fmt::Write;

use druid::kurbo::{Affine, BezPath, PathEl, Point, Rect, Shape};

use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream};

use crate::cubic_path::CubicPath;
use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::path::Path;
use crate::plist::Plist;
use crate::point::{EntityId, PathPoint, PointType};

//FIXME:
// this is all poorly done, especially for copy, where each clipboard format
// duplicates work and allocations when generating the paths to copy.
// I think there should be a single top-level function that returns a
// Vec<ClipboardFormat>, and handles only generating the paths once, passing
// them down to each encoding step.

/// Generates druid-compatible drawing code for all of the `Paths` in this
/// session, if any exist.
pub fn make_code_string(session: &EditSession) -> Option<String> {
    fn append_path(path: &BezPath, out: &mut String) -> std::fmt::Result {
        out.push('\n');
        for element in path.elements() {
            match element {
                PathEl::MoveTo(p) => writeln!(out, "bez.move_to(({:.1}, {:.1}));", p.x, p.y)?,
                PathEl::LineTo(p) => writeln!(out, "bez.line_to(({:.1}, {:.1}));", p.x, p.y)?,
                PathEl::QuadTo(p1, p2) => writeln!(
                    out,
                    "bez.quad_to(({:.1}, {:.1}), ({:.1}, {:.1}));",
                    p1.x, p1.y, p2.x, p2.y
                )?,
                PathEl::CurveTo(p1, p2, p3) => writeln!(
                    out,
                    "bez.curve_to(({:.1}, {:.1}), ({:.1}, {:.1}), ({:.1}, {:.1}));",
                    p1.x, p1.y, p2.x, p2.y, p3.x, p3.y
                )?,
                PathEl::ClosePath => writeln!(out, "bez.close_path();")?,
            }
        }
        Ok(())
    }

    if session.paths.is_empty() {
        return None;
    }

    let mut out = String::from("let mut bez = BezPath::new();\n");

    let paths = session
        .paths
        .iter()
        .map(|p| {
            let mut path = p.bezier();
            // glyphs are y-up, but piet generally expects y-down, so we flipy that
            path.apply_affine(Affine::FLIP_Y);
            path
        })
        .collect::<Vec<_>>();

    // we want to zero everything to a common origin.
    let origin = paths
        .iter()
        .map(|p| p.bounding_box().origin())
        .fold(Point::new(f64::MAX, f64::MAX), |acc, pt| {
            Point::new(acc.x.min(pt.x), acc.y.min(pt.y))
        })
        .to_vec2();

    for mut bezier in paths {
        bezier.apply_affine(Affine::translate(-origin));
        if let Err(e) = append_path(&bezier, &mut out) {
            log::error!("error generating code string: '{}'", e);
            return None;
        }
    }

    Some(out)
}

pub fn make_json(session: &EditSession) -> Option<String> {
    let paths: Vec<_> = session.paths_for_selection();
    serde_json::to_string(&paths).ok()
}

pub fn from_json(json: &str) -> Option<Vec<Path>> {
    serde_json::from_str(json).ok()
}

pub fn make_glyphs_plist(session: &EditSession) -> Option<Vec<u8>> {
    let paths: Vec<_> = session
        .paths_for_selection()
        .iter()
        .filter_map(|path| match path {
            Path::Cubic(path) => Some(GlyphPlistPath::from(path)),
            Path::Hyper(_) => None,
        })
        .collect();
    if paths.is_empty() {
        return None;
    }

    let plist = GlyphsPastePlist {
        glyph: session.name.to_string(),
        layer: String::new(),
        paths,
    };

    let mut data = Vec::new();
    if let Err(e) = plist::to_writer_binary(&mut data, &plist) {
        log::error!("failed to write plist '{}'", e);
        return None;
    }
    Some(data)
}

pub fn from_glyphs_plist(data: Vec<u8>) -> Option<Vec<Path>> {
    let cursor = std::io::Cursor::new(data);
    match plist::from_reader(cursor) {
        Ok(GlyphsPastePlist { paths, .. }) => {
            Some(paths.iter().map(|p| CubicPath::from(p).into()).collect())
        }
        Err(e) => {
            log::warn!("failed to parse glyphs plist: '{}'", e);
            None
        }
    }
}

pub fn from_glyphs_plist_string(text: String) -> Option<Vec<Path>> {
    let plist = match Plist::parse(&text) {
        Ok(Plist::Dictionary(d)) => d,
        Ok(other) => {
            log::warn!("unexpected plist value {:?}", other);
            return None;
        }
        Err(e) => {
            log::warn!("failed to parse string plist: '{:?}'", e);
            return None;
        }
    };
    paths_from_plist_dict(plist)
}

fn paths_from_plist_dict(dict: HashMap<String, Plist>) -> Option<Vec<Path>> {
    let paths = dict.get("paths").and_then(Plist::as_array)?;
    let mut result = Vec::new();
    for path in paths {
        if let Plist::Dictionary(dict) = path {
            if let Some(path) = GlyphPlistPath::from_dict(dict)
                .as_ref()
                .map(CubicPath::from)
            {
                result.push(path.into());
            }
        }
    }
    Some(result)
}

/// Attempt to generate a minimal PDF representation of the current session,
/// for use on the system pasteboard.
pub fn make_pdf_data(session: &EditSession) -> Option<Vec<u8>> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut ops = Vec::new();
    let mut rect = Rect::ZERO;

    for path in session.paths_for_selection() {
        let bezier = path.bezier();
        rect = rect.union(bezier.bounding_box());
        append_pdf_ops(&mut ops, &bezier);
    }

    ops.push(Operation::new("f", vec![]));

    let content = Content { operations: ops };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
    });

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
        "MediaBox" => vec![rect.x0.into(), rect.y0.into(), rect.x1.into(), rect.y1.into()],
    };

    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });

    doc.trailer.set("Root", catalog_id);
    doc.compress();
    let mut out = Vec::new();
    if let Err(e) = doc.save_to(&mut out) {
        log::warn!("error writing pdf for clipboard: '{}'", e);
        None
    } else {
        Some(out)
    }
}

pub fn from_pdf_data(data: Vec<u8>) -> Option<Vec<Path>> {
    match Document::load_mem(&data) {
        Ok(doc) => {
            if doc.get_pages().len() > 1 {
                log::warn!("pasted pdf has multiple pages, we will only look at the first.");
            }
            let page = doc.page_iter().next()?;
            let content = doc
                .get_and_decode_page_content(page)
                .map_err(|e| log::warn!("failed to decode pdf content: '{}'", e))
                .ok()?;
            Some(paths_for_pdf_contents(content))
        }
        Err(e) => {
            log::warn!("failed to load pdf data: '{}'", e);
            None
        }
    }
}

fn append_pdf_ops(ops: &mut Vec<Operation>, path: &BezPath) {
    for element in path.elements() {
        let op = match element {
            PathEl::MoveTo(p) => Operation::new("m", vec![p.x.into(), p.y.into()]),
            PathEl::LineTo(p) => Operation::new("l", vec![p.x.into(), p.y.into()]),
            PathEl::QuadTo(_p1, _p2) => {
                //FIXME: should we convert quads to cubes?
                log::warn!("pdf copy does not support quadratic beziers!");
                continue;
            }
            PathEl::CurveTo(p1, p2, p3) => Operation::new(
                "c",
                vec![
                    p1.x.into(),
                    p1.y.into(),
                    p2.x.into(),
                    p2.y.into(),
                    p3.x.into(),
                    p3.y.into(),
                ],
            ),
            PathEl::ClosePath => Operation::new("h", vec![]),
        };
        ops.push(op);
    }
}

// pdf paths have some weird duplication thing going on?
fn paths_for_pdf_contents(contents: Content) -> Vec<Path> {
    //contents.operations.iter().for_each(|op| eprintln!("{}: [{:?}]", op.operator, op.operands));
    let bez = bez_path_for_pdf_contents(contents);
    let mut result = Vec::new();
    for path in iter_paths_for_bez_path(&bez) {
        if !result.last().map(|p| approx_eq(p, &path)).unwrap_or(false) {
            result.push(path)
        }
    }
    result.into_iter().map(Path::from).collect()
}

//HACK: in some instances, at least on mac, a PDF on the pasteboard will have
//two different paths for each of fill and stroke; we try to deduplicate that.
fn approx_eq(path1: &CubicPath, path2: &CubicPath) -> bool {
    // that's pretty close in my opinion
    const ARBITRARY_DISTANCE_THRESHOLD: f64 = 0.00001;
    path1.path_points().len() == path2.path_points().len()
        && path1
            .path_points()
            .iter_points()
            .zip(path2.path_points().iter_points())
            .all(|(p1, p2)| {
                p1.typ == p2.typ && (p1.point - p2.point).hypot() < ARBITRARY_DISTANCE_THRESHOLD
            })
}

// going to unjustifiable lengths to avoid an unecessary allocation :|
fn iter_paths_for_bez_path(src: &BezPath) -> impl Iterator<Item = CubicPath> + '_ {
    let mut cur_path_id = EntityId::next();
    let mut cur_points = Vec::new();
    let mut closed = false;
    let mut iter = src.elements().iter();

    std::iter::from_fn(move || loop {
        let path_el = match iter.next() {
            Some(el) => el,
            None => {
                if !cur_points.is_empty() {
                    let points = std::mem::take(&mut cur_points);
                    return Some(CubicPath::from_raw_parts(cur_path_id, points, None, closed));
                }
                return None;
            }
        };

        match path_el {
            PathEl::MoveTo(pt) => {
                let points = std::mem::take(&mut cur_points);
                let path = if points.is_empty() {
                    None
                } else {
                    Some(CubicPath::from_raw_parts(cur_path_id, points, None, closed))
                };
                cur_path_id = EntityId::next();
                closed = false;
                cur_points.push(PathPoint::on_curve(cur_path_id, DPoint::from_raw(*pt)));
                if let Some(path) = path {
                    return Some(path);
                }
            }
            PathEl::LineTo(pt) => {
                cur_points.push(PathPoint::on_curve(cur_path_id, DPoint::from_raw(*pt)))
            }
            PathEl::QuadTo(..) => log::warn!("ignoring quad_to in paste"),
            PathEl::CurveTo(p1, p2, p3) => {
                cur_points.push(PathPoint::off_curve(cur_path_id, DPoint::from_raw(*p1)));
                cur_points.push(PathPoint::off_curve(cur_path_id, DPoint::from_raw(*p2)));
                cur_points.push(PathPoint::on_curve(cur_path_id, DPoint::from_raw(*p3)));
            }
            PathEl::ClosePath => closed = true,
        }
    })
}

fn bez_path_for_pdf_contents(contents: Content) -> BezPath {
    let mut bez = BezPath::new();
    let mut transform = Affine::default();

    for op in contents.operations.into_iter() {
        let Operation { operator, operands } = op;
        match (operator.as_str(), operands.as_slice()) {
            (op, args) if ["m", "l", "c", "h"].contains(&op) => {
                if let Some(el) = path_el_for_operation(op, args) {
                    bez.push(transform * el);
                }
            }
            ("cm", args) => {
                if let Some(affine) = affine_for_args(args) {
                    transform *= affine;
                }
            }
            (other, _) => log::warn!("unhandled pdf operation '{}'", other),
        }
    }
    bez
}

fn affine_for_args(args: &[lopdf::Object]) -> Option<Affine> {
    fn float(x: &lopdf::Object) -> Option<f64> {
        x.as_f64().or_else(|_| x.as_i64().map(|i| i as f64)).ok()
    }
    if args.len() == 6 {
        let mut coeffs: [f64; 6] = [0.0; 6];
        for (i, arg) in args.iter().enumerate() {
            coeffs[i] = float(arg)?;
        }
        Some(Affine::new(coeffs))
    } else {
        None
    }
}

fn path_el_for_operation(op: &str, operands: &[lopdf::Object]) -> Option<PathEl> {
    fn pt(x_: &lopdf::Object, y_: &lopdf::Object) -> Option<Point> {
        let x = x_
            .as_f64()
            .or_else(|_| x_.as_i64().map(|i| i as f64))
            .map_err(|e| log::warn!("bad point x val in '{:?}': '{}'", x_, e));
        let y = y_
            .as_f64()
            .or_else(|_| y_.as_i64().map(|i| i as f64))
            .map_err(|e| log::warn!("bad point y val in '{:?}': '{}'", y_, e));
        if let (Ok(x), Ok(y)) = (x, y) {
            Some(Point::new(x, y))
        } else {
            None
        }
    }

    match (op, operands) {
        ("m", &[ref x, ref y]) => Some(PathEl::MoveTo(pt(x, y)?)),
        ("l", &[ref x, ref y]) => Some(PathEl::LineTo(pt(x, y)?)),
        ("c", &[ref x1, ref y1, ref x2, ref y2, ref x3, ref y3]) => {
            Some(PathEl::CurveTo(pt(x1, y1)?, pt(x2, y2)?, pt(x3, y3)?))
        }
        ("h", &[]) => Some(PathEl::ClosePath),
        (other, _) => {
            log::warn!("unhandled pdf operation '{}'", other);
            None
        }
    }
}

pub fn make_svg_data(session: &EditSession) -> Option<Vec<u8>> {
    use svg::node::element::path::Data;
    use svg::node::element::Path;
    use svg::Document;

    let mut bbox = Rect::ZERO;
    let mut data = Data::new();

    for path in session.paths_for_selection() {
        let bezier = path.bezier();
        bbox = bbox.union(bezier.bounding_box());
        for element in bezier.elements() {
            data = match element {
                PathEl::MoveTo(p) => data.move_to((p.x, p.y)),
                PathEl::LineTo(p) => data.line_to((p.x, p.y)),
                PathEl::QuadTo(p1, p2) => data.quadratic_curve_to((p1.x, p1.y, p2.x, p2.y)),
                PathEl::CurveTo(p1, p2, p3) => {
                    data.cubic_curve_to((p1.x, p1.y, p2.x, p2.y, p3.x, p3.y))
                }
                PathEl::ClosePath => data.close(),
            };
        }
    }

    let path = Path::new()
        .set("fill", "none")
        .set("stroke", "black")
        .set("stroke-width", 1)
        .set("d", data);

    let document = Document::new()
        .set("viewBox", (bbox.x0, bbox.y0, bbox.x1, bbox.y1))
        .add(path);

    let mut data = Vec::new();
    if let Err(e) = svg::write(&mut data, &document) {
        log::warn!("error writing svg data: '{}'", e);
        None
    } else {
        Some(data)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct GlyphsPastePlist {
    glyph: String,
    layer: String,
    paths: Vec<GlyphPlistPath>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GlyphPlistPath {
    closed: u32,
    nodes: Vec<String>,
}

impl GlyphPlistPath {
    fn from_dict(dict: &HashMap<String, Plist>) -> Option<Self> {
        let closed = dict.get("closed").and_then(Plist::as_i64)? as u32;
        let nodes = dict.get("nodes").and_then(Plist::as_array)?;
        let nodes = nodes
            .iter()
            .flat_map(|pl| match pl {
                Plist::String(s) => Some(s.to_owned()),
                _ => None,
            })
            .collect();

        Some(GlyphPlistPath { closed, nodes })
    }
}

impl From<&CubicPath> for GlyphPlistPath {
    fn from(src: &CubicPath) -> GlyphPlistPath {
        let mut next_is_curve = src
            .path_points()
            .as_slice()
            .last()
            .map(|p| p.is_off_curve())
            .unwrap_or(false);
        let nodes = src
            .path_points()
            .as_slice()
            .iter()
            .map(|p| {
                let ptyp = match p.typ {
                    PointType::OnCurve { smooth: false } if next_is_curve => "CURVE",
                    PointType::OnCurve { smooth: false } => "LINE",
                    PointType::OnCurve { smooth: true } => "CURVE SMOOTH",
                    PointType::OffCurve { .. } => "OFFCURVE",
                };

                next_is_curve = p.is_off_curve();

                format!("\"{} {} {}\"", p.point.x, p.point.y, ptyp)
            })
            .collect();
        let closed = if src.is_closed() { 1 } else { 0 };
        GlyphPlistPath { closed, nodes }
    }
}

impl From<&GlyphPlistPath> for CubicPath {
    fn from(src: &GlyphPlistPath) -> CubicPath {
        let path_id = EntityId::next();
        let paths: Vec<PathPoint> = src
            .nodes
            .iter()
            .flat_map(|node| from_glyphs_plist_point(node, path_id))
            .collect();
        CubicPath::from_raw_parts(path_id, paths, None, src.closed > 0)
    }
}

fn from_glyphs_plist_point(s: &str, parent_id: EntityId) -> Option<PathPoint> {
    let mut iter = s.trim_matches('"').splitn(3, ' ');
    match (iter.next(), iter.next(), iter.next()) {
        (Some(x_), Some(y_), Some(typ_)) => {
            let x: f64 = x_
                .parse()
                .or_else(|_| x_.parse::<i64>().map(|i| i as f64))
                .map_err(|e| log::warn!("bad glyphs plist point x val in '{}': '{}'", x_, e))
                .ok()?;
            let y: f64 = y_
                .parse()
                .or_else(|_| y_.parse::<i64>().map(|i| i as f64))
                .map_err(|e| log::warn!("bad glyphs plist point y val in '{}': '{}'", y_, e))
                .ok()?;
            let typ = match typ_ {
                "CURVE" | "LINE" => PointType::OnCurve { smooth: false },
                "CURVE SMOOTH" => PointType::OnCurve { smooth: true },
                "OFFCURVE" => PointType::OffCurve { auto: false },
                other => {
                    log::warn!("unhandled glyphs point type '{}'", other);
                    return None;
                }
            };
            let point = DPoint::new(x.round(), y.round());
            let id = EntityId::new_with_parent(parent_id);
            Some(PathPoint { id, point, typ })
        }
        _other => {
            log::warn!("unrecognized glyphs point format: '{}'", s);
            None
        }
    }
}
