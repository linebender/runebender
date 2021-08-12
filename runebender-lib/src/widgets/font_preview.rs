//! a widget that uses harfbuzz to preview shaping.

use druid::kurbo::Affine;
use druid::widget::prelude::*;
use harfbuzz_rs::{Blob, Face, Font, GlyphBuffer, UnicodeBuffer};

use crate::data::PreviewState;
use crate::theme;
use crate::virtual_font::{GlyphId, VirtualFont};

const CMAP: [u8; 4] = [b'c', b'm', b'a', b'p'];
const HHEA: [u8; 4] = [b'h', b'h', b'e', b'a'];
const HMTX: [u8; 4] = [b'h', b'm', b't', b'x'];

#[derive(Debug, Default)]
pub struct Preview {
    virtual_font: VirtualFont,
    layout: Vec<Run>,
}

#[derive(Debug, Default)]
struct Run {
    // glyphs + advances
    glyphs: Vec<(GlyphId, i32)>,
    // the total width of the run in design points
    width: i32,
}

impl Run {
    fn new(hb_output: &GlyphBuffer) -> Self {
        let info = hb_output.get_glyph_infos();
        let positions = hb_output.get_glyph_positions();
        let mut pos = 0;
        let mut glyphs = Vec::with_capacity(info.len());
        for (info, position) in info.iter().zip(positions.iter()) {
            glyphs.push((info.codepoint as u16, pos + position.x_offset));
            pos += position.x_advance;
        }
        Run { glyphs, width: pos }
    }
}

impl Widget<PreviewState> for Preview {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut PreviewState, _env: &Env) {
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &PreviewState,
        _env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) {
            self.virtual_font = VirtualFont::new(&data.font);
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &PreviewState,
        data: &PreviewState,
        _env: &Env,
    ) {
        if !old_data.font.same(&data.font) {
            self.virtual_font = VirtualFont::new(&data.font);
        }
        if !old_data.same(data) {
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &PreviewState,
        _env: &Env,
    ) -> Size {
        // get around borrowck
        let Preview { virtual_font, .. } = self;
        let face = Face::from_table_func(|tag| match tag.to_bytes() {
            CMAP => Some(Blob::with_bytes(virtual_font.cmap()).to_shared()),
            HHEA => Some(Blob::with_bytes(virtual_font.hhea()).to_shared()),
            HMTX => Some(Blob::with_bytes(virtual_font.hmtx()).to_shared()),
            _ => None,
        });

        let mut font = Font::new(face);
        let upm = data.font.units_per_em();
        font.set_ppem(upm as u32, upm as u32);
        let mut reuseable_buffer = None;
        self.layout.clear();
        for line in data.text().lines() {
            let buffer = reuseable_buffer
                .take()
                .unwrap_or_else(UnicodeBuffer::new)
                .add_str(line);
            let output = harfbuzz_rs::shape(&font, buffer, &[]);
            self.layout.push(Run::new(&output));
            reuseable_buffer = Some(output.clear());
        }
        let width = self
            .layout
            .iter()
            .map(|run| run.width)
            .max()
            .unwrap_or_default();
        bc.constrain((width as f64, bc.max().height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &PreviewState, env: &Env) {
        let glyph_color = env.get(theme::PRIMARY_TEXT_COLOR);
        let font_size = data.font_size();
        let scale = font_size / data.font.units_per_em();
        for (line_n, run) in self.layout.iter().enumerate() {
            let y_pos = (line_n + 1) as f64 * font_size;
            for (glyph, pos) in &run.glyphs {
                if let Some(bez) = self
                    .virtual_font
                    .glyph_for_id(*glyph)
                    .and_then(|name| data.font.get_bezier(name))
                {
                    //FIXME: actually calculate the baseline
                    let transform =
                        Affine::new([scale, 0., 0., -scale, *pos as f64 * scale, y_pos]);
                    ctx.fill(transform * &*bez, &glyph_color);
                }
            }
        }
    }
}
