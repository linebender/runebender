//! A glyph embedded in another glyph.

use druid::kurbo::Affine;
use druid::Data;
use norad::GlyphName;

use crate::path::EntityId;

#[derive(Debug, Clone)]
pub struct Component {
    pub base: GlyphName,
    pub transform: Affine,
    pub id: EntityId,
}

impl Data for Component {
    fn same(&self, other: &Component) -> bool {
        self.base.same(&other.base)
            && self.id.same(&other.id)
            && self.transform.as_coeffs() == other.transform.as_coeffs()
    }
}

impl Component {
    pub fn from_norad(src: &norad::glyph::Component) -> Self {
        let base = src.base.as_str().into();
        let transform = src.transform.clone().into();
        let id = EntityId::new_with_parent(0);
        Component {
            base,
            transform,
            id,
        }
    }
}
