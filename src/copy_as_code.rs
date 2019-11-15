//! Converting paths to piet drawing code.

use std::fmt::Write;

use druid::kurbo::{Affine, BezPath, PathEl, Rect, Shape};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream};

use crate::edit_session::EditSession;
use crate::path::{Path, PointType};

/// Generates druid-compatible drawing code for all of the `Paths` in this
/// session, if any exist.
pub fn make_code_string(session: &EditSession) -> Option<String> {
    if session.paths.is_empty() {
        return None;
    }

    let mut out = String::from("let mut bez = BezPath::new();\n");
    for path in session.paths.iter() {
        let mut bezier = path.bezier();

        // glyphs are y-up, but piet generally expects y-down, so we flipy that
        bezier.apply_affine(Affine::FLIP_Y);

        // and then we set our origin to be equal the origin of our bounding box
        let bbox = bezier.bounding_box();
        bezier.apply_affine(Affine::translate(-bbox.origin().to_vec2()));

        if let Err(e) = append_path(&bezier, &mut out) {
            log::error!("error generating code string: '{}'", e);
            return None;
        }
    }

    Some(out)
}

pub fn make_glyphs_plist(session: &EditSession) -> Option<Vec<u8>> {
    let paths: Vec<_> = session.paths.iter().map(GlyphPlistPath::from).collect();
    if paths.is_empty() {
        return None;
    }

    let plist = GlyphsPastePlist {
        glyph: session.name.to_string(),
        layer: "",
        paths,
    };

    let mut data = Vec::new();
    if let Err(e) = plist::to_writer_binary(&mut data, &plist) {
        log::error!("failed to write plist '{}'", e);
        return None;
    }
    Some(data)
}

/// Attempt to generate a minimal PDF representation of the current session,
/// for use on the system pasteboard.
pub fn make_pdf_data(session: &EditSession) -> Option<Vec<u8>> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let mut ops = Vec::new();
    let mut rect = Rect::ZERO;

    for path in session.paths.iter() {
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

#[derive(Debug, Serialize)]
struct GlyphsPastePlist {
    glyph: String,
    layer: &'static str,
    paths: Vec<GlyphPlistPath>,
}

#[derive(Debug, Serialize)]
struct GlyphPlistPath {
    closed: u32,
    nodes: Vec<String>,
}

impl From<&Path> for GlyphPlistPath {
    fn from(src: &Path) -> GlyphPlistPath {
        let mut next_is_curve = src
            .points()
            .last()
            .map(|p| p.typ == PointType::OffCurve)
            .unwrap_or(false);
        let nodes = src
            .points()
            .iter()
            .map(|p| {
                let ptyp = match p.typ {
                    PointType::OnCurve if next_is_curve => "CURVE",
                    PointType::OnCurve => "LINE",
                    PointType::OnCurveSmooth => "CURVE SMOOTH",
                    PointType::OffCurve => "OFFCURVE",
                };

                next_is_curve = p.typ == PointType::OffCurve;

                format!("\"{} {} {}\"", p.point.x, p.point.y, ptyp)
            })
            .collect();
        let closed = if src.is_closed() { 1 } else { 0 };
        GlyphPlistPath { closed, nodes }
    }
}
