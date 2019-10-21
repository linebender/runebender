use std::collections::BTreeSet;
use std::sync::Arc;

use druid::Data;
use norad::{GlyphName, Ufo};

use crate::component::Component;
use crate::guides::Guide;
use crate::path::{EntityId, Path};

type UndoStack = ();

/// The editing state of a particular glyph.
#[derive(Debug, Clone, Data)]
pub struct EditSession {
    pub name: GlyphName,
    pub paths: Arc<Vec<Path>>,
    pub selection: Arc<BTreeSet<EntityId>>,
    pub components: Arc<Vec<Component>>,
    pub guides: Arc<Vec<Guide>>,
    pub undo_stack: UndoStack,
}

impl EditSession {
    pub fn new(name: &GlyphName, ufo: &Ufo) -> Self {
        let name = name.to_owned();
        let glyph = ufo.get_glyph(&name).unwrap();
        let paths = glyph
            .outline
            .as_ref()
            .map(|ol| ol.contours.iter().map(Path::from_norad).collect())
            .unwrap_or_default();
        let components = glyph
            .outline
            .as_ref()
            .map(|ol| ol.components.iter().map(Component::from_norad).collect())
            .unwrap_or_default();
        let guides = glyph
            .guidelines
            .as_ref()
            .map(|guides| guides.iter().map(Guide::from_norad).collect())
            .unwrap_or_default();

        EditSession {
            name,
            paths: Arc::new(paths),
            selection: Arc::default(),
            components: Arc::new(components),
            guides: Arc::new(guides),
            undo_stack: (),
        }
    }
}
