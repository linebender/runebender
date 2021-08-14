//! Controller widgets

use druid::widget::prelude::*;
use druid::{InternalLifeCycle, LensExt, Rect, WidgetExt, WidgetPod};

use crate::consts;
use crate::data::EditorState;
use crate::edit_session::EditSession;
use crate::widgets::{CoordPane, FloatingPanel, GlyphPane, Toolbar};

/// the distance from the edge of a floating panel to the edge of the window.
const FLOATING_PANEL_PADDING: f64 = 24.0;

/// More like this is 'Editor' and 'Editor' is 'Canvas'?
//TODO: we could combine this with controller above if we wanted?
pub struct EditorController<W> {
    inner: W,
    toolbar: WidgetPod<(), FloatingPanel<Toolbar>>,
    coord_panel: WidgetPod<EditorState, FloatingPanel<Box<dyn Widget<EditorState>>>>,
    glyph_panel: WidgetPod<EditorState, FloatingPanel<Box<dyn Widget<EditorState>>>>,
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
            glyph_panel: WidgetPod::new(FloatingPanel::new(GlyphPane::new().boxed())),
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
        self.glyph_panel.event(ctx, event, data, env);
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
        self.glyph_panel.lifecycle(ctx, event, data, env);
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
        self.glyph_panel.update(ctx, data, env);
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
        let orig = (FLOATING_PANEL_PADDING, FLOATING_PANEL_PADDING);
        self.toolbar
            .set_layout_rect(ctx, &(), env, Rect::from_origin_size(orig, size));
        let our_size = self.inner.layout(ctx, bc, data, env);
        let coords_size = self.coord_panel.layout(ctx, &child_bc, data, env);
        let coords_origin = (
            (our_size.width) - coords_size.width - FLOATING_PANEL_PADDING,
            our_size.height - coords_size.height - FLOATING_PANEL_PADDING,
        );
        let coord_frame = Rect::from_origin_size(coords_origin, coords_size);
        self.coord_panel
            .set_layout_rect(ctx, data, env, coord_frame);

        let size = self.glyph_panel.layout(ctx, &child_bc, data, env);
        let orig = (
            FLOATING_PANEL_PADDING,
            our_size.height - size.height - FLOATING_PANEL_PADDING,
        );
        let frame = Rect::from_origin_size(orig, size);
        self.glyph_panel.set_layout_rect(ctx, data, env, frame);
        our_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorState, env: &Env) {
        self.inner.paint(ctx, data, env);
        self.coord_panel.paint(ctx, data, env);
        self.glyph_panel.paint(ctx, data, env);
        self.toolbar.paint(ctx, &(), env);
    }
}
