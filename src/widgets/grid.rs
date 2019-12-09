//! The top-level widget for the main glyph list window.

use druid::kurbo::{Affine, Line, Rect, Shape, Size};
use druid::piet::{
    FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder,
};
use druid::{
    theme, BaseState, BoxConstraints, Command, Data, Env, Event, EventCtx, LayoutCtx, LensWrap,
    PaintCtx, UpdateCtx, Widget, WidgetPod,
};

use crate::app_delegate::EDIT_GLYPH;
use crate::data::{lenses, GlyphPlus, GlyphSet};

pub struct GlyphGrid {
    children: Vec<WidgetPod<GlyphSet, LensWrap<GlyphPlus, lenses::glyph_set::Glyph, GridInner>>>,
}

const GLYPH_SIZE: f64 = 100.;

impl Widget<GlyphSet> for GlyphGrid {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &GlyphSet, env: &Env) {
        ctx.render_ctx.clear(env.get(theme::BACKGROUND_LIGHT));
        let row_len = 1.0_f64.max(state.size().width / GLYPH_SIZE).floor() as usize;
        let row_count = if self.children.is_empty() {
            0
        } else {
            self.children.len() / row_len + 1
        };

        for row in 0..row_count {
            let baseline = row as f64 * GLYPH_SIZE + GLYPH_SIZE * (1.0 - 0.16);
            let line = Line::new((0., baseline), (state.size().width + GLYPH_SIZE, baseline));
            ctx.render_ctx
                .stroke(&line, &env.get(theme::FOREGROUND_DARK), 1.0);
        }
        for child in &mut self.children {
            child.paint_with_offset(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &GlyphSet,
        env: &Env,
    ) -> Size {
        let width = (bc.max().width / GLYPH_SIZE).floor() * GLYPH_SIZE;
        let mut x: f64 = 0.;
        let mut y: f64 = 0.;

        let child_bc = BoxConstraints::tight(Size::new(GLYPH_SIZE, GLYPH_SIZE));

        for child in &mut self.children {
            if x > 0. && x + GLYPH_SIZE > width {
                y += GLYPH_SIZE;
                x = 0.;
            }
            child.layout(ctx, &child_bc, data, env);
            child.set_layout_rect(Rect::from_origin_size((x, y), (GLYPH_SIZE, GLYPH_SIZE)));
            x += GLYPH_SIZE;
        }
        Size::new(width, y + GLYPH_SIZE)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GlyphSet, env: &Env) {
        for child in &mut self.children {
            child.event(ctx, event, data, env)
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old: Option<&GlyphSet>, new: &GlyphSet, _env: &Env) {
        if new.font.ufo.glyph_count() != self.children.len() {
            let units_per_em = new
                .font
                .ufo
                .font_info
                .as_ref()
                .and_then(|info| info.units_per_em.clone())
                .unwrap_or(1000.);
            let widget = GridInner { units_per_em };
            self.children.clear();
            for key in new.font.ufo.iter_names() {
                self.children.push(WidgetPod::new(LensWrap::new(
                    widget,
                    lenses::glyph_set::Glyph(key),
                )));
            }
        }
        ctx.invalidate();
    }
}

impl GlyphGrid {
    pub fn new() -> GlyphGrid {
        GlyphGrid {
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GridInner {
    units_per_em: f64,
}

impl Widget<GlyphPlus> for GridInner {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &GlyphPlus, env: &Env) {
        //TODO: replacement for missing glyphs
        let path = data.get_bezier();
        let bb = path.bounding_box();
        let geom = Rect::ZERO.with_size(state.size());
        let scale = geom.height() as f64 / self.units_per_em;
        let scale = scale * 0.85; // some margins around glyphs
        let scaled_width = bb.width() * scale as f64;
        let l_pad = ((geom.width() as f64 - scaled_width) / 2.).round();
        let baseline = (geom.height() * 0.16) as f64;
        let affine = Affine::new([
            scale as f64,
            0.0,
            0.0,
            -scale as f64,
            l_pad,
            geom.height() - baseline,
        ]);

        let hl_color = env.get(theme::SELECTION_COLOR);
        let glyph_color = env.get(theme::FOREGROUND_DARK);
        let glyph_body_color = if state.is_active() {
            &hl_color
        } else {
            &glyph_color
        };
        ctx.render_ctx.fill(affine * &*path, glyph_body_color);

        if state.is_hot() {
            ctx.render_ctx.stroke(affine * &*path, &hl_color, 1.0);
            ctx.render_ctx.stroke(geom, &hl_color, 1.0);
        }

        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let name_color = if state.is_hot() {
            hl_color
        } else {
            glyph_color
        };
        let text = get_text_layout(&mut ctx.text(), &data.glyph.name, env);
        let xpos = geom.x0 + (geom.width() - text.width()) * 0.5;
        let ypos = geom.y0 + geom.height() - font_size * 0.25;
        let pos = (xpos, ypos);

        //draw a semi-translucent background
        let text_bg_rect = Rect::from_origin_size(
            (pos.0 as f64, (pos.1 - font_size * 0.75) as f64),
            (text.width() as f64, font_size as f64),
        );

        ctx.render_ctx.fill(
            &text_bg_rect,
            &env.get(theme::BACKGROUND_DARK).with_alpha(0.5),
        );
        // draw the text
        ctx.render_ctx.draw_text(&text, pos, &name_color)
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _d: &GlyphPlus,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut GlyphPlus, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.invalidate();
                    if ctx.is_hot() {
                        ctx.submit_command(Command::new(EDIT_GLYPH, data.glyph.name.clone()), None);
                    }
                }
            }
            Event::HotChanged(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old: Option<&GlyphPlus>,
        new: &GlyphPlus,
        _env: &Env,
    ) {
        if old.map(|old| !old.same(new)).unwrap_or(true) {
            ctx.invalidate();
        }
    }
}

fn get_text_layout(text_ctx: &mut PietText, text: &str, env: &Env) -> PietTextLayout {
    let font_name = env.get(theme::FONT_NAME);
    let font_size = env.get(theme::TEXT_SIZE_NORMAL);
    // TODO: caching of both the format and the layout
    let font = text_ctx
        .new_font_by_name(font_name, font_size)
        .build()
        .unwrap();
    text_ctx.new_text_layout(&font, text).build().unwrap()
}
