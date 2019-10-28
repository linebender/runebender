//! the main editor widget.

use druid::kurbo::{Rect, Size};
use druid::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::data::EditorState;
use crate::draw;

/// The root widget of the glyph editor window.
pub struct Editor {}

pub const CANVAS_SIZE: Size = Size::new(5000., 5000.);

impl Editor {
    pub fn new() -> Editor {
        Editor {}
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &EditorState, _env: &Env) {
        use druid::piet::{Color, RenderContext};
        //paint_checkerboard(ctx, data.session.viewport.zoom);
        let rect =
            Rect::ZERO.with_size((CANVAS_SIZE.to_vec2() * data.session.viewport.zoom).to_size());
        ctx.fill(rect, &Color::WHITE);

        draw::draw_session(
            ctx,
            data.session.viewport,
            state.size(),
            &data.metrics,
            &data.session,
            &data.ufo,
        );
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        data: &EditorState,
        _env: &Env,
    ) -> Size {
        (CANVAS_SIZE.to_vec2() * data.session.viewport.zoom).to_size()
    }

    fn event(&mut self, _event: &Event, _ctx: &mut EventCtx, _data: &mut EditorState, _env: &Env) {}

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old: Option<&EditorState>,
        new: &EditorState,
        _env: &Env,
    ) {
        if !Some(new).same(&old) {
            ctx.invalidate();
        }
    }
}
