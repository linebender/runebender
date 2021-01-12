use druid::kurbo::{Affine, Shape, TranslateScale};
use druid::widget::prelude::*;
use druid::{Color, Data, KeyOrValue};

use crate::data::GlyphDetail;
use crate::theme;

/// A widget that draws a glyph.
pub struct GlyphPainter {
    color: KeyOrValue<Color>,
    placeholder_color: KeyOrValue<Color>,
    draw_frame: bool,
}

impl GlyphPainter {
    pub fn new() -> Self {
        GlyphPainter {
            color: theme::PRIMARY_TEXT_COLOR.into(),
            placeholder_color: theme::PLACEHOLDER_GLYPH_COLOR.into(),
            draw_frame: false,
        }
    }

    pub fn color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.color = color.into();
        self
    }

    pub fn draw_layout_frame(mut self, draw_frame: bool) -> Self {
        self.draw_frame = draw_frame;
        self
    }
}

impl Widget<GlyphDetail> for GlyphPainter {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut GlyphDetail, _env: &Env) {}
    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &GlyphDetail,
        _env: &Env,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &GlyphDetail, data: &GlyphDetail, _env: &Env) {
        if !old.outline.same(&data.outline)
            || old.glyph.advance != data.glyph.advance
            || (ctx.env_changed()
                && (ctx.env_key_changed(&self.color)
                    || ctx.env_key_changed(&self.placeholder_color)))
        {
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &GlyphDetail,
        _env: &Env,
    ) -> Size {
        let glyph_layout_bounds = data.layout_bounds();
        let aspect_ratio = glyph_layout_bounds.aspect_ratio();
        let width = bc.max().width;
        let size = bc.constrain_aspect_ratio(aspect_ratio, width);
        let scale = size.width / glyph_layout_bounds.width();
        let inking_rect = TranslateScale::scale(scale) * data.outline.bounding_box();
        let paint_insets = inking_rect - glyph_layout_bounds;
        let baseline = glyph_layout_bounds.min_y().abs() * scale;
        ctx.set_paint_insets(paint_insets);
        ctx.set_baseline_offset(baseline);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GlyphDetail, env: &Env) {
        let glyph_bounds = data.layout_bounds();
        let paint_rect = ctx.size().to_rect();
        let scale = paint_rect.height() as f64 / glyph_bounds.height();
        let baseline = glyph_bounds.max_y().abs() * scale;
        let affine = Affine::new([scale as f64, 0.0, 0.0, -scale as f64, 0.0, baseline]);

        let glyph_color = if data.is_placeholder_glyph() {
            self.placeholder_color.resolve(env)
        } else {
            self.color.resolve(env)
        };

        if self.draw_frame {
            let frame_rect = (glyph_bounds.size() * scale).to_rect();
            ctx.stroke(frame_rect, &glyph_color, 0.5);
        }
        ctx.fill(affine * &*data.outline, &glyph_color);
    }
}

impl Default for GlyphPainter {
    fn default() -> Self {
        GlyphPainter::new()
    }
}
