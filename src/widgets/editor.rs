//! the main editor widget.

use druid::kurbo::{Point, Size};
use druid::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use norad::GlyphName;

use crate::data::{lenses, AppState, EditorState};
use crate::design_space::ViewPort;
use crate::draw;
use crate::lens2::Lens2Wrap;

/// The root widget of the glyph editor window.
pub struct Editor(Point /* mouse pos; hacky, just to get zoom working */);

impl Editor {
    pub fn new(glyph_name: GlyphName) -> impl Widget<AppState> {
        Lens2Wrap::new(
            Editor(Point::ZERO),
            lenses::app_state::EditorState(glyph_name),
        )
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &EditorState, _env: &Env) {
        //TODO: replacement for missing glyphs
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
        bc: &BoxConstraints,
        _d: &EditorState,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut EditorState, _env: &Env) {
        match event {
            Event::MouseMoved(event) => self.0 = event.pos,
            Event::Wheel(wheel) => {
                if wheel.mods.meta {
                    data.session.viewport.zoom(wheel, self.0);
                } else {
                    data.session.viewport.scroll(wheel);
                }
                ctx.set_handled();
                ctx.invalidate();
            }
            _ => (),
        }
    }

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
