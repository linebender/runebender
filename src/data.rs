//! Application state.

use std::path::PathBuf;
use std::rc::Rc;

use druid::Data;
use norad::Ufo;

#[derive(Clone, Default)]
pub struct AppState {
    pub file: Option<FontObject>,
}

#[derive(Clone)]
pub struct FontObject {
    pub path: Option<PathBuf>,
    pub object: Rc<Ufo>,
}

impl AppState {
    pub fn set_file(&mut self, object: Ufo, path: impl Into<Option<PathBuf>>) {
        let obj = FontObject {
            path: path.into(),
            object: Rc::new(object),
        };
        self.file = Some(obj);
    }
}

impl Data for FontObject {
    fn same(&self, other: &Self) -> bool {
        self.path == other.path && Rc::ptr_eq(&self.object, &other.object)
    }
}

impl Data for AppState {
    fn same(&self, other: &Self) -> bool {
        self.file.same(&other.file)
    }
}
