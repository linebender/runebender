// Copyright 2019 the Runebender authors.

//! Load a layer from a UFO file and display all glyphs

use norad::Ufo;

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::{
    UiMain, UiState,
};

#[path="../widgets/grid.rs"]
mod grid;

#[path="../widgets/glyph.rs"]
mod glyph_widget;

fn main() {
    let glyph_path = match std::env::args().skip(1).next() {
        Some(arg) => arg,
        None => {
            eprintln!("Please pass a path to a .ufo file");
            std::process::exit(1);
        }
    };

    println!("loading {}", glyph_path);
    let mut ufo = Ufo::load(&glyph_path).expect("failed to load ufo");
    let layer = ufo.find_layer(|l| l.name == "foreground").expect("failed to find foreground layer");
    let glyph_names = layer.iter_contents().map(|(n, _)| n.clone()).collect::<Vec<_>>();
    //dbg!(glyphs);

    let glyphs = glyph_names.iter().flat_map(|n| {
        match layer.get_glyph(n) {
            Ok(g) => Some(g.to_owned()),
            Err(e) => {
                eprintln!("error loading glyph {}: {:?}", n, e);
                None
            }
        }
    }).collect::<Vec<_>>();

    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();

    let grid = grid::Grid::new((100.0, 100.0));
    let glyph_widgets = glyphs.into_iter()
        .map(|g| glyph_widget::GlyphWidget::new(g).ui(&mut state))
        .collect::<Vec<_>>();
    let root_id = grid.ui(&glyph_widgets, &mut state);
    state.set_root(root_id);

    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Ufo Toy");
    let window = builder.build().expect("building window");

    window.show();
    run_loop.run();
}

