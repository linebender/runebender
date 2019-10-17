//! The `AppDelegate`.

use druid::{AppDelegate, Command, Event, LocalizedString, Selector, WindowDesc};
use norad::GlyphName;

use crate::data::AppState;
use crate::widgets::Editor;

pub const EDIT_GLYPH: Selector = Selector::new("runebender.open-editor-with-glyph");

pub fn make_delegate() -> AppDelegate<AppState> {
    AppDelegate::new().event_handler(|event, _data, _env, ctx| match event {
        Event::Command(ref cmd) if cmd.selector == EDIT_GLYPH => {
            let payload = cmd
                .get_object::<GlyphName>()
                .map(GlyphName::clone)
                .expect("EDIT_GLYPH has incorrect payload");

            let title = payload.to_string();

            let new_win = WindowDesc::new(move || Editor::new(payload.clone()))
                .title(LocalizedString::new("").with_placeholder(title))
                .menu(crate::menus::make_menu::<AppState>());
            let command = Command::new(druid::command::sys::NEW_WINDOW, new_win);
            ctx.submit_command(command, None);
            None
        }
        other => Some(other),
    })
}
