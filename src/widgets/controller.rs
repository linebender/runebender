//! The root widget.

use druid::kurbo::Size;
use druid::{
    BaseState, BoxConstraints, Env, Event, EventCtx, FileInfo, LayoutCtx, PaintCtx, UpdateCtx,
    Widget,
};
use log;
use norad::Ufo;

use crate::data::AppState;

pub struct Controller {
    inner: Box<dyn Widget<AppState> + 'static>,
}

impl Widget<AppState> for Controller {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, d: &AppState, env: &Env) {
        self.inner.paint(ctx, state, d, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        d: &AppState,
        env: &Env,
    ) -> Size {
        self.inner.layout(ctx, bc, d, env)
    }

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut AppState, env: &Env) {
        match event {
            Event::OpenFile(file_info) => self.try_open_file(file_info, ctx, data),
            other => self.inner.event(ctx, other, data, env),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: Option<&AppState>, new: &AppState, env: &Env) {
        self.inner.update(ctx, old, new, env)
    }
}

impl Controller {
    pub fn new(inner: impl Widget<AppState> + 'static) -> Controller {
        Controller {
            inner: Box::new(inner),
        }
    }

    fn try_open_file(&mut self, info: &FileInfo, ctx: &mut EventCtx, data: &mut AppState) {
        match Ufo::load(info.path()) {
            Ok(ufo) => data.set_file(ufo, info.path().to_owned()),
            Err(e) => {
                log::error!("failed to open file {:?}, errror: '{:?}'", info.path(), e);
                return;
            }
        };
        ctx.invalidate();
    }
}
