//! the main editor widget.

use std::sync::Arc;

use druid::kurbo::{Point, Rect, Size};
use druid::{
    Application, BoxConstraints, Clipboard, ClipboardFormat, Command, ContextMenu, Data, Env,
    Event, EventCtx, KeyCode, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, UpdateCtx, Widget,
};

use crate::consts::{self, CANVAS_SIZE};
use crate::data::EditorState;
use crate::draw;
use crate::edit_session::EditSession;
use crate::mouse::{Mouse, TaggedEvent};
use crate::tools::{EditType, Select, Tool, ToolId};
use crate::undo::UndoState;

/// The root widget of the glyph editor window.
pub struct Editor {
    mouse: Mouse,
    tool: Box<dyn Tool>,
    // in the case of the temporary preview (spacebar) this is the tool
    // that will be restored when spacebar is released.
    //prev_tool: Option<Box<dyn Tool>>,
    undo: UndoState<Arc<EditSession>>,
    last_edit: EditType,
    /// If true, this session should be drawn with all glyphs filled and
    /// with no non-glyph items visible.
    draw_filled_outlines: bool,
}

impl Editor {
    pub fn new(session: Arc<EditSession>) -> Editor {
        Editor {
            mouse: Mouse::default(),
            tool: Box::new(Select::default()),
            //prev_tool: None,
            undo: UndoState::new(session),
            last_edit: EditType::Normal,
            draw_filled_outlines: false,
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
                .mouse_event(event, &mut self.mouse, ctx, data.session_mut(), env);
        } else if let TaggedEvent::Down(m) = event {
            let menu = crate::menus::make_context_menu(data, m.pos);
            let menu = ContextMenu::new(menu, m.window_pos);
            ctx.show_context_menu(menu);
            //let cmd = Command::new(druid::commands::SHOW_CONTEXT_MENU, menu);
            //ctx.submit_command(cmd, None);
        }
        None
    }

    fn update_undo(&mut self, edit: Option<EditType>, data: &Arc<EditSession>) {
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

    fn do_undo(&mut self) -> Option<&Arc<EditSession>> {
        self.undo.undo()
    }

    fn do_redo(&mut self) -> Option<&Arc<EditSession>> {
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
            Application::global().clipboard().put_formats(&formats);
        }
    }

    fn do_paste(&self, session: &mut EditSession, clipboard: &Clipboard) -> Option<EditType> {
        let paste_types = [
            crate::consts::GLYPHS_APP_PASTEBOARD_TYPE,
            ClipboardFormat::PDF,
            ClipboardFormat::SVG,
        ];
        if let Some(match_) = clipboard.preferred_format(&paste_types) {
            let paths = match (match_, clipboard.get_format(match_)) {
                (_, None) => {
                    log::warn!("no data returned for declared clipboard format {}", match_);
                    return None;
                }
                (crate::consts::GLYPHS_APP_PASTEBOARD_TYPE, Some(data)) => {
                    match String::from_utf8(data) {
                        Ok(s) => crate::clipboard::from_glyphs_plist_string(s),
                        Err(e) => crate::clipboard::from_glyphs_plist(e.into_bytes()),
                    }
                }
                (ClipboardFormat::PDF, Some(data)) => crate::clipboard::from_pdf_data(data),
                _ => None,
            };
            if let Some(paths) = paths {
                session.paste_paths(paths);
                return Some(EditType::Normal);
            }
        }

        None
    }

    /// handle a `Command`. Returns a bool indicating whether the command was
    /// handled at all, and an optional `EditType` if this command did work
    /// that should go on the undo stack.
    fn handle_cmd(&mut self, cmd: &Command, data: &mut EditorState) -> (bool, Option<EditType>) {
        match cmd.selector {
            consts::cmd::SELECT_ALL => data.session_mut().select_all(),
            consts::cmd::DESELECT_ALL => data.session_mut().clear_selection(),
            consts::cmd::DELETE => data.session_mut().delete_selection(),
            consts::cmd::TOGGLE_PREVIEW_TOOL => {
                let is_mouse_down: &bool = cmd.get_object().unwrap();
                // we don't toggle preview if we're actually *in* preview
                if self.tool.name() != "Preview" {
                    self.draw_filled_outlines = *is_mouse_down;
                    return (true, None);
                }
            }
            consts::cmd::ADD_GUIDE => {
                let point = cmd.get_object::<Point>().unwrap();
                data.session_mut().add_guide(*point);
                return (true, Some(EditType::Normal));
            }
            consts::cmd::TOGGLE_GUIDE => {
                let consts::cmd::ToggleGuideCmdArgs { id, pos } = cmd.get_object().unwrap();
                data.session_mut().toggle_guide(*id, *pos);
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
                    data.session_mut().viewport = saved_viewport;
                }
            }
            druid::commands::REDO => {
                if let Some(next) = self.do_redo() {
                    let saved_viewport = data.session.viewport;
                    data.session = next.clone();
                    data.session_mut().viewport = saved_viewport;
                }
            }
            // all unhandled commands:
            _ => return (false, None),
        }

        // the default: commands with an `EditType` return explicitly.
        (true, None)
    }

    fn set_tool(&mut self, data: &mut EditorState, tool: Box<dyn Tool>) {
        self.draw_filled_outlines = tool.name() == "Preview";
        data.session_mut().tool_desc = tool.name().into();
        self.tool = tool;
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorState, env: &Env) {
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
            &data.font,
            self.draw_filled_outlines,
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
        // we request_paint if selection changes after this event;
        let pre_selection = data.session.selection.clone();
        let pre_paths = data.session.paths.clone();
        let pre_components = data.session.components.clone();

        let edit = match event {
            Event::WindowConnected => {
                ctx.request_focus();
                None
            }
            Event::Command(cmd) => match cmd {
                c if c.selector == crate::consts::cmd::TAKE_FOCUS => {
                    ctx.request_focus();
                    ctx.set_handled();
                    None
                }

                c if c.selector == consts::cmd::SET_TOOL => {
                    let tool = cmd.get_object::<ToolId>().unwrap();
                    let tool = crate::tools::tool_for_id(tool).unwrap();
                    self.set_tool(data, tool);
                    None
                }
                c => {
                    let (handled, edit) = self.handle_cmd(c, data);
                    if handled {
                        ctx.set_handled();
                        ctx.request_paint();
                    }
                    edit
                }
            },
            Event::KeyDown(k) if k.key_code == KeyCode::Escape => {
                data.session_mut().clear_selection();
                None
            }
            Event::KeyDown(k) => self.tool.key_down(k, ctx, data.session_mut(), env),
            Event::KeyUp(k) => self.tool.key_up(k, ctx, data.session_mut(), env),
            Event::MouseUp(m) => self.send_mouse(ctx, TaggedEvent::Up(m.clone()), data, env),
            Event::MouseMove(m) => self.send_mouse(ctx, TaggedEvent::Moved(m.clone()), data, env),
            Event::MouseDown(m) => self.send_mouse(ctx, TaggedEvent::Down(m.clone()), data, env),
            Event::Paste(clipboard) => self.do_paste(data.session_mut(), clipboard),
            _ => None,
        };

        self.update_undo(edit, &data.session);
        if edit.is_some() || !pre_selection.same(&data.session.selection) {
            ctx.request_paint();
        }

        if !pre_paths.same(&data.session.paths) || !pre_components.same(&data.session.components) {
            data.session_mut().rebuild_glyph();
        }
    }

    fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &EditorState, _: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old: &EditorState, new: &EditorState, _env: &Env) {
        if !old.same(new) {
            ctx.request_paint();
        }
    }
}
