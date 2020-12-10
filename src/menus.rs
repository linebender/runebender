//! Application menus.

use druid::commands;
use druid::platform_menus;
use druid::{
    Data, FileDialogOptions, FileSpec, KbKey, LocalizedString, MenuDesc, MenuItem, Point, SysMods,
};

use crate::consts;
use crate::data::{AppState, EditorState};

pub const UFO_FILE_TYPE: FileSpec = FileSpec::new("Font Object", &["ufo"]);

/// Context menu's inner menu must have type T == the root app state.
pub fn make_context_menu(data: &EditorState, pos: Point) -> MenuDesc<AppState> {
    let mut menu = MenuDesc::empty().append(MenuItem::new(
        LocalizedString::new("menu-item-add-guide").with_placeholder("Add Guide"),
        consts::cmd::ADD_GUIDE.with(pos),
    ));

    // only show 'toggle guide' if a guide is selected
    if data.session.selection.len() == 1 && data.session.selection.iter().all(|s| s.is_guide()) {
        let id = *data.session.selection.iter().next().unwrap();
        let args = consts::cmd::ToggleGuideCmdArgs { id, pos };
        menu = menu.append(MenuItem::new(
            LocalizedString::new("menu-item-toggle-guide")
                .with_placeholder("Toggle Guide Orientation"),
            consts::cmd::TOGGLE_GUIDE.with(args),
        ));
    }
    menu
}

/// The main window/app menu.
#[allow(unused_mut)]
pub(crate) fn make_menu(data: &AppState) -> MenuDesc<AppState> {
    let mut menu = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        menu = menu.append(platform_menus::mac::application::default());
    }

    menu.append(file_menu(data))
        .append(edit_menu())
        .append(view_menu())
        .append(glyph_menu(data))
        .append(paths_menu())
        .append(tools_menu())
}

fn file_menu(data: &AppState) -> MenuDesc<AppState> {
    let has_path = data.workspace.font.path.is_some();
    let mut menu = MenuDesc::new(LocalizedString::new("common-menu-file-menu"))
        .append(platform_menus::mac::file::new_file().disabled())
        .append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-open"),
                commands::SHOW_OPEN_PANEL
                    .with(FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE])),
            )
            .hotkey(SysMods::Cmd, "o"),
        )
        .append_separator()
        .append(platform_menus::mac::file::close());
    if has_path {
        menu = menu.append(platform_menus::mac::file::save()).append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-save-as"),
                commands::SHOW_SAVE_PANEL
                    .with(FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE])),
            )
            .hotkey(SysMods::CmdShift, "S"),
        );
    } else {
        menu = menu.append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-save-as"),
                commands::SHOW_SAVE_PANEL
                    .with(FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE])),
            )
            .hotkey(SysMods::Cmd, "s"),
        );
    }
    menu.append_separator()
        .append(platform_menus::mac::file::page_setup().disabled())
        .append(platform_menus::mac::file::print().disabled())
}

fn edit_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("common-menu-edit-menu"))
        .append(platform_menus::common::undo())
        .append(platform_menus::common::redo())
        .append_separator()
        .append(platform_menus::common::cut().disabled())
        .append(platform_menus::common::copy())
        .append(platform_menus::common::paste())
        .append(MenuItem::new(
            LocalizedString::new("menu-item-delete").with_placeholder("Delete"),
            consts::cmd::DELETE,
        ))
        .append_separator()
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-select-all").with_placeholder("Select All"),
                consts::cmd::SELECT_ALL,
            )
            .hotkey(SysMods::Cmd, "a"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-deselect-all").with_placeholder("Deselect All"),
                consts::cmd::DESELECT_ALL,
            )
            .hotkey(SysMods::AltCmd, "A"),
        )
}

fn view_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("menu-view-menu").with_placeholder("View"))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-increase-zoom").with_placeholder("Zoom In"),
                consts::cmd::ZOOM_IN,
            )
            .hotkey(SysMods::Cmd, "+"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-decrease-zoom").with_placeholder("Zoom Out"),
                consts::cmd::ZOOM_OUT,
            )
            .hotkey(SysMods::Cmd, "-"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-reset-zoom").with_placeholder("Reset Zoom"),
                consts::cmd::ZOOM_DEFAULT,
            )
            .hotkey(SysMods::Cmd, "0"),
        )
}

fn glyph_menu(data: &AppState) -> MenuDesc<AppState> {
    MenuDesc::new(LocalizedString::new("menu-glyph-menu").with_placeholder("Glyph"))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-new-glyph").with_placeholder("New Glyph"),
                consts::cmd::NEW_GLYPH,
            )
            .hotkey(SysMods::CmdShift, "N"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-delete-glyph").with_placeholder("Delete Glyph"),
                consts::cmd::DELETE_SELECTED_GLYPH,
            )
            .hotkey(SysMods::Cmd, KbKey::Backspace)
            .disabled_if(|| data.workspace.selected.is_none()),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-add-component").with_placeholder("Add Component"),
                consts::cmd::ADD_COMPONENT,
            )
            .hotkey(SysMods::CmdShift, "C")
            .disabled(),
        )
}

fn paths_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("menu-paths-menu").with_placeholder("Paths"))
        .append(MenuItem::new(
            LocalizedString::new("menu-item-reverse-contours").with_placeholder("Reverse Contours"),
            consts::cmd::REVERSE_CONTOURS,
            // TODO: hotkey on mac should be ctrl-alt-cmd R, but what about non-mac?
        ))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-align-selection")
                    .with_placeholder("Align Selection"),
                consts::cmd::ALIGN_SELECTION,
            )
            .hotkey(SysMods::CmdShift, "A"),
        )
}

fn tools_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("menu-tools-menu").with_placeholder("Tools"))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-select-tool").with_placeholder("Select"),
                consts::cmd::SET_TOOL.with("Select"),
            )
            .hotkey(SysMods::None, "v"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-pen-tool").with_placeholder("Pen"),
                consts::cmd::SET_TOOL.with("Pen"),
            )
            .hotkey(SysMods::None, "p"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-preview-tool").with_placeholder("Preview"),
                consts::cmd::SET_TOOL.with("Preview"),
            )
            .hotkey(SysMods::None, "h"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-rectangle-tool").with_placeholder("Rectangle"),
                consts::cmd::SET_TOOL.with("Rectangle"),
            )
            .hotkey(SysMods::None, "u"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-ellipse-tool").with_placeholder("Ellipse"),
                consts::cmd::SET_TOOL.with("Ellipse"),
            )
            .hotkey(SysMods::Shift, "u"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-knife-tool").with_placeholder("Knife"),
                consts::cmd::SET_TOOL.with("Knife"),
            )
            .hotkey(SysMods::None, "e"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-measure-tool").with_placeholder("Measure"),
                consts::cmd::SET_TOOL.with("Measure"),
            )
            .hotkey(SysMods::None, "m"),
        )
}
