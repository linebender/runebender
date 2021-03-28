//! The `AppDelegate`.

use std::sync::Arc;

use druid::{
    AppDelegate, Command, DelegateCtx, Handled, Selector, Target, Widget, WindowDesc, WindowId,
};

use druid::kurbo::Line;
use druid::lens::LensExt;
use druid::text::format::ParseFormatter;
use druid::widget::{prelude::*, Flex, Label, Painter, TextBox, WidgetExt};
use norad::{GlyphName, Ufo};

use crate::consts;
use crate::data::{AppState, PreviewSession, PreviewState, Workspace};
use crate::edit_session::{EditSession, SessionId};
use crate::widgets::{Editor, EditorController, Preview, ScrollZoom};

pub const EDIT_GLYPH: Selector<GlyphName> = Selector::new("runebender.open-editor-with-glyph");

#[derive(Debug, Default)]
pub struct Delegate;

impl AppDelegate<AppState> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut AppState,
        _env: &Env,
    ) -> Handled {
        if let Some(info) = cmd.get(druid::commands::OPEN_FILE) {
            match Ufo::load(info.path()) {
                Ok(ufo) => data.workspace.set_file(ufo, info.path().to_owned()),
                Err(e) => log::error!("failed to open file {:?}: '{:?}'", info.path(), e),
            };
            Handled::Yes
        } else if cmd.is(druid::commands::SAVE_FILE) {
            if let Err(e) = data.workspace.save() {
                log::error!("saving failed: '{}'", e);
            }
            Handled::Yes
        } else if let Some(info) = cmd.get(druid::commands::SAVE_FILE_AS) {
            Arc::make_mut(&mut data.workspace.font).path = Some(info.path().into());
            if let Err(e) = data.workspace.save() {
                log::error!("saving failed: '{}'", e);
            }
            Handled::Yes
        } else if cmd.is(consts::cmd::NEW_GLYPH) {
            let new_glyph_name = data.workspace.add_new_glyph();
            data.workspace.selected = Some(new_glyph_name);
            Handled::Yes
        } else if cmd.is(consts::cmd::DELETE_SELECTED_GLYPH) {
            data.workspace.delete_selected_glyph();
            Handled::Yes
        } else if let Some(consts::cmd::RenameGlyphArgs { old, new }) =
            cmd.get(consts::cmd::RENAME_GLYPH)
        {
            data.workspace.rename_glyph(old.clone(), new.clone());
            Handled::Yes
        } else if cmd.is(consts::cmd::NEW_PREVIEW_WINDOW) {
            let session_id = data.workspace.new_preview_session();
            let new_win = WindowDesc::new(make_preview(session_id))
                .title("Preview")
                .window_size(Size::new(800.0, 400.0))
                .menu(crate::menus::make_menu);
            ctx.new_window(new_win);
            Handled::Yes
        } else if let Some(payload) = cmd.get(EDIT_GLYPH) {
            match data.workspace.open_glyphs.get(payload).to_owned() {
                Some(id) => {
                    ctx.submit_command(druid::commands::SHOW_WINDOW.to(*id));
                }
                None => {
                    let session = data.workspace.get_or_create_session(&payload);
                    let session_id = session.id;
                    let new_win = WindowDesc::new(make_editor(&session))
                        .title(move |d: &AppState, _: &_| {
                            d.workspace
                                .sessions
                                .get(&session_id)
                                .map(|s| s.name.to_string())
                                .unwrap_or_else(|| "Unknown".to_string())
                        })
                        .window_size(Size::new(900.0, 800.0))
                        .menu(crate::menus::make_menu);

                    let id = new_win.id;
                    ctx.new_window(new_win);

                    Arc::make_mut(&mut data.workspace.open_glyphs).insert(payload.clone(), id);
                }
            }
            Handled::Yes
        } else {
            Handled::No
        }
    }

    /// The handler for window deletion events.
    /// This function is called after a window has been removed.
    fn window_removed(
        &mut self,
        id: WindowId,
        data: &mut AppState,
        _env: &Env,
        _ctx: &mut DelegateCtx,
    ) {
        let to_remove = data
            .workspace
            .open_glyphs
            .iter()
            .find(|(_k, v)| v == &&id)
            .map(|(k, _v)| k.clone());
        match to_remove {
            Some(open_glyph) => {
                log::info!("removing '{}' from open list", open_glyph);
                Arc::make_mut(&mut data.workspace.open_glyphs).remove(&open_glyph);
            }
            None => log::info!("window {:?} is not an editor window", id),
        }
    }
}

fn make_editor(session: &Arc<EditSession>) -> impl Widget<AppState> {
    crate::theme::wrap_in_theme_loader(
        EditorController::new(ScrollZoom::new(Editor::new(session.clone())))
            .lens(AppState::workspace.then(Workspace::editor_state(session.id))),
    )
}

fn make_preview(session: SessionId) -> impl Widget<AppState> {
    // this is duplicated in main.rs
    let hline_painter = Painter::new(|ctx, _: &PreviewState, env| {
        let rect = ctx.size().to_rect();
        let max_y = rect.height() - 0.5;
        let line = Line::new((0.0, max_y), (rect.width(), max_y));

        ctx.fill(rect, &env.get(crate::theme::GLYPH_LIST_BACKGROUND));
        ctx.stroke(line, &env.get(crate::theme::SIDEBAR_EDGE_STROKE), 1.0);
    });
    crate::theme::wrap_in_theme_loader(
        Flex::column()
            .cross_axis_alignment(druid::widget::CrossAxisAlignment::Start)
            .with_child(
                Flex::row()
                    .cross_axis_alignment(druid::widget::CrossAxisAlignment::Baseline)
                    .with_child(Label::new("Font Size:"))
                    .with_default_spacer()
                    .with_child(
                        TextBox::new()
                            .with_formatter(ParseFormatter::new())
                            .lens(PreviewState::session.then(PreviewSession::font_size)),
                    )
                    .with_default_spacer()
                    .with_flex_child(
                        TextBox::multiline()
                            .expand_width()
                            .lens(PreviewState::session.then(PreviewSession::text)),
                        1.0,
                    )
                    .padding(8.0)
                    .background(hline_painter),
            )
            .with_flex_child(Preview::default().padding(8.0), 1.0)
            .background(crate::theme::GLYPH_LIST_BACKGROUND)
            .lens(AppState::workspace.then(Workspace::preview_state(session))),
    )
}
