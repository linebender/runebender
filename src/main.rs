//! A font editor.

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lopdf;

mod app_delegate;
mod clipboard;
mod component;
mod consts;
mod data;
mod design_space;
mod draw;
mod edit_session;
mod guides;
mod menus;
mod mouse;
mod path;
mod plist;
mod tools;
mod undo;
pub mod widgets;

use druid::widget::{DynLabel, Flex, Scroll, WidgetExt};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

use data::{lenses, AppState};

use widgets::{Controller, GlyphGrid};

fn main() {
    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Runebender"))
        .menu(menus::make_menu());

    let state = get_initial_state();

    AppLauncher::with_window(main_window)
        .delegate(app_delegate::Delegate::default())
        .use_simple_logger()
        .launch(state)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    let mut col = Flex::column();
    let label = DynLabel::new(|data: &AppState, _| {
        data.file
            .object
            .font_info
            .as_ref()
            .and_then(|info| info.family_name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    });
    col.add_child(label.padding(5.0).center().fix_height(40.), 0.);
    col.add_child(
        Scroll::new(GlyphGrid::new().lens(lenses::app_state::GlyphSet)).vertical(),
        1.,
    );
    Controller::new(col)
}

/// If there was an argument passed at the command line, try to open it as a .ufo
/// file, otherwise return blank state.
fn get_initial_state() -> AppState {
    let mut state = AppState::default();
    if let Some(arg) = std::env::args().skip(1).next() {
        match norad::Ufo::load(&arg) {
            Ok(ufo) => state.set_file(ufo, std::path::PathBuf::from(arg)),
            Err(e) => {
                eprintln!(
                    "Failed to load first arg '{}' as ufo file.\nError:'{}'",
                    arg, e
                );
                std::process::exit(1);
            }
        }
    }
    state
}
