use kurbo::{Affine, Circle, Line, BezPath};
use piet::{RenderContext, FillRule};
use norad::{glyph::PointType, Glyph};

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::widget::{ScrollEvent, Widget};
use druid::{
    BoxConstraints, Geometry, HandlerCtx, Id, LayoutCtx, LayoutResult, PaintCtx, Ui,
    UiMain, UiState,
};

struct GlyphEditor {
    glyph: Glyph,
    //path: BezPath,
    height: f32,
}

impl GlyphEditor {
    fn new(glyph: Glyph) -> Self {
        let mut path = BezPath::new();

        // assume glyph height is 1000 or 2000 units
        let height = if glyph.outline.as_ref()
            .map(|o| o.contours.iter()
                 .flat_map(|c| c.points.iter().map(|p| p.y))
                 .any(|h| h > 1000.))
            .unwrap_or(false) { 2000. } else { 1000. };

        GlyphEditor { glyph, height }
    }
}

impl Widget for GlyphEditor {
    fn paint(&mut self, ctx: &mut PaintCtx, geom: &Geometry) {
        let baseline = (geom.size.1 / 5.) as f64 * 3.;
        let l_pad = 100.;
        let baseline_fg = ctx.render_ctx.solid_brush(0x00_80_f0_ff).unwrap();

        let scale = ( geom.size.1 / self.height * 0.65).min(1.0).max(0.2);
        println!("scale {}", scale);

        let line = Line::new((0., baseline), (geom.size.0 as f64, baseline));
        ctx.render_ctx.stroke(line, &baseline_fg, 0.5, None);

        let fg = ctx.render_ctx.solid_brush(0xf0_f0_ea_ff).unwrap();
        let control_point_fg = ctx.render_ctx.solid_brush(0x70_80_7a_ff).unwrap();

        for shape in self.glyph.outline.as_ref().iter().map(|o| o.contours.iter()).flatten() {
            for point in shape.points.iter() {
                eprintln!("{:?}", point);
                let color = if point.typ == PointType::OffCurve { &control_point_fg } else { &fg };
                let circ = Circle::new((l_pad + (point.x * scale) as f64, (baseline - (point.y * scale) as f64)), 10.0 * scale as f64);
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
    //println!("loaded glyph\n{:?}", glyph);

    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();

    let root = GlyphEditor::new(glyph);
    let root_id = state.add(root, &[]);
    state.set_root(root_id);

    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Ufo Toy");
    let window = builder.build().expect("building window");

    window.show();
    run_loop.run();
}

