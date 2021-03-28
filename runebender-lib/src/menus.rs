//! Application menus.

use druid::commands;
use druid::platform_menus;
use druid::{
    Data, Env, FileDialogOptions, FileSpec, KbKey, LocalizedString, Menu, MenuItem, Point, SysMods,
    WindowId,
};

use crate::consts;
use crate::data::{AppState, EditorState};

pub const UFO_FILE_TYPE: FileSpec = FileSpec::new("Font Object", &["ufo"]);

/// Context menu's inner menu must have type T == the root app state.
pub fn make_context_menu(data: &EditorState, pos: Point) -> Menu<AppState> {
    let mut menu = Menu::empty().entry(
        MenuItem::new(LocalizedString::new("menu-item-add-guide").with_placeholder("Add Guide"))
            .on_activate(move |ctx, _, _| ctx.submit_command(consts::cmd::ADD_GUIDE.with(pos))),
    );

    // only show 'toggle guide' if a guide is selected
    if data.session.selection.len() == 1 && data.session.selection.iter().all(|s| s.is_guide()) {
        let id = *data.session.selection.iter().next().unwrap();
        menu = menu.entry(
            MenuItem::new(
                LocalizedString::new("menu-item-toggle-guide")
                    .with_placeholder("Toggle Guide Orientation"),
            )
            .on_activate(move |ctx, _, _| {
                let args = consts::cmd::ToggleGuideCmdArgs { id, pos };
                ctx.submit_command(consts::cmd::TOGGLE_GUIDE.with(args))
            }),
        );
    }
    menu
}

/// The main window/app menu.
pub fn make_menu(_window: Option<WindowId>, data: &AppState, _: &Env) -> Menu<AppState> {
    let menu = if cfg!(target_os = "macos") {
        Menu::empty().entry(platform_menus::mac::application::default())
    } else {
        Menu::empty()
    };

    menu.entry(file_menu(data))
        .entry(edit_menu())
        .entry(view_menu())
        .entry(glyph_menu(data))
        .entry(paths_menu())
        .entry(window_menu(data))
}

fn file_menu(data: &AppState) -> Menu<AppState> {
    let has_path = data.workspace.font.path.is_some();
    let mut menu = Menu::new(LocalizedString::new("common-menu-file-menu"))
        .entry(platform_menus::mac::file::new_file().enabled(false))
        .entry(
            MenuItem::new(LocalizedString::new("common-menu-file-open"))
                .on_activate(|ctx, _, _| {
                    ctx.submit_command(
                        commands::SHOW_OPEN_PANEL
                            .with(FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE])),
                    )
                })
                .hotkey(SysMods::Cmd, "o"),
        )
        .separator()
        .entry(platform_menus::mac::file::close());
    if has_path {
        menu = menu.entry(platform_menus::mac::file::save()).entry(
            MenuItem::new(LocalizedString::new("common-menu-file-save-as"))
                .on_activate(|ctx, _, _| {
                    ctx.submit_command(
                        commands::SHOW_SAVE_PANEL
                            .with(FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE])),
                    )
                })
                .hotkey(SysMods::CmdShift, "S"),
        );
    } else {
        menu = menu.entry(
            MenuItem::new(LocalizedString::new("common-menu-file-save-as"))
                .on_activate(|ctx, _, _| {
                    ctx.submit_command(
                        commands::SHOW_SAVE_PANEL
                            .with(FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE])),
                    )
                })
                .hotkey(SysMods::Cmd, "s"),
        );
    }
    menu.separator()
        .entry(platform_menus::mac::file::page_setup().enabled(false))
        .entry(platform_menus::mac::file::print().enabled(false))
}

fn edit_menu<T: Data>() -> Menu<T> {
    Menu::new(LocalizedString::new("common-menu-edit-menu"))
        .entry(platform_menus::common::undo())
        .entry(platform_menus::common::redo())
        .separator()
        .entry(platform_menus::common::cut().enabled(false))
        .entry(platform_menus::common::copy())
        .entry(platform_menus::common::paste())
        .entry(
            MenuItem::new(LocalizedString::new("menu-item-delete").with_placeholder("Delete"))
                .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::DELETE)),
        )
        .separator()
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-select-all").with_placeholder("Select All"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::SELECT_ALL))
            .hotkey(SysMods::Cmd, "a"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-deselect-all").with_placeholder("Deselect All"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::DESELECT_ALL))
            .hotkey(SysMods::AltCmd, "A"),
        )
}

fn view_menu<T: Data>() -> Menu<T> {
    Menu::new(LocalizedString::new("menu-view-menu").with_placeholder("View"))
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-increase-zoom").with_placeholder("Zoom In"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::ZOOM_IN))
            .hotkey(SysMods::Cmd, "+"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-decrease-zoom").with_placeholder("Zoom Out"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::ZOOM_OUT))
            .hotkey(SysMods::Cmd, "-"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-reset-zoom").with_placeholder("Reset Zoom"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::ZOOM_DEFAULT))
            .hotkey(SysMods::Cmd, "0"),
        )
}

fn glyph_menu(_data: &AppState) -> Menu<AppState> {
    Menu::new(LocalizedString::new("menu-glyph-menu").with_placeholder("Glyph"))
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-new-glyph").with_placeholder("New Glyph"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::NEW_GLYPH))
            .hotkey(SysMods::CmdShift, "N"),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-delete-glyph").with_placeholder("Delete Glyph"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::DELETE_SELECTED_GLYPH))
            .hotkey(SysMods::Cmd, KbKey::Backspace)
            .enabled_if(|data: &AppState, _| data.workspace.selected.is_some()),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-add-component").with_placeholder("Add Component"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::ADD_COMPONENT))
            .hotkey(SysMods::CmdShift, "C")
            .enabled(false),
        )
        .refresh_on(|old, new, _| old.workspace.selected != new.workspace.selected)
}

fn paths_menu<T: Data>() -> Menu<T> {
    Menu::new(LocalizedString::new("menu-paths-menu").with_placeholder("Paths"))
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-reverse-contours")
                    .with_placeholder("Reverse Contours"),
            )
            .on_activate(|ctx, _, _| {
                ctx.submit_command(
                    consts::cmd::REVERSE_CONTOURS,
                    // TODO: hotkey on mac should be ctrl-alt-cmd R, but what about non-mac?
                )
            }),
        )
        .entry(
            MenuItem::new(
                LocalizedString::new("menu-item-align-selection")
                    .with_placeholder("Align Selection"),
            )
            .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::ALIGN_SELECTION))
            .hotkey(SysMods::CmdShift, "A"),
        )
}

fn window_menu(_app_state: &AppState) -> Menu<AppState> {
    Menu::new(LocalizedString::new("menu-window-menu").with_placeholder("Window")).entry(
        MenuItem::new(
            LocalizedString::new("menu-item-new-preview").with_placeholder("New Preview"),
        )
        .on_activate(|ctx, _, _| ctx.submit_command(consts::cmd::NEW_PREVIEW_WINDOW))
        .hotkey(SysMods::AltCmd, "p"),
    )
}
