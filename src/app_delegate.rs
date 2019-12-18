//! The `AppDelegate`.

use std::sync::Arc;

use druid::{
    AppDelegate, Command, DelegateCtx, Env, Event, FileInfo, LocalizedString, Selector, Widget,
    WindowDesc, WindowId,
};

use druid::kurbo::Size;
use druid::lens::LensExt;
use druid::widget::WidgetExt;
use norad::{GlyphName, Ufo};

use crate::consts;
use crate::data::{lenses, AppState};
use crate::edit_session::EditSession;
use crate::widgets::{Controller, Editor, ScrollZoom};

pub const EDIT_GLYPH: Selector = Selector::new("runebender.open-editor-with-glyph");

#[derive(Debug, Default)]
pub struct Delegate;

impl AppDelegate<AppState> for Delegate {
    fn event(
        &mut self,
        event: Event,
        data: &mut AppState,
        _env: &Env,
        ctx: &mut DelegateCtx,
    ) -> Option<Event> {
        match event {
            Event::Command(cmd) if cmd.selector == druid::commands::OPEN_FILE => {
                let info = cmd.get_object::<FileInfo>().expect("api violation");
                match Ufo::load(info.path()) {
                    Ok(ufo) => data.workspace.set_file(ufo, info.path().to_owned()),
                    Err(e) => log::error!("failed to open file {:?}: '{:?}'", info.path(), e),
                };
                ctx.submit_command(consts::cmd::REBUILD_MENUS.into(), None);
                None
            }
            Event::Command(cmd) if cmd.selector == druid::commands::SAVE_FILE => {
                if let Some(info) = cmd.get_object::<FileInfo>() {
                    Arc::make_mut(&mut data.workspace.font).path = Some(info.path().into());
                    ctx.submit_command(consts::cmd::REBUILD_MENUS.into(), None);
                }
                if let Err(e) = data.workspace.save() {
                    log::error!("saving failed: '{}'", e);
                }
                None
            }
            Event::Command(ref cmd) if cmd.selector == EDIT_GLYPH => {
                let payload = cmd
                    .get_object::<GlyphName>()
                    .map(GlyphName::clone)
                    .expect("EDIT_GLYPH has incorrect payload");

                match data.workspace.open_glyphs.get(&payload).to_owned() {
                    Some(id) => {
                        let command = Command::new(druid::commands::SHOW_WINDOW, *id);
                        ctx.submit_command(command, None);
                    }
                    None => {
                        let session = get_or_create_session(&payload, data);
                        let new_win = WindowDesc::new(move || make_editor(&session))
                            .title(LocalizedString::new("").with_placeholder(payload.to_string()))
                            .window_size(Size::new(900.0, 800.0))
                            .menu(crate::menus::make_menu(&data));

                        let id = new_win.id;
                        let command = Command::new(druid::commands::NEW_WINDOW, new_win);
                        ctx.submit_command(command, None);

                        Arc::make_mut(&mut data.workspace.open_glyphs).insert(payload.clone(), id);
                    }
                }
                None
            }
            other => Some(other),
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

fn get_or_create_session(name: &GlyphName, data: &mut AppState) -> Arc<EditSession> {
    data.workspace
        .sessions
        .get(name)
        .cloned()
        .unwrap_or_else(|| {
            let session = Arc::new(EditSession::new(name, &data.workspace));
            Arc::make_mut(&mut data.workspace.sessions).insert(name.clone(), session.clone());
            session
        })
}

fn make_editor(session: &Arc<EditSession>) -> impl Widget<AppState> {
    Controller::new(
        ScrollZoom::new(Editor::new(session.clone()))
            .lens(AppState::workspace.then(lenses::app_state::EditorState(session.name.clone()))),
    )
}
