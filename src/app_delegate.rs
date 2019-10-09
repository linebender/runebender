//! The `AppDelegate`.

//use druid::widget::{Align, DynLabel, Padding, SizedBox};
use druid::{AppDelegate, Command, Event, LocalizedString, Selector, Widget, WindowDesc};
use norad::GlyphName;

use crate::data::{lenses, AppState};
use crate::lens2::Lens2Wrap;

pub const EDIT_GLYPH: Selector = Selector::new("runebender.open-editor-with-glyph");

pub fn make_delegate() -> AppDelegate<AppState> {
    AppDelegate::new().event_handler(|event, _data, _env, ctx| match event {
        Event::Command(ref cmd) if cmd.selector == EDIT_GLYPH => {
            let payload = cmd
                .get_object::<GlyphName>()
                .map(GlyphName::clone)
                .expect("EDIT_GLYPH has incorrect payload");

            let title = payload.to_string();

            let new_win = WindowDesc::new(move || make_editor2(payload.clone()))
                .title(LocalizedString::new("").with_placeholder(title))
                .menu(crate::menus::make_menu::<AppState>());
            let command = Command::new(druid::command::sys::NEW_WINDOW, new_win);
            ctx.submit_command(command, None);
            None
        }
        other => Some(other),
    })
}

//fn make_editor(glyph_name: GlyphName) -> impl Widget<AppState> {
//// this is just a placeholder for the actual editor
//let label = DynLabel::new(|data: &EditorState, _| data.glyph.name.as_str().to_string());
//let widget = SizedBox::new(Align::centered(Padding::uniform(5.0, label))).height(40.);
//Lens2Wrap::new(widget, lenses::app_state::EditorState(glyph_name))
//}

fn make_editor2(glyph_name: GlyphName) -> impl Widget<AppState> {
    Lens2Wrap::new(
        crate::widgets::GridInner {
            units_per_em: 1000.,
        },
        lenses::app_state::Glyph(glyph_name),
    )
}
