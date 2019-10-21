//! the main editor widget.

use druid::kurbo::{Affine, Rect, Shape, Size};
use druid::piet::RenderContext;
use druid::{
    theme, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx,
    Widget,
};

use norad::GlyphName;

use crate::data::{lenses, AppState, EditorState};
use crate::lens2::Lens2Wrap;

/// The root widget of the glyph editor window.
pub struct Editor;

impl Editor {
    pub fn new(glyph_name: GlyphName) -> impl Widget<AppState> {
        Lens2Wrap::new(Editor, lenses::app_state::EditorState(glyph_name))
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &EditorState, env: &Env) {
        //TODO: replacement for missing glyphs
        let path = crate::data::get_bezier(&data.session.name, &data.ufo, None).unwrap_or_default();
        let bb = path.bounding_box();
        let geom = Rect::ZERO.with_size(state.size());
        let scale = geom.height() as f64 / data.metrics.units_per_em;
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

        ctx.render_ctx
            .fill(affine * &*path, &env.get(theme::FOREGROUND_DARK));
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
