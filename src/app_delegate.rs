//! The `AppDelegate`.

use std::sync::Arc;

use druid::{AppDelegate, Command, Event, LocalizedString, Selector, WindowDesc};
use norad::GlyphName;

use crate::data::{AppState, OpenGlyph};
use crate::widgets::Editor;

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

                    let new_win = WindowDesc::new(move || Editor::new(payload2.clone()))
                        .title(LocalizedString::new("").with_placeholder(title))
                        .menu(crate::menus::make_menu::<AppState>());
                    let command = Command::new(druid::command::sys::NEW_WINDOW, new_win);
                    ctx.submit_command(command, None);
                    Arc::make_mut(&mut data.open_glyphs)
                        .insert(payload.clone(), OpenGlyph::Pending);
                }
            }
            None
        }
        other => Some(other),
    })
}
