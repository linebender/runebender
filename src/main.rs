//! A font editor.

mod data;
mod menus;
mod widgets;

use druid::widget::{Align, DynLabel, Padding};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

use data::AppState;
use widgets::Controller;

fn main() {
    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Runebender"))
        .menu(menus::make_menu());

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(AppState::default())
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppState> {
    let label = DynLabel::new(|data: &AppState, _| {
        format!(
            "{:?}",
            data.file.as_ref().map(|obj| format!("{:?}", &obj.path))
        )
    });
    Controller::new(Align::centered(Padding::uniform(5.0, label)))
}
