//! Controller widgets

use druid::widget::{prelude::*, Controller};
use druid::{InternalLifeCycle, LensExt, Rect, WidgetExt, WidgetPod};

use crate::consts;
use crate::data::{AppState, EditorState};
use crate::edit_session::EditSession;
use crate::menus;
use crate::widgets::{CoordPane, FloatingPanel, Toolbar};

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
            Event::Command(cmd) if cmd.is(consts::cmd::REBUILD_MENUS) => {
                let menu = menus::make_menu(data);
                ctx.set_menu(menu);
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
            ctx.set_menu(menu);
        }
        child.update(ctx, old_data, data, env);
    }
}

/// More like this is 'Editor' and 'Editor' is 'Canvas'?
//TODO: we could combine this with controller above if we wanted?
pub struct EditorController<W> {
    inner: W,
    toolbar: WidgetPod<(), FloatingPanel<Toolbar>>,
    coord_panel: WidgetPod<EditorState, FloatingPanel<Box<dyn Widget<EditorState>>>>,
}

impl<W> EditorController<W> {
    pub fn new(inner: W) -> Self {
        EditorController {
            inner,
            toolbar: WidgetPod::new(FloatingPanel::new(Toolbar::default())),
            coord_panel: WidgetPod::new(FloatingPanel::new(
                CoordPane::new()
                    .lens(EditorState::session.then(EditSession::selected_coord.in_arc()))
                    .boxed(),
            )),
        }
    }
}

impl<W: Widget<EditorState>> Widget<EditorState> for EditorController<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut EditorState, env: &Env) {
        // we would prefer to just handle this event in toolbar but it won't have focus
        // and so won't get the key event.
        if let Event::KeyDown(k) = event {
            if let Some(new_tool) = self.toolbar.widget().inner().tool_for_keypress(k) {
                let cmd = consts::cmd::SET_TOOL.with(new_tool);
                ctx.submit_command(cmd);
                ctx.set_handled();
                return;
            }
        }
        self.toolbar.event(ctx, event, &mut (), env);
        self.coord_panel.event(ctx, event, data, env);
        if !ctx.is_handled() {
            self.inner.event(ctx, event, data, env);
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &EditorState,
        env: &Env,
    ) {
        //HACK: we don't have 'ambient focus', so after the coord panel takes
        //focus, and then finishes editing, we need to tell the editor to
        //take focus back again so that it can handle keyboard input.
        if matches!(event, LifeCycle::Internal(InternalLifeCycle::RouteFocusChanged { new, .. }) if new.is_none())
        {
            ctx.submit_command(crate::consts::cmd::TAKE_FOCUS);
        }
        self.toolbar.lifecycle(ctx, event, &(), env);
        self.coord_panel.lifecycle(ctx, event, data, env);
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &EditorState,
        data: &EditorState,
        env: &Env,
    ) {
        self.coord_panel.update(ctx, data, env);
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &EditorState,
        env: &Env,
    ) -> Size {
        let child_bc = bc.loosen();
        let size = self.toolbar.layout(ctx, &child_bc, &(), env);
        self.toolbar
            .set_layout_rect(ctx, &(), env, Rect::from_origin_size((20.0, 20.0), size));
        let our_size = self.inner.layout(ctx, bc, data, env);
        let coords_size = self.coord_panel.layout(ctx, &child_bc, data, env);
        let coords_origin = (
            (our_size.width / 2.0) - coords_size.width / 2.0,
            our_size.height - coords_size.height - 20.0,
        );
        self.coord_panel.set_layout_rect(
            ctx,
            data,
            env,
            Rect::from_origin_size(coords_origin, coords_size),
        );

        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorState, env: &Env) {
        self.inner.paint(ctx, data, env);
        self.coord_panel.paint(ctx, data, env);
        self.toolbar.paint(ctx, &(), env);
    }
}
