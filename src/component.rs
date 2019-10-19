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
