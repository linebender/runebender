//! The top-level widget for the main glyph list window.

use druid::kurbo::{Rect, Size};
use druid::piet::{Color, RenderContext};

use druid::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::data::AppState;

pub struct GlyphGrid {}

const TOTAL_HEIGHT: f64 = 2000.;

impl Widget<AppState> for GlyphGrid {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, _d: &AppState, _env: &Env) {
        let rows = 20;
        let cols = 10;
        let item_width = state.size().width / cols as f64;
        let item_height = TOTAL_HEIGHT / (rows as f64);
        for row in 0..rows {
            let row_progress = row as f64 / rows as f64;
            for col in 0..cols {
                let col_progress = col as f64 / cols as f64;
                let color = Color::rgb(1.0 * col_progress, 1.0 * row_progress, 1.0);
                let x = item_width * col as f64;
                let y = item_height * row as f64;
                let rect = Rect::new(x, y, x + item_width, y + item_height);
                ctx.render_ctx.fill(rect, &color);
            }
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _d: &AppState,
        _env: &Env,
    ) -> Size {
        Size::new(bc.max().width, TOTAL_HEIGHT)
    }

    fn event(&mut self, _event: &Event, _ctx: &mut EventCtx, _data: &mut AppState, _env: &Env) {}

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old: Option<&AppState>,
        _new: &AppState,
        _env: &Env,
    ) {
        ctx.invalidate();
    }
}

impl GlyphGrid {
    pub fn new() -> GlyphGrid {
        GlyphGrid {}
    }
}
