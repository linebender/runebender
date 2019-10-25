//! The `AppDelegate`.

use std::sync::Arc;

use druid::{AppDelegate, Command, Event, LocalizedString, Selector, Widget, WindowDesc};
use norad::GlyphName;

use crate::data::{lenses, AppState, OpenGlyph};
use crate::edit_session::EditSession;
use crate::lens2::Lens2Wrap;
use crate::widgets::{Editor, ScrollZoom};

pub const EDIT_GLYPH: Selector = Selector::new("runebender.open-editor-with-glyph");

pub fn make_delegate() -> AppDelegate<AppState> {
    AppDelegate::new().event_handler(|event, data: &mut AppState, _env, ctx| match event {
        Event::Command(ref cmd) if cmd.selector == EDIT_GLYPH => {
            let payload = cmd
                .get_object::<GlyphName>()
                .map(GlyphName::clone)
                .expect("EDIT_GLYPH has incorrect payload");

            match data.open_glyphs.get(&payload).to_owned() {
                Some(OpenGlyph::Pending) => (),
                //TODO: when we have a window-connect event, and can stash window id, fix this
                Some(OpenGlyph::Window(_window_id)) => (), // we want to show this window,
                None => {
                    let title = payload.to_string();
                    let payload2 = payload.clone();

                    let new_win = WindowDesc::new(move || make_editor(payload2.clone()))
                        .title(LocalizedString::new("").with_placeholder(title))
                        .menu(crate::menus::make_menu::<AppState>());
                    let command = Command::new(druid::command::sys::NEW_WINDOW, new_win);
                    ctx.submit_command(command, None);
                    Arc::make_mut(&mut data.open_glyphs)
                        .insert(payload.clone(), OpenGlyph::Pending);
                    let session = EditSession::new(&payload, &data.file.object);
                    Arc::make_mut(&mut data.sessions).insert(payload.clone(), session);
                }
            }
            None
        }
        other => Some(other),
    })
}

fn make_editor(glyph: GlyphName) -> impl Widget<AppState> {
    Lens2Wrap::new(
        ScrollZoom::new(Editor::new()),
        lenses::app_state::EditorState(glyph),
    )
}
