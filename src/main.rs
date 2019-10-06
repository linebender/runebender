//! A font editor!

mod data;
mod lens2;
mod menus;
mod widgets;

use druid::widget::{Align, Column, DynLabel, Padding, Scroll, SizedBox};
use druid::{AppLauncher, LocalizedString, Widget, WindowDesc};

use data::{lenses, AppState};
use lens2::Lens2Wrap;

use widgets::{Controller, GlyphGrid};

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
        SizedBox::new(Align::centered(Padding::uniform(5.0, label))).height(40.),
        0.,
    );
    col.add_child(
        Scroll::new(Lens2Wrap::new(GlyphGrid::new(), lenses::app_state::Ufo)).vertical(),
        1.,
    );
    Controller::new(col)
}
