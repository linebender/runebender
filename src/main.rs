//! A font editor.

mod menus;

use druid::widget::{Align, Label, Padding};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

type AppState = u32;

fn main() {
    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Runebender"))
        .menu(menus::make_menu());

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(0)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    let text = LocalizedString::new("Fontville!");
    let label = Label::new(text);
    Align::centered(Padding::uniform(5.0, label))
}
