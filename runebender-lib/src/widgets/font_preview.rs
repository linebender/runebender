//! a widget that uses harfbuzz to preview shaping.

use druid::kurbo::Affine;
use druid::widget::prelude::*;
use harfbuzz_rs::{Blob, Face, Font, UnicodeBuffer};

use crate::data::PreviewState;
use crate::theme;
use crate::virtual_font::{GlyphId, VirtualFont};

const CMAP: [u8; 4] = [b'c', b'm', b'a', b'p'];
const HHEA: [u8; 4] = [b'h', b'h', b'e', b'a'];
const HMTX: [u8; 4] = [b'h', b'm', b't', b'x'];

#[derive(Debug, Clone, Default)]
pub struct Preview {
    virtual_font: VirtualFont,
    layout: Vec<(GlyphId, f64)>,
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
        if !old_data.same(&data) {
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
        font.set_ppem(1000, 1000);
        let buffer = UnicodeBuffer::new().add_str(data.text());
        let output = harfbuzz_rs::shape(&font, buffer, &[]);
        let positions = output.get_glyph_positions();
        let info = output.get_glyph_infos();
        self.layout.clear();
        let scale = data.font_size() / 1000.0;
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

    fn paint(&mut self, ctx: &mut PaintCtx, data: &PreviewState, env: &Env) {
        let glyph_color = env.get(theme::PRIMARY_TEXT_COLOR);
        for (glyph, pos) in &self.layout {
            if let Some(bez) = self
                .virtual_font
                .glyph_for_id(*glyph)
                .and_then(|name| data.font.get_bezier(name))
            {
                let scale = data.font_size() / 1000.0;
                //FIXME: actually calculate the baseline
                let transform = Affine::new([scale, 0., 0., -scale, *pos, data.font_size()]);
                ctx.fill(transform * &*bez, &glyph_color);
            }
        }
    }
}
