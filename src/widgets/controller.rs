//! Controller widgets

use druid::widget::prelude::*;
use druid::widget::Controller;
use druid::{Command, Data, Env, Event, EventCtx, Rect, UpdateCtx, Widget, WidgetPod};

use crate::consts;
use crate::data::AppState;
use crate::menus;
use crate::widgets::Toolbar;

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

    fn update(
        &mut self,
        child: &mut W,
        ctx: &mut UpdateCtx,
        old_data: &AppState,
        data: &AppState,
        env: &Env,
    ) {
        if old_data.workspace.selected.is_none() != data.workspace.selected.is_none() {
            let menu = menus::make_menu(data);
            let cmd = Command::new(druid::commands::SET_MENU, menu);
            ctx.submit_command(cmd, None);
        }
        child.update(ctx, old_data, data, env);
    }
}

/// More like this is 'Editor' and 'Editor' is 'Canvas'?
//TODO: we could combine this with controller above if we wanted?
pub struct EditorController<W> {
    inner: W,
    toolbar: WidgetPod<(), Toolbar>,
}

impl<W> EditorController<W> {
    pub fn new(inner: W) -> Self {
        EditorController {
            inner,
            toolbar: WidgetPod::new(Toolbar::default()),
        }
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for EditorController<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.toolbar.event(ctx, event, &mut (), env);
        if !ctx.is_handled() {
            self.inner.event(ctx, event, data, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.toolbar.lifecycle(ctx, event, &(), env);
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.toolbar.layout(ctx, bc, &(), env);
        self.toolbar
            .set_layout_rect(ctx, &(), env, Rect::from_origin_size((20.0, 20.0), size));
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.inner.paint(ctx, data, env);
        self.toolbar.paint_with_offset(ctx, &(), env);
    }
}
