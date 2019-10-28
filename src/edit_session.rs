use std::collections::BTreeSet;
use std::sync::Arc;

use druid::kurbo::{Rect, Shape};
use druid::Data;
use norad::{Glyph, GlyphName, Ufo};

use crate::component::Component;
use crate::design_space::ViewPort;
use crate::guides::Guide;
use crate::path::{EntityId, Path};

type UndoStack = ();

#[derive(Debug, Clone)]
struct DataRect(Rect);

/// The editing state of a particular glyph.
#[derive(Debug, Clone, Data)]
pub struct EditSession {
    pub name: GlyphName,
    pub glyph: Arc<Glyph>,
    pub paths: Arc<Vec<Path>>,
    pub selection: Arc<BTreeSet<EntityId>>,
    pub components: Arc<Vec<Component>>,
    pub guides: Arc<Vec<Guide>>,
    pub undo_stack: UndoStack,
    pub viewport: ViewPort,
    work_bounds: DataRect,
}

impl EditSession {
    pub fn new(name: &GlyphName, ufo: &Ufo) -> Self {
        let name = name.to_owned();
        let glyph = ufo.get_glyph(&name).unwrap().to_owned();
        let paths: Vec<Path> = glyph
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

        let work_bounds = crate::data::get_bezier(&name, ufo, None)
            .map(|o| o.bounding_box())
            .unwrap_or_default();

        EditSession {
            name,
            glyph,
            paths: Arc::new(paths),
            selection: Arc::default(),
            components: Arc::new(components),
            guides: Arc::new(guides),
            undo_stack: (),
            viewport: ViewPort::default(),
            work_bounds: DataRect(work_bounds),
        }
    }

    /// Returns the current layout bounds of the 'work', that is, all the things
    /// that are 'part of the glyph'.
    pub fn work_bounds(&self) -> Rect {
        self.work_bounds.0
    }
}

impl Data for DataRect {
    fn same(&self, other: &DataRect) -> bool {
        self.0.x0.same(&other.0.x0)
            && self.0.x1.same(&other.0.x1)
            && self.0.y0.same(&other.0.y0)
            && self.0.y1.same(&other.0.y1)
    }
}
