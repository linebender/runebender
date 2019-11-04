//! the main editor widget.

use druid::kurbo::{Point, Rect, Size};
use druid::{
    BaseState, BoxConstraints, Command, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};
use druid::menu::ContextMenu;

use crate::consts::CANVAS_SIZE;
use crate::data::EditorState;
use crate::draw;
use crate::mouse::{Mouse, TaggedEvent};
use crate::tools::{Select, Tool};

/// The root widget of the glyph editor window.
pub struct Editor {
    mouse: Mouse,
    tool: Box<dyn Tool>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            mouse: Mouse::default(),
            tool: Box::new(Select::default()),
        }
    }

    fn send_mouse(
        &mut self,
        event: TaggedEvent,
        ctx: &mut EventCtx,
        data: &mut EditorState,
        env: &Env,
    ) {
        if !event.inner().button.is_right() {
            self.tool
                .mouse_event(event, &mut self.mouse, ctx, &mut data.session, env);
        } else if let TaggedEvent::Down(m) = event {
            let menu = crate::menus::make_context_menu(data, m.pos);
            let menu = ContextMenu::new(menu, m.window_pos);
            let cmd = Command::new(druid::command::sys::SHOW_CONTEXT_MENU, menu);
            ctx.submit_command(cmd, None);
        }
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, _: &BaseState, data: &EditorState, env: &Env) {
        use druid::piet::{Color, RenderContext};
        let rect =
            Rect::ZERO.with_size((CANVAS_SIZE.to_vec2() * data.session.viewport.zoom).to_size());
        ctx.fill(rect, &Color::WHITE);

        draw::draw_session(
            ctx,
            data.session.viewport,
            ctx.region().to_rect(),
            &data.metrics,
            &data.session,
            &data.ufo,
        );

        self.tool.paint(ctx, &data.session, env);
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        _bc: &BoxConstraints,
        data: &EditorState,
        _env: &Env,
    ) -> Size {
        (CANVAS_SIZE.to_vec2() * data.session.viewport.zoom).to_size()
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut EditorState, env: &Env) {
        use crate::consts::cmd;

        match event {
            Event::Command(c) => {
                let mut handled = true;
                match c.selector {
                    cmd::REQUEST_FOCUS => ctx.request_focus(),
                    cmd::SELECT_ALL => data.session.select_all(),
                    cmd::DESELECT_ALL => data.session.selection_mut().clear(),
                    cmd::DELETE => data.session.delete_selection(),
                    cmd::ADD_GUIDE => {
                        let point = c.get_object::<Point>().unwrap();
                        data.session.add_guide(*point);
                    }
                    cmd::TOGGLE_GUIDE => {
                        let cmd::ToggleGuideCmdArgs { id, pos } = c.get_object().unwrap();
                        data.session.toggle_guide(*id, *pos);
                    }
                    _ => handled = false,
                }
                if handled {
                    ctx.is_handled();
                    ctx.invalidate();
                }
            }
            Event::KeyDown(k) => self.tool.key_down(k, ctx, &mut data.session, env),
            Event::MouseUp(m) => self.send_mouse(TaggedEvent::Up(m.clone()), ctx, data, env),
            Event::MouseMoved(m) => self.send_mouse(TaggedEvent::Moved(m.clone()), ctx, data, env),
            Event::MouseDown(m) => self.send_mouse(TaggedEvent::Down(m.clone()), ctx, data, env),
            _ => (),
        };
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
