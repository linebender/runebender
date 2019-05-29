// Copyright 2019 the Runebender authors.

//! A quick demonstration of loading and displaying a UFO glyph.

use kurbo::{Affine, BezPath, Circle, Line, Vec2};
use norad::glyph::Glyph;

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;
//use druid_shell::window::MouseButton;

use druid::{
    UiMain, UiState,
};

#[path="../widgets/grid.rs"]
mod grid;

#[path="../widgets/glyph.rs"]
mod glyph_widget;

#[path="../widgets/editor.rs"]
mod editor;

fn build_ui(ui: &mut UiState, glyph: Glyph) {
    let root_id = editor::GlyphEditor::new(glyph).ui(ui);
    ui.set_focus(Some(root_id));
    ui.set_root(root_id);
}

fn main() {
    let glyph_path = match std::env::args().skip(1).next() {
        Some(arg) => arg,
        None => {
            eprintln!("Please pass a path to a .glif file");
            std::process::exit(1);
        }
    };

    println!("loading {}", glyph_path);
    let glyph = norad::Glyph::load(&glyph_path).expect("failed to load glyph");

    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();

    build_ui(&mut state, glyph);

    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Ufo Toy");
    let window = builder.build().expect("building window");

    window.show();
    run_loop.run();
}

