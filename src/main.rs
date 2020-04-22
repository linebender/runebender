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
mod glyph_names;
mod guides;
mod menus;
mod mouse;
mod path;
mod plist;
mod theme;
mod tools;
mod undo;
pub mod widgets;

use druid::kurbo::Line;
use druid::widget::{Flex, Label, Painter, Scroll, WidgetExt};
use druid::{AppLauncher, Env, LocalizedString, RenderContext, Size, Widget, WindowDesc};

use data::{AppState, Workspace};

use widgets::{GlyphGrid, RootWindowController, Sidebar};

fn main() {
    let state = get_initial_state();

    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Runebender"))
        .menu(menus::make_menu(&state))
        .window_size(Size::new(900.0, 800.0));

    AppLauncher::with_window(main_window)
        .delegate(app_delegate::Delegate::default())
        .configure_env(|env, _| theme::configure_env(env))
        .use_simple_logger()
        .launch(state)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    // paint a line under the top title bar
    let hline_painter = Painter::new(|ctx, _: &Workspace, env| {
        let max_y = ctx.size().height - 0.5;
        let line = Line::new((0.0, max_y), (ctx.size().width, max_y));
        ctx.stroke(line, &env.get(theme::SIDEBAR_EDGE_STROKE), 1.0);
    });

    let label = Label::new(|data: &Workspace, _: &Env| {
        data.font
            .ufo
            .font_info
            .as_ref()
            .and_then(|info| info.family_name.clone())
            .unwrap_or_else(|| "Untitled".to_string())
    });

    Flex::column()
        .with_child(
            label
                .padding(5.0)
                .center()
                .fix_height(40.)
                .expand_width()
                .background(hline_painter),
        )
        .with_flex_child(
            Flex::row()
                .with_child(Sidebar::new().fix_width(180.))
                .with_flex_child(Scroll::new(GlyphGrid::new()).vertical().expand_width(), 1.0),
            1.,
        )
        .lens(AppState::workspace)
        .controller(RootWindowController::default())
}

/// If there was an argument passed at the command line, try to open it as a .ufo
/// file, otherwise return blank state.
fn get_initial_state() -> AppState {
    let (font_file, path) = if let Some(arg) = std::env::args().nth(1) {
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

    let mut workspace = Workspace::default();
    workspace.set_file(font_file, path);
    AppState { workspace }
}

/// temporary; creates a new blank  font with some placeholder glyphs.
fn create_blank_font() -> norad::Ufo {
    let mut ufo = norad::Ufo::new();
    let a_ = 'a' as u32;
    #[allow(non_snake_case)]
    let A_ = 'A' as u32;

    let layer = ufo.get_default_layer_mut().unwrap();
    (0..26)
        .map(|i| std::char::from_u32(a_ + i).unwrap())
        .chain((0..26).map(|i| std::char::from_u32(A_ + i).unwrap()))
        .map(|chr| {
            let mut glyph = norad::Glyph::new_named(chr.to_string());
            glyph.codepoints = Some(vec![chr]);
            glyph
        })
        .for_each(|glyph| layer.insert_glyph(glyph));
    ufo
}
