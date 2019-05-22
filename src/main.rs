use kurbo::Affine;
use piet::RenderContext;

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::widget::{ScrollEvent, Widget};
use druid::{
    BoxConstraints, Geometry, HandlerCtx, Id, LayoutCtx, LayoutResult, PaintCtx, Ui,
    UiMain, UiState,
};

mod font;

use font::Font;

struct GlyphList {
    font: Font,
    box_size: (f32, f32),
    scroll: f32,
}

impl Widget for GlyphList {
    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let fg = paint_ctx.render_ctx.solid_brush(0xf0_f0_ea_ff).unwrap();
        let scale = 0.001 * self.box_size.1;

        let (x0, y0) = geom.pos;

        let n_wide = self.n_wide(geom.size.0);
        let glyph_start = (self.scroll / self.box_size.1).floor() as usize * n_wide;
        let glyph_end = ((self.scroll + geom.size.1) / self.box_size.1).ceil() as usize * n_wide;
        let n_glyphs = self.font.glyphs.len();
        let glyph_start = glyph_start.min(n_glyphs);
        let glyph_end = glyph_end.min(n_glyphs);
        for i in glyph_start..glyph_end {
            if let Some(bp) = self.font.glyphs[i].as_ref() {
                let (dx, dy) = self.pos_of_ix(i, geom.size.0);
                let x = x0 + dx;
                let y = y0 + dy;
                let affine = Affine::new([
                    scale as f64,
                    0.0,
                    0.0,
                    -scale as f64,
                    x as f64,
                    (800.0 * scale + y) as f64,
                ]);
                paint_ctx.render_ctx.stroke(affine * bp, &fg, 1.0, None);
            } else {
                println!("no glyph for {}", i);
            }
        }
    }

    fn scroll(&mut self, event: &ScrollEvent, ctx: &mut HandlerCtx) {
        if event.dy != 0.0 {
            self.scroll = (self.scroll + event.dy).max(0.0);
            // TODO: cap scroll past end; requires geometry, which should be
            // available from HandlerCtx, but this is not plumbed in druid.
            ctx.invalidate();
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

impl GlyphList {
    fn new(font: Font) -> GlyphList {
        GlyphList {
            font,
            box_size: (50.0, 50.0),
            scroll: 0.0,
        }
    }

    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }

    fn n_wide(&self, width: f32) -> usize {
        (width / self.box_size.0).floor().max(1.0) as usize
    }

    fn pos_of_ix(&self, ix: usize, width: f32) -> (f32, f32) {
        let n_wide = self.n_wide(width);
        let i = ix % n_wide;
        let j = ix / n_wide;
        let x = (i as f32) * self.box_size.0;
        let y = (j as f32) * self.box_size.1 - self.scroll;
        (x, y)
    }
}

fn build_ui(ui: &mut UiState, font: Font) {
    let root = GlyphList::new(font).ui(ui);
    ui.set_root(root);
}

fn main() {
    let mut args = std::env::args();
    let _ = args.next();
    let filename = args.next().expect("expected a font file");
    let font = Font::load_from_file(&filename);

    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();
    build_ui(&mut state, font);
    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Runebender");
    let window = builder.build().expect("building window");
    window.show();
    run_loop.run();
}
