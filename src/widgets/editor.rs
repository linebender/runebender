//! the main editor widget.

use druid::kurbo::Size;
use druid::{
    BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use norad::GlyphName;

use crate::data::{lenses, AppState, EditorState};
use crate::design_space::ViewPort;
use crate::draw;
use crate::lens2::Lens2Wrap;

/// The root widget of the glyph editor window.
pub struct Editor;

impl Editor {
    pub fn new(glyph_name: GlyphName) -> impl Widget<AppState> {
        Lens2Wrap::new(Editor, lenses::app_state::EditorState(glyph_name))
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &EditorState, _env: &Env) {
        //TODO: replacement for missing glyphs
        draw::draw_session(
            ctx,
            ViewPort::default(),
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
