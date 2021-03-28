//! A font editor.

use druid::kurbo::Line;
use druid::widget::{Button, Flex, Label, Painter, Scroll, WidgetExt};
use druid::{AppLauncher, Env, LocalizedString, RenderContext, Size, Widget, WindowDesc};

use runebender_lib::data::{AppState, Workspace};
use runebender_lib::widgets::{self, GlyphGrid, ModalHost, Sidebar};
use runebender_lib::{menus, theme, Delegate};

fn main() {
    let state = get_initial_state();

    let main_window = WindowDesc::new(make_ui())
        .title(LocalizedString::new("Runebender"))
        .menu(menus::make_menu)
        .window_size(Size::new(900.0, 800.0));

    AppLauncher::with_window(main_window)
        .delegate(Delegate::default())
        .configure_env(|env, _| theme::configure_env(env))
        .log_to_console()
        .launch(state)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    // paint a line under the top title bar
    let hline_painter = Painter::new(|ctx, _: &Workspace, env| {
        let rect = ctx.size().to_rect();
        let max_y = rect.height() - 0.5;
        let line = Line::new((0.0, max_y), (rect.width(), max_y));

        ctx.fill(rect, &env.get(theme::GLYPH_LIST_BACKGROUND));
        ctx.stroke(line, &env.get(theme::SIDEBAR_EDGE_STROKE), 1.0);
    });

    let label = Label::new(|data: &Workspace, _: &Env| {
        format!("{} {}", data.info.family_name, data.info.style_name)
    });

    let button = Button::new("(edit)").on_click(|ctx, _data, _env| {
        let cmd = ModalHost::make_modal_command(crate::widgets::font_info);
        ctx.submit_command(cmd);
    });

    let main_view = Flex::column()
        .with_child(
            Flex::row()
                .with_child(label)
                .with_spacer(8.0)
                .with_child(button)
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
        );

    crate::theme::wrap_in_theme_loader(ModalHost::new(main_view).lens(AppState::workspace))
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
        (runebender_lib::create_blank_font(), None)
    };

    let mut workspace = Workspace::default();
    workspace.set_file(font_file, path);
    AppState { workspace }
}
