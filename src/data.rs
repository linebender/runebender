//! Application state.

use std::path::PathBuf;
use std::rc::Rc;

use druid::Data;
use norad::Ufo;

//use lens2::Lens2;

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

pub mod lenses {
    pub mod app_state {
        use std::rc::Rc;

        use super::super::{AppState, FontObject};
        use crate::lens2::Lens2;

        pub struct Glyph {
            name: String,
        }

        impl Lens2<AppState, Rc<norad::Glyph>> for Glyph {
            fn get<V, F: FnOnce(&Rc<norad::Glyph>) -> V>(&self, data: &AppState, f: F) -> V {
                let glyph = data
                    .file
                    .as_ref()
                    .unwrap()
                    .object
                    .get_default_layer()
                    .unwrap()
                    .get_glyph(&self.name)
                    .unwrap();
                f(&glyph)
            }

            fn with_mut<V, F: FnOnce(&mut Rc<norad::Glyph>) -> V>(
                &self,
                data: &mut AppState,
                f: F,
            ) -> V {
                let mut glyph = data
                    .file
                    .as_mut()
                    .unwrap()
                    .object
                    .get_default_layer()
                    .unwrap()
                    .get_glyph(&self.name)
                    .unwrap();
                f(&mut glyph)
            }
        }
    }
}
