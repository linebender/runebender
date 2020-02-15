//! The sidebar of the main glyph list/grid view.

use druid::kurbo::Line;
use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Rect, RenderContext, Size, UpdateCtx, Widget,
};

use crate::theme;

pub struct Sidebar;

impl<T: Data> Widget<T> for Sidebar {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        _event: &LifeCycle,
        _data: &T,
        _env: &Env,
    ) {
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {}
    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let rect = Rect::ZERO.with_size(ctx.size());
        ctx.fill(rect, &env.get(theme::SIDEBAR_BACKGROUND));

        // to get clean strokes we have to *not* align on pixel boundaries
        let max_x = rect.max_x() - 0.5;
        let line = Line::new((max_x, 0.0), (max_x, rect.max_y()));
        ctx.stroke(line, &env.get(theme::SIDEBAR_EDGE_STROKE), 1.0);
    }
}
