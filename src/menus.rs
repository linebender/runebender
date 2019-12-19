//! Application menus.

use druid::kurbo::Point;

use druid::commands;
use druid::platform_menus;
use druid::{
    Command, Data, FileDialogOptions, FileSpec, LocalizedString, MenuDesc, MenuItem, SysMods,
};

use crate::consts;
use crate::data::{AppState, EditorState};

pub const UFO_FILE_TYPE: FileSpec = FileSpec::new("Font Object", &["ufo"]);

/// Context menu's inner menu must have type T == the root app state.
pub fn make_context_menu(data: &EditorState, pos: Point) -> MenuDesc<AppState> {
    let mut menu = MenuDesc::empty().append(MenuItem::new(
        LocalizedString::new("menu-item-add-guide").with_placeholder("Add Guide".into()),
        Command::new(consts::cmd::ADD_GUIDE, pos),
    ));

    // only show 'toggle guide' if a guide is selected
    if data.session.selection.len() == 1 && data.session.selection.iter().all(|s| s.is_guide()) {
        let id = *data.session.selection.iter().next().unwrap();
        let args = consts::cmd::ToggleGuideCmdArgs { id, pos };
        menu = menu.append(MenuItem::new(
            LocalizedString::new("menu-item-toggle-guide")
                .with_placeholder("Toggle Guide Orientation".into()),
            Command::new(consts::cmd::TOGGLE_GUIDE, args),
        ));
    }
    menu
}

/// The main window/app menu.
pub(crate) fn make_menu(data: &AppState) -> MenuDesc<AppState> {
    let mut menu = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        menu = menu.append(platform_menus::mac::application::default());
    }

    menu.append(file_menu(data))
        .append(edit_menu())
        .append(view_menu())
        .append(glyph_menu())
        .append(tools_menu())
}

/// a work around for the fact that the first windows MenuDesc has to
/// have the root data type. (:shrug:)
//pub(crate) fn make_root_menu(data: &AppState) -> MenuDesc<AppState> {
//make_menu(&data.workspace)
//}

fn file_menu(data: &AppState) -> MenuDesc<AppState> {
    let has_path = data.workspace.font.path.is_some();
    let mut menu = MenuDesc::new(LocalizedString::new("common-menu-file-menu"))
        .append(platform_menus::mac::file::new_file().disabled())
        .append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-open"),
                Command::new(
                    commands::SHOW_OPEN_PANEL,
                    FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE]),
                ),
            )
            .hotkey(SysMods::Cmd, "o"),
        )
        .append_separator()
        .append(platform_menus::mac::file::close());
    if has_path {
        menu = menu.append(platform_menus::mac::file::save()).append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-save-as"),
                Command::new(
                    commands::SHOW_SAVE_PANEL,
                    FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE]),
                ),
            )
            .hotkey(SysMods::CmdShift, "s"),
        );
    } else {
        menu = menu.append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-save-as"),
                Command::new(
                    commands::SHOW_SAVE_PANEL,
                    FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE]),
                ),
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
            LocalizedString::new("menu-item-delete").with_placeholder("Delete".into()),
            consts::cmd::DELETE,
        ))
        .append_separator()
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-select-all").with_placeholder("Select All".into()),
                consts::cmd::SELECT_ALL,
            )
            .hotkey(SysMods::Cmd, "a"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-deselect-all")
                    .with_placeholder("Deselect All".into()),
                consts::cmd::DESELECT_ALL,
            )
            .hotkey(SysMods::CmdShift, "a"),
        )
}

fn view_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("menu-view-menu").with_placeholder("View".into()))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-increase-zoom").with_placeholder("Zoom In".into()),
                consts::cmd::ZOOM_IN,
            )
            .hotkey(SysMods::Cmd, "+"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-decrease-zoom").with_placeholder("Zoom Out".into()),
                consts::cmd::ZOOM_OUT,
            )
            .hotkey(SysMods::Cmd, "-"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-reset-zoom").with_placeholder("Reset Zoom".into()),
                consts::cmd::ZOOM_DEFAULT,
            )
            .hotkey(SysMods::Cmd, "0"),
        )
}

fn glyph_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("menu-glyph-menu").with_placeholder("Glyph".into())).append(
        MenuItem::new(
            LocalizedString::new("menu-item-add-component")
                .with_placeholder("Add Component".into()),
            consts::cmd::ADD_COMPONENT,
        )
        .hotkey(SysMods::CmdShift, "c")
        .disabled(),
    )
}

fn tools_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("menu-tools-menu").with_placeholder("Tools".into()))
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-select-tool").with_placeholder("Select".into()),
                consts::cmd::SELECT_TOOL,
            )
            .hotkey(SysMods::None, "v"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-pen-tool").with_placeholder("Pen".into()),
                consts::cmd::PEN_TOOL,
            )
            .hotkey(SysMods::None, "p"),
        )
        .append(
            MenuItem::new(
                LocalizedString::new("menu-item-preview-tool").with_placeholder("Preview".into()),
                consts::cmd::PREVIEW_TOOL,
            )
            .hotkey(SysMods::None, "h"),
        )
}
