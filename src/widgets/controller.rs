//! The root widget.

use druid::kurbo::Size;
use druid::{
    BoxConstraints, Command, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::consts;
use crate::data::AppState;
use crate::menus;

/// A widget that wraps all root widgets
pub struct Controller {
    inner: Box<dyn Widget<AppState> + 'static>,
}

impl Widget<AppState> for Controller {
    fn paint(&mut self, ctx: &mut PaintCtx, d: &AppState, env: &Env) {
        self.inner.paint(ctx, d, env)
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
            Event::Command(cmd) if cmd.selector == consts::cmd::REBUILD_MENUS => {
                let menu = menus::make_menu(data);
                let cmd = Command::new(druid::commands::SET_MENU, menu);
                ctx.submit_command(cmd, None);
            }
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
}
