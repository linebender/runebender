//! The `AppDelegate`.

use std::sync::Arc;

use druid::{
    AppDelegate, Command, DelegateCtx, Env, Event, LocalizedString, Selector, Widget, WindowDesc,
    WindowId,
};

use druid::widget::WidgetExt;
use norad::GlyphName;

use crate::data::{lenses, AppState};
use crate::edit_session::EditSession;
use crate::widgets::{Editor, ScrollZoom};

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
            Event::Command(ref cmd) if cmd.selector == EDIT_GLYPH => {
                let payload = cmd
                    .get_object::<GlyphName>()
                    .map(GlyphName::clone)
                    .expect("EDIT_GLYPH has incorrect payload");

                match data.open_glyphs.get(&payload).to_owned() {
                    Some(id) => {
                        let command = Command::new(druid::commands::SHOW_WINDOW, *id);
                        ctx.submit_command(command, None);
                    }
                    None => {
                        let session = get_or_create_session(&payload, data);
                        let new_win = WindowDesc::new(move || make_editor(&session))
                            .title(LocalizedString::new("").with_placeholder(payload.to_string()))
                            .menu(crate::menus::make_menu::<AppState>());

                        let id = new_win.id;
                        let command = Command::new(druid::commands::NEW_WINDOW, new_win);
                        ctx.submit_command(command, None);

                        Arc::make_mut(&mut data.open_glyphs).insert(payload.clone(), id);
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
            .open_glyphs
            .iter()
            .find(|(_k, v)| v == &&id)
            .map(|(k, _v)| k.clone());
        match to_remove {
            Some(open_glyph) => {
                log::info!("removing '{}' from open list", open_glyph);
                Arc::make_mut(&mut data.open_glyphs).remove(&open_glyph);
            }
            None => log::info!("window {:?} is not an editor window", id),
        }
    }
}

fn get_or_create_session(name: &GlyphName, data: &mut AppState) -> EditSession {
    if let Some(session) = data.sessions.get(name) {
        return session.to_owned();
    } else {
        let session = EditSession::new(name, &data.file.object);
        Arc::make_mut(&mut data.sessions).insert(name.clone(), session.clone());
        session
    }
}

fn make_editor(session: &EditSession) -> impl Widget<AppState> {
    ScrollZoom::new(Editor::new(session.clone()))
        .lens(lenses::app_state::EditorState(session.name.clone()))
}
