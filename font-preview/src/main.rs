use std::sync::Arc;

use druid::{AppLauncher, Data, LocalizedString, Size, Widget, WindowDesc};
use harfbuzz_rs::{Blob, Face, Font, UnicodeBuffer};
use norad::Ufo;

mod opentype;
mod preview_widget;
use opentype::VirtualFont;

#[derive(Debug, Clone, Data)]
struct AppData {
    text: String,
    #[data(ignore)]
    font: Arc<VirtualFont>,
}
fn main() {
    let data = get_initial_state();
    //test_some_other_font();
    //opentype::test(&data.font);
    //test_harfbuzz_stuff(&data);

    let main_window = WindowDesc::new(make_ui)
        .title(LocalizedString::new("Font Preview"))
        .window_size(Size::new(900.0, 600.0));

    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}

fn make_ui() -> impl Widget<AppData> {
    preview_widget::Preview::new(48.0)
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

//fn test_harfbuzz_stuff(data: &AppData) {
////let virtual_font = opentype::VirtualFont::new(Ufo::clone(&data.font));
//let face = Face::from_table_func(|tag| {
//eprintln!("{}", tag);
//match tag.to_bytes() {
//CMAP => Some(Blob::with_bytes(data.font.cmap()).to_shared()),
//HHEA => Some(Blob::with_bytes(data.font.hhea()).to_shared()),
//HMTX => Some(Blob::with_bytes(data.font.hmtx()).to_shared()),
//_ => None,
//}
//});

//let font = Font::new(face);
//let buffer = UnicodeBuffer::new().add_str("aA");
//let output = harfbuzz_rs::shape(&font, buffer, &[]);

//dbg!(&output);

//// The results of the shaping operation are stored in the `output` buffer.

////let positions = output.get_glyph_positions();
////let infos = output.get_glyph_infos();
//}
