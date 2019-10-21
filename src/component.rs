//! A glyph embedded in another glyph.

use std::sync::Arc;

use druid::kurbo::Affine;
use druid::Data;
use norad::GlyphName;

use crate::path::EntityId;

#[derive(Debug, Clone, Data)]
pub struct Component {
    pub base: GlyphName,
    pub transform: Arc<Affine>,
    pub id: EntityId,
}

impl Component {
    pub fn from_norad(src: &norad::glyph::Component) -> Self {
        let base = src.base.as_str().into();
        let transform = Arc::new(src.transform.clone().into());
        let id = EntityId::new_with_parent(0);
        Component {
            base,
            transform,
            id,
        }
    }
}
