//! Application menus.

use druid::command::{sys as sys_cmd, Command};
use druid::menu::{self, MenuDesc, MenuItem};
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

    menu.append(
        MenuDesc::new(LocalizedString::new("common-menu-file-menu")).append(
            MenuItem::new(
                LocalizedString::new("common-menu-file-open"),
                Command::new(
                    sys_cmd::OPEN_FILE,
                    FileDialogOptions::new().allowed_types(vec![UFO_FILE_TYPE]),
                ),
            )
            .hotkey(SysMods::Cmd, "o"),
        ),
    )
}
