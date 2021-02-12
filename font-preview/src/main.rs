use std::sync::Arc;

use druid::widget::{Flex, TextBox};
use druid::{AppLauncher, Data, Lens, LocalizedString, Size, Widget, WidgetExt, WindowDesc};
use norad::Ufo;

mod opentype;
mod preview_widget;
use opentype::VirtualFont;

#[derive(Debug, Clone, Lens, Data)]
struct AppData {
    text: String,
    #[data(ignore)]
    font: Arc<VirtualFont>,
}
fn main() {
    let data = get_initial_state();

    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Font Preview"))
        .window_size(Size::new(900.0, 600.0));

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppData> {
    Flex::column()
        .with_child(TextBox::multiline().lens(AppData::text).center())
        .with_flex_child(preview_widget::Preview::new(48.0), 1.0)
}

/// If there was an argument passed at the command line, try to open it as a .ufo
/// file, otherwise return blank state.
fn get_initial_state() -> AppData {
    if let Some(arg) = std::env::args().nth(1) {
        let ufo = match Ufo::load(&arg) {
            Ok(ufo) => ufo,
            Err(e) => {
                eprintln!(
                    "Failed to load first arg '{}' as ufo file.\nError:'{}'",
                    arg, e
                );
                std::process::exit(1);
            }
        };
        return AppData {
            font: Arc::new(VirtualFont::new(ufo)),
            text: "abcde ABCDE".into(),
        };
    } else {
        eprintln!("missing expected argument: path to Ufo file");
        std::process::exit(1);
    };
}
