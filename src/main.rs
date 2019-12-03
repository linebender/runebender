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
    let state = get_initial_state();

    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Runebender"))
        .menu(menus::make_menu(&state));

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
    let (font_file, path) = if let Some(arg) = std::env::args().skip(1).next() {
        match norad::Ufo::load(&arg) {
            Ok(ufo) => (ufo, Some(std::path::PathBuf::from(arg))),
            Err(e) => {
                eprintln!(
                    "Failed to load first arg '{}' as ufo file.\nError:'{}'",
                    arg, e
                );
                std::process::exit(1);
            }
        }
    } else {
        (create_blank_font(), None)
    };

    let mut state = AppState::default();
    state.set_file(font_file, path);
    state
}

/// temporary; creates a new blank  font with some placeholder glyphs.
fn create_blank_font() -> norad::Ufo {
    let mut ufo = norad::Ufo::new(norad::MetaInfo::default());
    let a_ = 'a' as u32;
    #[allow(non_snake_case)]
    let A_ = 'A' as u32;

    let layer = ufo.get_default_layer_mut().unwrap();
    (0..25)
        .map(|i| std::char::from_u32(a_ + i).unwrap())
        .chain((0..25).map(|i| std::char::from_u32(A_ + i).unwrap()))
        .map(|chr| norad::Glyph::new_named(chr.to_string()))
        .for_each(|glyph| layer.insert_glyph(glyph));
    ufo
}
