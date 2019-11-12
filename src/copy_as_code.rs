//! Converting paths to piet drawing code.

use std::fmt::Write;

use crate::edit_session::EditSession;
use druid::kurbo::{Affine, BezPath, PathEl, Shape};

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
