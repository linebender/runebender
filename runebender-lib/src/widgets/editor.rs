//! the main editor widget.

use std::sync::Arc;

use druid::widget::prelude::*;
use druid::{Application, Clipboard, ClipboardFormat, Command, Data, KbKey};

use crate::consts::{self, CANVAS_SIZE};
use crate::data::EditorState;
use crate::draw;
use crate::edit_session::EditSession;
use crate::mouse::{Mouse, TaggedEvent};
use crate::theme;
use crate::tools::{EditType, Preview, Select, Tool};
use crate::undo::UndoState;

/// The root widget of the glyph editor window.
pub struct Editor {
    mouse: Mouse,
    tool: Box<dyn Tool>,
    /// Set only if we're temporarily in preview mode. (when spacebar is held)
    temp_preview: Option<Box<dyn Tool>>,
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
            temp_preview: None,
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
            // set active, to ensure we receive events if the mouse leaves
            // the window:
            match &event {
                TaggedEvent::Down(_) => ctx.set_active(true),
                TaggedEvent::Up(m) if m.buttons.is_empty() => ctx.set_active(false),
                _ => (),
            };

            let tool = self.temp_preview.as_mut().unwrap_or(&mut self.tool);
            return tool.mouse_event(event, &mut self.mouse, ctx, data.session_mut(), env);
        } else if let TaggedEvent::Down(m) = event {
            let menu = crate::menus::make_context_menu(data, m.pos);
            ctx.show_context_menu(menu, m.pos);
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
        if let Some(data) = crate::clipboard::make_json(data) {
            formats.push(ClipboardFormat::new(
                crate::consts::RUNEBENDER_PASTEBOARD_TYPE,
                data,
            ));
        }
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
            crate::consts::RUNEBENDER_PASTEBOARD_TYPE,
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
                (crate::consts::RUNEBENDER_PASTEBOARD_TYPE, Some(data)) => String::from_utf8(data)
                    .ok()
                    .and_then(|s| crate::clipboard::from_json(&s)),
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
        match cmd {
            c if c.is(consts::cmd::SELECT_ALL) => data.session_mut().select_all(),
            c if c.is(consts::cmd::DESELECT_ALL) => data.session_mut().selection.clear(),
            c if c.is(consts::cmd::DELETE) => data.session_mut().delete_selection(),
            c if c.is(consts::cmd::ADD_GUIDE) => {
                let point = cmd.get_unchecked(consts::cmd::ADD_GUIDE);
                data.session_mut().add_guide(*point);
                return (true, Some(EditType::Normal));
            }
            c if c.is(consts::cmd::TOGGLE_GUIDE) => {
                let consts::cmd::ToggleGuideCmdArgs { id, pos } =
                    cmd.get_unchecked(consts::cmd::TOGGLE_GUIDE);
                data.session_mut().toggle_guide(*id, *pos);
                return (true, Some(EditType::Normal));
            }
            c if c.is(druid::commands::COPY) => self.do_copy(&data.session),
            c if c.is(druid::commands::UNDO) => {
                if let Some(prev) = self.do_undo() {
                    //HACK: because zoom & offset is part of data, and we don't
                    //want to jump around during undo/redo, we always manually
                    //reuse the current viewport when handling these actions.
                    let saved_viewport = data.session.viewport;
                    data.session = prev.clone();
                    data.session_mut().viewport = saved_viewport;
                }
            }
            c if c.is(druid::commands::REDO) => {
                if let Some(next) = self.do_redo() {
                    let saved_viewport = data.session.viewport;
                    data.session = next.clone();
                    data.session_mut().viewport = saved_viewport;
                }
            }
            c if c.is(consts::cmd::ALIGN_SELECTION) => {
                data.session_mut().align_selection();
                return (true, Some(EditType::Normal));
            }
            c if c.is(consts::cmd::NUDGE_SELECTION) => {
                let nudge = c.get_unchecked(consts::cmd::NUDGE_SELECTION);
                data.session_mut().nudge_selection(*nudge);
                return (true, Some(EditType::Normal));
            }
            c if c.is(consts::cmd::ADJUST_SIDEBEARING) => {
                let adjust = c.get_unchecked(consts::cmd::ADJUST_SIDEBEARING);
                data.session_mut()
                    .adjust_sidebearing(adjust.delta, adjust.is_left);
                return (true, Some(EditType::Normal));
            }

            c if c.is(consts::cmd::SCALE_SELECTION) => {
                let consts::cmd::ScaleSelectionArgs { scale, origin } =
                    c.get_unchecked(consts::cmd::SCALE_SELECTION);
                data.session_mut().scale_selection(*scale, *origin);
            }
            c if c.is(consts::cmd::REVERSE_CONTOURS) => {
                data.session_mut().reverse_contours();
                return (true, Some(EditType::Normal));
            }
            // all unhandled commands:
            _ => return (false, None),
        }

        // the default: commands with an `EditType` return explicitly.
        (true, None)
    }

    fn set_tool(&mut self, tool: Box<dyn Tool>) {
        self.draw_filled_outlines = tool.name() == "Preview";
        self.tool = tool;
        self.mouse.reset();
        self.tool.init_mouse(&mut self.mouse);
    }

    fn toggle_temporary_preview(
        &mut self,
        ctx: &mut EventCtx,
        data: &mut EditorState,
        turn_on: bool,
    ) -> Option<EditType> {
        self.draw_filled_outlines = turn_on;
        self.mouse.reset();
        let edit = if turn_on {
            self.temp_preview = Some(Box::new(Preview::default()));
            self.tool.cancel(&mut self.mouse, ctx, data.session_mut())
        } else {
            self.temp_preview = None;
            None
        };
        let tool = self.temp_preview.as_mut().unwrap_or(&mut self.tool);
        tool.init_mouse(&mut self.mouse);
        ctx.set_cursor(&tool.default_cursor());
        edit
    }
}

impl Widget<EditorState> for Editor {
    fn paint(&mut self, ctx: &mut PaintCtx, data: &EditorState, env: &Env) {
        let rect = (CANVAS_SIZE * data.session.viewport.zoom).to_rect();
        ctx.fill(rect, &env.get(theme::GLYPH_LIST_BACKGROUND));

        draw::draw_session(
            ctx,
            env,
            data.session.viewport,
            ctx.region().bounding_box(),
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
            Event::Command(cmd) => {
                if cmd.is(consts::cmd::TAKE_FOCUS) {
                    ctx.request_focus();
                    ctx.set_handled();
                    None
                } else if let Some(tool) = cmd.get(consts::cmd::SET_TOOL) {
                    let tool = crate::tools::tool_for_id(tool).unwrap();
                    ctx.set_cursor(&tool.default_cursor());
                    self.set_tool(tool);
                    None
                } else if let Some(flag) = cmd.get(consts::cmd::TOGGLE_PREVIEW_TOOL) {
                    // we don't toggle preview if we're actually *in* preview
                    if self.tool.name() != "Preview" {
                        self.toggle_temporary_preview(ctx, data, *flag)
                    } else {
                        None
                    }
                } else {
                    let (handled, edit) = self.handle_cmd(cmd, data);
                    if handled {
                        ctx.set_handled();
                        ctx.request_paint();
                    }
                    edit
                }
            }
            Event::KeyDown(k) if k.key == KbKey::Escape => {
                data.session_mut().selection.clear();
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
