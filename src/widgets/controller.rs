//! Controller widgets

use druid::widget::Controller;
use druid::{Command, Env, Event, EventCtx, Widget};

use crate::consts;
use crate::data::AppState;
use crate::menus;

/// A widget that wraps all root widgets
#[derive(Debug, Default)]
pub struct RootWindowController;

impl<W: Widget<AppState>> Controller<AppState, W> for RootWindowController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.selector == consts::cmd::REBUILD_MENUS => {
                let menu = menus::make_menu(data);
                let cmd = Command::new(druid::commands::SET_MENU, menu);
                ctx.submit_command(cmd, None);
            }
            other => child.event(ctx, other, data, env),
        }
    }
}
