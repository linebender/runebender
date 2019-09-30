//! Application state.

use std::path::PathBuf;
use std::rc::Rc;

use druid::Data;
use norad::{FontInfo, FormatVersion, MetaInfo, Ufo};

#[derive(Clone, Default)]
pub struct AppState {
    pub file: FontObject,
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
        self.file = obj;
    }
}

impl Data for FontObject {
    fn same(&self, other: &Self) -> bool {
        self.path == other.path && other.object.same(&self.object)
    }
}

impl Data for AppState {
    fn same(&self, other: &Self) -> bool {
        self.file.same(&other.file)
    }
}

impl std::default::Default for FontObject {
    fn default() -> FontObject {
        let meta = MetaInfo {
            creator: "Runebender".into(),
            format_version: FormatVersion::V3,
        };

        let font_info = FontInfo {
            family_name: Some(String::from("Untitled")),
            ..Default::default()
        };

        let mut ufo = Ufo::new(meta);
        ufo.font_info = Some(font_info);

        FontObject {
            path: None,
            object: Rc::new(ufo),
        }
    }
}

pub mod lenses {
    pub mod app_state {
        use std::rc::Rc;

        use super::super::AppState;
        use crate::lens2::Lens2;
        use norad::{Glyph as Glyph_, GlyphName, Ufo as Ufo_};

        pub struct Ufo;

        pub struct Glyph(pub GlyphName);

        impl Lens2<AppState, Rc<Ufo_>> for Ufo {
            fn get<V, F: FnOnce(&Rc<Ufo_>) -> V>(&self, data: &AppState, f: F) -> V {
                f(&data.file.object)
            }
            fn with_mut<V, F: FnOnce(&mut Rc<Ufo_>) -> V>(&self, data: &mut AppState, f: F) -> V {
                f(&mut data.file.object)
            }
        }

        impl Lens2<Rc<Ufo_>, Rc<Glyph_>> for Glyph {
            fn get<V, F: FnOnce(&Rc<Glyph_>) -> V>(&self, data: &Rc<Ufo_>, f: F) -> V {
                let glyph = data.get_glyph(&self.0).expect("missing glyph in lens2");
                f(glyph)
            }

            fn with_mut<V, F: FnOnce(&mut Rc<Glyph_>) -> V>(&self, data: &mut Rc<Ufo_>, f: F) -> V {
                //FIXME: this is creating a new copy and then throwing it away
                //this is just so that the signatures work for now, we aren't actually doing any
                //mutating
                let mut glyph = data
                    .get_glyph(&self.0)
                    .map(Rc::clone)
                    .expect("missing glyph in lens2");
                f(&mut glyph)
            }
        }
    }
}
