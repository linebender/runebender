//! A font editor.

mod app_delegate;
mod component;
mod consts;
mod data;
mod design_space;
mod draw;
mod edit_session;
mod guides;
mod lens2;
mod menus;
mod mouse;
mod path;
mod tools;
mod undo;
pub mod widgets;

use druid::widget::{Align, Column, DynLabel, Padding, Scroll, SizedBox};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

use data::{lenses, AppState};
use lens2::Lens2Wrap;

use widgets::{Controller, GlyphGrid};

fn main() {
    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Runebender"))
        .menu(menus::make_menu());

    let state = get_initial_state();

    AppLauncher::with_window(main_window)
        .delegate(app_delegate::make_delegate())
        .use_simple_logger()
        .launch(state)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    let mut col = Column::new();
    let label = DynLabel::new(|data: &AppState, _| {
        data.file
            .object
            .font_info
            .as_ref()
            .and_then(|info| info.family_name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    });
    col.add_child(
        SizedBox::new(Align::centered(Padding::new(5.0, label))).height(40.),
        0.,
    );
    col.add_child(
        Scroll::new(Lens2Wrap::new(
            GlyphGrid::new(),
            lenses::app_state::GlyphSet,
        ))
        .vertical(),
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
