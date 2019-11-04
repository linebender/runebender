//! Application menus.

use crate::consts;
use druid::command::{sys as sys_cmd, Command};
use druid::menu::{self, sys as sys_menu, MenuDesc, MenuItem};
use druid::shell::dialog::{FileDialogOptions, FileSpec};
use druid::{Data, LocalizedString, SysMods};

const UFO_FILE_TYPE: FileSpec = FileSpec::new("Font Object", &["ufo"]);

/// The main window/app menu.
pub(crate) fn make_menu<T: Data>() -> MenuDesc<T> {
    let mut menu = MenuDesc::empty();
    #[cfg(target_os = "macos")]
    {
        menu = menu.append(menu::sys::mac::application::default());
    }

    menu.append(file_menu())
        .append(edit_menu())
        .append(glyph_menu())
        .append(tools_menu())
}

fn file_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("common-menu-file-menu"))
        .append(sys_menu::mac::file::new_file().disabled())
        .append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-open"),
                Command::new(
                    sys_cmd::OPEN_FILE,
                    FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE]),
                ),
            )
            .hotkey(SysMods::Cmd, "o"),
        )
        .append_separator()
        .append(sys_menu::mac::file::close())
        .append(sys_menu::mac::file::save().disabled())
        .append(sys_menu::mac::file::save_as().disabled())
        .append_separator()
        .append(sys_menu::mac::file::page_setup().disabled())
        .append(sys_menu::mac::file::print().disabled())
}

fn edit_menu<T: Data>() -> MenuDesc<T> {
    MenuDesc::new(LocalizedString::new("common-menu-edit-menu"))
        .append(sys_menu::common::undo().disabled())
        .append(sys_menu::common::redo().disabled())
        .append_separator()
        .append(sys_menu::common::cut().disabled())
        .append(sys_menu::common::copy().disabled())
        .append(sys_menu::common::paste().disabled())
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
            .hotkey(SysMods::None, "p")
            .disabled(),
        )
}
