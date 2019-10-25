//! the main editor widget.

use druid::kurbo::{Rect, Size, Vec2};
use druid::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::data::{glyph_rect, EditorState};
use crate::design_space::ViewPort;
use crate::draw;

/// The root widget of the glyph editor window.
pub struct Editor {
    work_offset: Vec2,
}

pub const CANVAS_SIZE: Size = Size::new(10000., 10000.);

impl Editor {
    pub fn new() -> Editor {
        Editor {
            work_offset: Vec2::ZERO,
        }
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &EditorState, _env: &Env) {
        use druid::piet::{Color, RenderContext};
        //paint_checkerboard(ctx, data.session.viewport.zoom);
        let rect =
            Rect::ZERO.with_size((CANVAS_SIZE.to_vec2() * data.session.viewport.zoom).to_size());
        ctx.fill(rect, &Color::WHITE);

        //FIXME HACK: we were stashing work_offset in the editor struct while playing
        //around, it should be removed and only live in ViewPort or equivalent
        let viewport = ViewPort {
            zoom: data.session.viewport.zoom,
            flipped_y: true,
            offset: self.work_offset,
        };
        draw::draw_session(
            ctx,
            viewport,
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
        // FIXME: use all items on canvas when computing size
        let work_rect = glyph_rect(data);
        let canvas_rect = Rect::ZERO.with_size(CANVAS_SIZE);
        let work_offset = canvas_rect.center() - work_rect.center();

        self.work_offset = work_offset * data.session.viewport.zoom;
        // we want to center the frame of the glyph on the canvas

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
