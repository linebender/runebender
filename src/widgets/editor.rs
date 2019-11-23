//! the main editor widget.

use std::sync::Arc;

use druid::kurbo::{Point, Rect, Size};
use druid::{
    Application, BaseState, BoxConstraints, ClipboardFormat, Command, ContextMenu, Data, Env,
    Event, EventCtx, KeyCode, LayoutCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::consts::{self, CANVAS_SIZE};
use crate::data::EditorState;
use crate::draw;
use crate::edit_session::EditSession;
use crate::mouse::{Mouse, TaggedEvent};
use crate::tools::{EditType, Pen, Select, Tool};
use crate::undo::UndoState;

/// The root widget of the glyph editor window.
pub struct Editor {
    mouse: Mouse,
    tool: Box<dyn Tool>,
    undo: UndoState<EditSession>,
    last_edit: EditType,
}

impl Editor {
    pub fn new(session: EditSession) -> Editor {
        Editor {
            mouse: Mouse::default(),
            tool: Box::new(Select::default()),
            undo: UndoState::new(session),
            last_edit: EditType::Normal,
        }
    }

    fn send_mouse(
        &mut self,
        ctx: &mut EventCtx,
        event: TaggedEvent,
        data: &mut EditorState,
        env: &Env,
    ) -> Option<EditType> {
        if !event.inner().button.is_right() {
            return self
                .tool
                .mouse_event(event, &mut self.mouse, ctx, &mut data.session, env);
        } else if let TaggedEvent::Down(m) = event {
            let menu = crate::menus::make_context_menu(data, m.pos);
            let menu = ContextMenu::new(menu, m.window_pos);
            let cmd = Command::new(druid::commands::SHOW_CONTEXT_MENU, menu);
            ctx.submit_command(cmd, None);
        }
        None
    }

    fn update_undo(&mut self, edit: Option<EditType>, data: &EditSession) {
        match edit {
            Some(edit) if self.last_edit.needs_new_undo_group(edit) => {
                self.undo.add_undo_group(data.clone())
            }
            Some(_) => self.undo.update_current_undo(|state| *state = data.clone()),
            // I'm not sure what to do here? I wanted to check if selections had
            // changed, and then update the current undo if necessary?
            // but that requires us to pass in the previous data. We can do that!
            // I'm just not sure, right now, that it makes sense
            None => (),
        }
        self.last_edit = edit.unwrap_or(self.last_edit);
    }

    fn do_undo(&mut self) -> Option<&EditSession> {
        self.undo.undo()
    }

    fn do_redo(&mut self) -> Option<&EditSession> {
        self.undo.redo()
    }

    fn do_copy(&self, data: &EditSession) {
        let mut formats = Vec::new();
        if let Some(data) = crate::clipboard::make_glyphs_plist(data) {
            formats.push(ClipboardFormat::new(
                crate::consts::GLYPHS_APP_PASTEBOARD_TYPE,
                data,
            ));
        }

        if let Some(bytes) = crate::clipboard::make_pdf_data(data) {
            formats.push(ClipboardFormat::new(ClipboardFormat::PDF, bytes));
        }

        if let Some(bytes) = crate::clipboard::make_svg_data(data) {
            formats.push(ClipboardFormat::new(ClipboardFormat::SVG, bytes))
        }

        if let Some(code) = crate::clipboard::make_code_string(data) {
            formats.push(code.into());
        }

        if !formats.is_empty() {
            Application::clipboard().put_formats(&formats);
        }
    }

    /// handle a `Command`. Returns a bool indicating whether the command was
    /// handled at all, and an optional `EditType` if this command did work
    /// that should go on the undo stack.
    fn handle_cmd(
        &mut self,
        cmd: &Command,
        ctx: &mut EventCtx,
        data: &mut EditorState,
    ) -> (bool, Option<EditType>) {
        match cmd.selector {
            consts::cmd::REQUEST_FOCUS => ctx.request_focus(),
            consts::cmd::SELECT_ALL => data.session.select_all(),
            consts::cmd::DESELECT_ALL => data.session.clear_selection(),
            consts::cmd::DELETE => data.session.delete_selection(),
            consts::cmd::SELECT_TOOL => {
                self.tool = Box::new(Select::default());
                data.session.tool_desc = Arc::from("Select");
            }
            consts::cmd::PEN_TOOL => {
                self.tool = Box::new(Pen::default());
                data.session.tool_desc = Arc::from("Pen");
            }
            consts::cmd::ADD_GUIDE => {
                let point = cmd.get_object::<Point>().unwrap();
                data.session.add_guide(*point);
                return (true, Some(EditType::Normal));
            }
            consts::cmd::TOGGLE_GUIDE => {
                let consts::cmd::ToggleGuideCmdArgs { id, pos } = cmd.get_object().unwrap();
                data.session.toggle_guide(*id, *pos);
                return (true, Some(EditType::Normal));
            }
            druid::commands::COPY => self.do_copy(&data.session),
            druid::commands::UNDO => {
                if let Some(prev) = self.do_undo() {
                    //HACK: because zoom & offset is part of data, and we don't
                    //want to jump around during undo/redo, we always manually
                    //reuse the current viewport when handling these actions.
                    let saved_viewport = data.session.viewport;
                    data.session = prev.clone();
                    data.session.viewport = saved_viewport;
                }
            }
            druid::commands::REDO => {
                if let Some(next) = self.do_redo() {
                    let saved_viewport = data.session.viewport;
                    data.session = next.clone();
                    data.session.viewport = saved_viewport;
                }
            }
            // all unhandled commands:
            _ => return (false, None),
        }

        // the default: commands with an `EditType` return explicitly.
        (true, None)
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

    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut EditorState, env: &Env) {
        // we invalidate if selection changes after this event;
        let pre_selection = data.session.selection.clone();

        let edit = match event {
            Event::Command(c) => {
                let (handled, edit) = self.handle_cmd(c, ctx, data);
                if handled {
                    ctx.is_handled();
                    ctx.invalidate();
                }
                edit
            }
            Event::KeyDown(k) if k.key_code == KeyCode::Escape => {
                data.session.clear_selection();
                None
            }
            Event::KeyDown(k) => self.tool.key_down(k, ctx, &mut data.session, env),
            Event::MouseUp(m) => self.send_mouse(ctx, TaggedEvent::Up(m.clone()), data, env),
            Event::MouseMoved(m) => self.send_mouse(ctx, TaggedEvent::Moved(m.clone()), data, env),
            Event::MouseDown(m) => self.send_mouse(ctx, TaggedEvent::Down(m.clone()), data, env),
            _ => None,
        };

        self.update_undo(edit, &data.session);
        if edit.is_some() || !pre_selection.same(&data.session.selection) {
            ctx.invalidate();
        }
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
