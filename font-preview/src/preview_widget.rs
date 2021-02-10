use crate::opentype::GlyphId;
use crate::AppData;
use druid::kurbo::Affine;
use druid::widget::prelude::*;
use druid::Color;
use harfbuzz_rs::{Blob, Face, Font, UnicodeBuffer};

const CMAP: [u8; 4] = [b'c', b'm', b'a', b'p'];
const HHEA: [u8; 4] = [b'h', b'h', b'e', b'a'];
const HMTX: [u8; 4] = [b'h', b'm', b't', b'x'];

#[derive(Debug, Clone)]
pub struct Preview {
    layout: Vec<(GlyphId, f64)>,
    font_size: f64,
}

impl Preview {
    pub fn new(font_size: f64) -> Preview {
        Preview {
            layout: Vec::new(),
            font_size,
        }
    }
}

impl Widget<AppData> for Preview {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut AppData, _env: &Env) {
        //todo!()
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &AppData,
        _env: &Env,
    ) {
        //todo!()
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &AppData, data: &AppData, _env: &Env) {
        if !old_data.same(&data) {
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppData,
        _env: &Env,
    ) -> Size {
        //bc.max()
        let face = Face::from_table_func(|tag| {
            eprintln!("{}", tag);
            match tag.to_bytes() {
                CMAP => Some(Blob::with_bytes(data.font.cmap()).to_shared()),
                HHEA => Some(Blob::with_bytes(data.font.hhea()).to_shared()),
                HMTX => Some(Blob::with_bytes(data.font.hmtx()).to_shared()),
                _ => None,
            }
        });

        let mut font = Font::new(face);
        font.set_ppem(1000, 1000);
        let buffer = UnicodeBuffer::new().add_str(&data.text);
        let output = harfbuzz_rs::shape(&font, buffer, &[]);
        let positions = output.get_glyph_positions();
        let info = output.get_glyph_infos();
        self.layout.clear();
        let scale = self.font_size / 1000.0;
        let mut pos = 0.0;
        for (info, position) in info.iter().zip(positions.iter()) {
            self.layout.push((
                info.codepoint as u16,
                pos + position.x_offset as f64 * scale,
            ));
            pos += position.x_advance as f64 * scale;
        }
        bc.constrain((pos, bc.max().height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &AppData, env: &Env) {
        for (glyph, pos) in &self.layout {
            dbg!(glyph, pos);
            if let Some(bez) = data.font.bez_path_for_glyph(*glyph) {
                //let transform = Affine::translate((*pos, 400.0)) * Affine::FLIP_Y;
                let scale = self.font_size / 1000.0;
                let transform = Affine::new([scale, 0., 0., -scale, *pos, 400.0]);
                ctx.fill(transform * &*bez, &Color::BLACK);
            }
        }
    }
}
