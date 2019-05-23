// Copyright 2019 the Runebender authors.

//! A quick demonstration of loading and displaying a UFO glyph.

use kurbo::{Affine, BezPath, Circle, Line, Vec2};
use norad::glyph::{Contour, ContourPoint, Glyph, PointType};
use piet::{FillRule, RenderContext};

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::{
    BoxConstraints, Geometry, Id, LayoutCtx, LayoutResult, PaintCtx, Ui, UiMain, UiState, Widget,
};

struct GlyphEditor {
    glyph: Glyph,
    path: BezPath,
    height: f32,
}

impl GlyphEditor {
    fn new(glyph: Glyph) -> Self {
        // assume glyph height is 1000 or 4000 'units'
        let height = if glyph.outline.as_ref()
            .map(|o| o.contours.iter()
                 .flat_map(|c| c.points.iter().map(|p| p.y))
                 .any(|h| h > 1000.))
            .unwrap_or(false) { 4000. } else { 1000. };

        let path = glyph.outline.as_ref().map(|o| make_path(&o.contours)).unwrap_or_default();
        GlyphEditor { glyph, height, path }
    }

    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

impl Widget for GlyphEditor {
    fn paint(&mut self, ctx: &mut PaintCtx, geom: &Geometry) {
        let baseline = (geom.size.1 * 0.66) as f64;
        let l_pad = 100.;

        let baseline_fg = ctx.render_ctx.solid_brush(0x00_80_f0_ff).unwrap();
        let fg = ctx.render_ctx.solid_brush(0xf0_f0_ea_ff).unwrap();
        let control_point_fg = ctx.render_ctx.solid_brush(0x70_80_7a_ff).unwrap();

        let scale = (geom.size.1 / self.height * 0.65).min(1.0).max(0.2);
        println!("scale {}", scale);
        let affine = Affine::new([scale as f64, 0.0, 0.0, -scale as f64, l_pad, baseline]);

        let line = Line::new((0., baseline), (geom.size.0 as f64, baseline));
        ctx.render_ctx.stroke(line, &baseline_fg, 0.5, None);
        ctx.render_ctx.stroke(affine * &self.path, &fg, 1.0, None);

        for shape in self.glyph.outline.as_ref().iter().map(|o| o.contours.iter()).flatten() {
            for point in shape.points.iter() {
                println!("{:?}", point);
                let color = if point.typ == PointType::OffCurve { &control_point_fg } else { &fg };
                let circ = Circle::new((l_pad + (point.x * scale) as f64, (baseline - (point.y * scale) as f64)), 8.0 * scale as f64);
                ctx.render_ctx.fill(circ, color, FillRule::NonZero);
            }
        }
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((100.0, 100.0)))
    }
}

fn build_ui(ui: &mut UiState, glyph: Glyph) {
    let root_id = GlyphEditor::new(glyph).ui(ui);
    ui.set_root(root_id);
}

fn main() {
    let glyph_path = match std::env::args().skip(1).next() {
        Some(arg) => arg,
        None => {
            eprintln!("Please pass a path to a .glif file");
            std::process::exit(1);
        }
    };

    println!("loading {}", glyph_path);
    let glyph = norad::Glyph::load(&glyph_path).expect("failed to load glyph");

    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();

    build_ui(&mut state, glyph);

    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Ufo Toy");
    let window = builder.build().expect("building window");

    window.show();
    run_loop.run();
}

fn make_path(contours: &[Contour]) -> BezPath {
    /// An outline can have multiple contours, which correspond to subpaths
    fn add_contour(path: &mut BezPath, contour: &Contour) {
        let mut close: Option<&ContourPoint> = None;

        if contour.points.is_empty() { return; }

        let first = &contour.points[0];
        path.moveto((first.x as f64, first.y as f64));
        if first.typ != PointType::Move {
            close = Some(first);
        }

        let mut idx = 1;
        let mut controls: (Option<Vec2>, Option<Vec2>) = (None, None);

        let mut add_curve = |to_point: Vec2, controls: &mut (Option<Vec2>, Option<Vec2>)| {
            match (controls.0.take(), controls.1.take()) {
                (Some(one), None) => path.quadto(one, to_point),
                (Some(one), Some(two)) => path.curveto(one, two, to_point),
                (None, None) => path.lineto(to_point),
                _illegal => panic!("existence of second point implies first"),
            }
        };

        while idx < contour.points.len() {
            let next = &contour.points[idx];
            let point: Vec2 = (next.x as f64, next.y as f64).into();
            match next.typ {
                PointType::OffCurve if controls.0.is_none() => controls.0 = Some(point.into()),
                PointType::OffCurve => controls.1 = Some(point.into()),
                PointType::Line => {
                    debug_assert!(controls.0.is_none(), "line type cannot follow offcurve");
                    add_curve(point, &mut controls);
                }
                PointType::Curve => add_curve(point, &mut controls),
                PointType::QCurve => {
                    eprintln!("TODO: handle qcurve");
                    add_curve(point, &mut controls);
                }
                PointType::Move => debug_assert!(false, "illegal move point in path?"),
            }
            idx += 1;
        }

        if let Some(to_close) = close.take() {
            add_curve((to_close.x as f64, to_close.y as f64).into(), &mut controls);
        }
    }

    let mut path = BezPath::new();
    contours.iter().for_each(|c| add_contour(&mut path, c));
    path
}
