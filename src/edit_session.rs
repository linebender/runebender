use std::collections::BTreeSet;
use std::sync::Arc;

use druid::Data;
use norad::Ufo;

use crate::component::Component;
use crate::guides::Guide;
use crate::path::{EntityId, Path};

type UndoStack = ();

/// The editing state of a particular glyph.
#[derive(Debug, Clone, Data)]
pub struct EditSession {
    pub paths: Arc<Vec<Path>>,
    pub selection: Arc<BTreeSet<EntityId>>,
    pub components: Arc<Vec<Component>>,
    pub guides: Arc<Vec<Guide>>,
    pub undo_stack: UndoStack,
}

impl EditSession {
    pub fn new(name: &str, ufo: &Ufo) -> Result<Self, ()> {
        Err(())
    }
}

// What we're doing next:
//
// Figure out how to massage the state needed for an editing session into shape
// (lenses etc) and then construct the EditSession whenever we open a glyph.
