// Copyright 2019 the Runebender authors.

//! Load a layer from a UFO file and display all glyphs

use std::any::Any;
use norad::{Ufo, Glyph};

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;

use druid::widget::EventForwarder;
use druid::{
    BoxConstraints, HandlerCtx, Id, LayoutCtx, LayoutResult, Ui, UiMain, UiState, Widget,
};

#[path="../widgets/grid.rs"]
mod grid;

#[path="../widgets/glyph.rs"]
mod glyph_widget;

#[path="../widgets/editor.rs"]
mod editor;

#[derive(Debug, Clone)]
pub enum Action {
    Edit(Glyph),
    EndEdit,
}

#[derive(Debug, Default)]
struct EditorState {
    active_editor: Option<Id>,
}

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
        .map(|g| {
            let mut action = Action::Edit(g.clone());
            let widget = glyph_widget::GlyphWidget::new(g).ui(&mut state);
            state.add_listener(widget, move |_: &mut bool, mut ctx| {
                ctx.poke_up(&mut action);
            });
            widget
        })
    .collect::<Vec<_>>();

    let grid = grid.ui(&glyph_widgets, &mut state);
    let forwarder = EventForwarder::<Action>::new().ui(grid, &mut state);
    state.set_root(forwarder);

    let mut editor_state = EditorState::default();

    state.add_listener(
        forwarder,
        move |action: &mut Action, mut ctx| match action {
            Action::Edit(glyph) => {
                let edit_widget = editor::GlyphEditor::new(glyph.clone()).ui(&mut ctx);
                ctx.add_listener(edit_widget, move |_: &mut bool, mut ctx| {
                    ctx.poke_up(&mut Action::EndEdit);
                });
                editor_state.active_editor = Some(edit_widget);
                ctx.remove_child(forwarder, grid);
                ctx.append_child(forwarder, edit_widget);
                ctx.set_focus(Some(edit_widget));
            }
            Action::EndEdit => {
                let editor = editor_state.active_editor.take().unwrap();
                ctx.remove_child(forwarder, editor);
                ctx.append_child(forwarder, grid);
                ctx.set_focus(Some(editor));
            }
        });

    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Ufo Toy");
    let window = builder.build().expect("building window");

    window.show();
    run_loop.run();
}

