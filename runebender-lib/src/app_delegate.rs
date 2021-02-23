//! The `AppDelegate`.

use std::sync::Arc;

use druid::{
    AppDelegate, Command, DelegateCtx, Env, Handled, Selector, Target, Widget, WindowDesc, WindowId,
};

use druid::kurbo::Size;
use druid::lens::LensExt;
use druid::widget::WidgetExt;
use norad::{GlyphName, Ufo};

use crate::consts;
use crate::data::{AppState, Workspace};
use crate::edit_session::EditSession;
use crate::widgets::{Editor, EditorController, RootWindowController, ScrollZoom};

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
            ctx.submit_command(consts::cmd::REBUILD_MENUS);
            Handled::Yes
        } else if cmd.is(druid::commands::SAVE_FILE) {
            if let Err(e) = data.workspace.save() {
                log::error!("saving failed: '{}'", e);
            }
            Handled::Yes
        } else if let Some(info) = cmd.get(druid::commands::SAVE_FILE_AS) {
            Arc::make_mut(&mut data.workspace.font).path = Some(info.path().into());
            ctx.submit_command(consts::cmd::REBUILD_MENUS);
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
                        .menu(crate::menus::make_menu(&data));

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
            .lens(AppState::workspace.then(Workspace::editor_state(session.id)))
            .controller(RootWindowController::default()),
    )
}
