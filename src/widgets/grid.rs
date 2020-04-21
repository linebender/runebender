//! The top-level widget for the main glyph list window.

use druid::kurbo::{Affine, Line, Rect, Shape, Size};
use druid::piet::{
    FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout, TextLayoutBuilder,
};
use druid::{
    BoxConstraints, Command, Data, Env, Event, EventCtx, Insets, LayoutCtx, LensWrap, LifeCycle,
    LifeCycleCtx, PaintCtx, UpdateCtx, Widget, WidgetExt, WidgetPod,
};

use crate::app_delegate::EDIT_GLYPH;
use crate::data::{lenses, GlyphPlus, Workspace};
use crate::theme;
use crate::widgets::Maybe;

const GLYPH_SIZE: f64 = 100.;

#[derive(Default)]
pub struct GlyphGrid {
    children: Vec<WidgetPod<Workspace, Box<dyn Widget<Workspace>>>>,
}

impl GlyphGrid {
    fn update_children(&mut self, data: &Workspace) {
        self.children.clear();
        for key in data.font.ufo.iter_names() {
            let widget = Maybe::or_empty(|| GridInner);
            self.children.push(WidgetPod::new(
                LensWrap::new(widget, lenses::app_state::Glyph(key)).boxed(),
            ));
        }
    }
}

impl Widget<Workspace> for GlyphGrid {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &Workspace, env: &Env) {
        ctx.render_ctx.clear(env.get(theme::GLYPH_LIST_BACKGROUND));
        let row_len = 1.0_f64.max(ctx.size().width / GLYPH_SIZE).floor() as usize;
        let row_count = if self.children.is_empty() {
            0
        } else {
            self.children.len() / row_len + 1
        };

        for row in 0..row_count {
            let baseline = row as f64 * GLYPH_SIZE + GLYPH_SIZE * (1.0 - 0.16) + 0.5;

            let line = Line::new((0., baseline), (ctx.size().width + GLYPH_SIZE, baseline));
            ctx.render_ctx
                .stroke(&line, &env.get(theme::GLYPH_LIST_STROKE), 1.0);
        }

        for child in &mut self.children {
            child.paint_with_offset(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Workspace,
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
            let rect = Rect::from_origin_size((x, y), (GLYPH_SIZE, GLYPH_SIZE));
            child.set_layout_rect(ctx, data, env, rect);
            x += GLYPH_SIZE;
        }
        Size::new(width, y + GLYPH_SIZE)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Workspace, env: &Env) {
        for child in &mut self.children {
            child.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Workspace,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if self.children.is_empty() {
                self.update_children(data);
            }
        }

        for child in &mut self.children {
            child.lifecycle(ctx, event, data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &Workspace, new: &Workspace, env: &Env) {
        if !old.font.same(&new.font) {
            self.update_children(new);
            ctx.children_changed();
            ctx.request_paint();
        } else {
            for child in &mut self.children {
                child.update(ctx, new, env);
            }
        }
    }
}

impl GlyphGrid {
    pub fn new() -> GlyphGrid {
        Default::default()
    }
}

#[derive(Debug, Clone, Copy)]
struct GridInner;

impl Widget<GlyphPlus> for GridInner {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &GlyphPlus, env: &Env) {
        let path = data.get_bezier();
        let bb = path.bounding_box();
        let geom = Rect::ZERO.with_size(ctx.size());
        let scale = geom.height() as f64 / data.upm();
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

        let hl_color = env.get(druid::theme::SELECTION_COLOR);
        if ctx.is_active() || data.is_selected {
            let selection_rect: Rect = geom - Insets::uniform(5.0);
            let rounded = selection_rect.to_rounded_rect(5.0);
            ctx.fill(rounded, &hl_color);
        }
        let glyph_color = if data.is_placeholder_glyph() {
            env.get(theme::PLACEHOLDER_GLYPH_COLOR)
        } else {
            env.get(theme::GLYPH_COLOR)
        };

        ctx.render_ctx.fill(affine * &*path, &glyph_color);

        let font_size = env.get(theme::GLYPH_LIST_LABEL_TEXT_SIZE);
        let text_color = env.get(theme::GLYPH_COLOR);

        let text = get_text_layout(&mut ctx.text(), &data.glyph.name, env);
        let xpos = geom.x0 + (geom.width() - text.width()) * 0.5;
        let ypos = geom.y0 + geom.height() - font_size * 0.25;
        let pos = (xpos, ypos);

        // draw the text
        ctx.render_ctx.draw_text(&text, pos, &text_color)
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
            Event::MouseDown(m) => {
                ctx.set_active(true);
                ctx.request_paint();
                if m.count == 1 {
                    data.is_selected = true;
                } else if m.count == 2 {
                    ctx.submit_command(Command::new(EDIT_GLYPH, data.glyph.name.clone()), None);
                }
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _: &GlyphPlus, _: &Env) {
        if let LifeCycle::HotChanged(_) = event {
            ctx.request_paint();
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: &GlyphPlus, new: &GlyphPlus, _env: &Env) {
        if !old.same(new) {
            ctx.request_paint();
        }
    }
}

fn get_text_layout(text_ctx: &mut PietText, text: &str, env: &Env) -> PietTextLayout {
    let font_name = env.get(theme::FONT_NAME);
    let font_size = env.get(theme::GLYPH_LIST_LABEL_TEXT_SIZE);
    // TODO: caching of both the format and the layout
    let font = text_ctx
        .new_font_by_name(font_name, font_size)
        .build()
        .unwrap();
    text_ctx
        .new_text_layout(&font, text, std::f64::INFINITY)
        .build()
        .unwrap()
}
