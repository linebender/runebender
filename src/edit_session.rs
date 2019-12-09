use std::collections::BTreeSet;
use std::sync::Arc;

use druid::kurbo::{BezPath, Point, Rect, Shape};
use druid::Data;
use norad::glyph::Outline;
use norad::{Glyph, GlyphName};

use crate::component::Component;
use crate::data::FontObject;
use crate::design_space::{DPoint, DVec2, ViewPort};
use crate::guides::Guide;
use crate::path::{EntityId, Path, PathPoint};

/// Minimum distance in screen units that a click must occur to be considered
/// on a point?
//TODO: this doesn't feel very robust; items themselves should have hitzones?
pub const MIN_CLICK_DISTANCE: f64 = 10.0;

/// The editing state of a particular glyph.
#[derive(Debug, Clone, Data)]
pub struct EditSession {
    pub name: GlyphName,
    pub glyph: Arc<Glyph>,
    pub paths: Arc<Vec<Path>>,
    pub selection: Arc<BTreeSet<EntityId>>,
    pub components: Arc<Vec<Component>>,
    pub guides: Arc<Vec<Guide>>,
    pub viewport: ViewPort,
    #[druid(same_fn = "rect_same")]
    work_bounds: Rect,
    /// A string describing the current tool
    pub tool_desc: Arc<str>,
}

impl EditSession {
    pub fn new(name: &GlyphName, font: &FontObject) -> Self {
        let name = name.to_owned();
        let glyph = font.ufo.get_glyph(&name).unwrap().to_owned();
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

        let work_bounds = font
            .get_bezier(&name)
            .map(|o| o.bounding_box())
            .unwrap_or_default();

        EditSession {
            name,
            glyph,
            paths: Arc::new(paths),
            selection: Arc::default(),
            components: Arc::new(components),
            guides: Arc::new(guides),
            viewport: ViewPort::default(),
            tool_desc: Arc::from("Select"),
            work_bounds: work_bounds,
        }
    }

    /// Construct a bezier of the paths in this glyph, ignoring components.
    pub fn to_bezier(&self) -> BezPath {
        let mut bez = BezPath::new();
        for path in self.paths.iter() {
            path.append_to_bezier(&mut bez);
        }
        bez
    }

    /// Returns the current layout bounds of the 'work', that is, all the things
    /// that are 'part of the glyph'.
    pub fn work_bounds(&self) -> Rect {
        self.work_bounds
    }

    pub fn selection_mut(&mut self) -> &mut BTreeSet<EntityId> {
        Arc::make_mut(&mut self.selection)
    }

    pub fn paths_mut(&mut self) -> &mut Vec<Path> {
        Arc::make_mut(&mut self.paths)
    }

    pub fn guides_mut(&mut self) -> &mut Vec<Guide> {
        Arc::make_mut(&mut self.guides)
    }

    pub fn iter_points(&self) -> impl Iterator<Item = &PathPoint> {
        self.paths.iter().flat_map(|p| p.points().iter())
    }

    /// For hit testing; iterates 'clickable items' (right now just points
    /// and guides) near a given point.
    pub fn iter_items_near_point<'a>(
        &'a self,
        point: Point,
        max_dist: Option<f64>,
    ) -> impl Iterator<Item = EntityId> + 'a {
        let max_dist = max_dist.unwrap_or(MIN_CLICK_DISTANCE);
        self.paths
            .iter()
            .flat_map(|p| p.points().iter())
            .filter(move |p| p.screen_dist(self.viewport, point) <= max_dist)
            .map(|p| p.id)
            .chain(
                self.guides
                    .iter()
                    .filter(move |g| g.screen_dist(self.viewport, point) <= max_dist)
                    .map(|g| g.id),
            )
    }

    /// Return the index of the path that is currently drawing. To be currently
    /// drawing, there must be a single currently selected point.
    fn active_path_idx(&self) -> Option<usize> {
        if self.selection.len() == 1 {
            let active = self.selection.iter().next().unwrap();
            self.paths.iter().position(|p| *p == *active)
        } else {
            None
        }
    }

    pub fn active_path_mut(&mut self) -> Option<&mut Path> {
        match self.active_path_idx() {
            Some(idx) => self.paths_mut().get_mut(idx),
            None => None,
        }
    }

    pub fn active_path(&self) -> Option<&Path> {
        match self.active_path_idx() {
            Some(idx) => self.paths.get(idx),
            None => None,
        }
    }

    pub fn path_point_for_id(&self, id: EntityId) -> Option<PathPoint> {
        self.paths
            .iter()
            .find(|p| **p == id)
            .and_then(|path| path.path_point_for_id(id))
    }

    pub fn path_for_point_mut(&mut self, point: EntityId) -> Option<&mut Path> {
        self.paths_mut().iter_mut().find(|p| **p == point)
    }

    fn new_path(&mut self, start: Point) {
        let start = self.viewport.from_screen(start);
        let path = Path::new(start);
        let point = path.points()[0].id;

        self.paths_mut().push(path);
        self.clear_selection();
        self.selection_mut().insert(point);
    }

    pub fn paste_paths(&mut self, paths: Vec<Path>) {
        self.clear_selection();
        self.selection_mut()
            .extend(paths.iter().flat_map(|p| p.points().iter().map(|pt| pt.id)));
        self.paths_mut().extend(paths);
    }

    pub fn add_point(&mut self, point: Point) {
        if self.active_path_idx().is_none() {
            self.new_path(point);
        } else {
            let point = self.viewport.from_screen(point);
            let new_point = self.active_path_mut().unwrap().append_point(point);
            self.selection_mut().clear();
            self.selection_mut().insert(new_point);
        }
    }

    pub fn update_for_drag(&mut self, drag_point: Point) {
        let drag_point = self.viewport.from_screen(drag_point);
        self.active_path_mut().unwrap().update_for_drag(drag_point);
    }

    /// If there is a single on curve point selected, toggle it between corner and smooth
    pub fn toggle_selected_on_curve_type(&mut self) {
        if self.selection.len() == 1 {
            let point = self.selection.iter().copied().next().unwrap();
            let path = self.active_path_mut().unwrap();
            path.toggle_on_curve_point_type(point);
        }
    }

    /// if a guide his horizontal or vertical, toggle between the two.
    pub fn toggle_guide(&mut self, id: EntityId, pos: Point) {
        let pos = self.viewport.from_screen(pos);
        if let Some(guide) = self.guides_mut().iter_mut().find(|g| g.id == id) {
            guide.toggle_vertical_horiz(pos);
        }
    }

    pub fn delete_selection(&mut self) {
        let to_delete = PathSelection::new(&self.selection);
        self.selection_mut().clear();
        for path_points in to_delete.iter() {
            if let Some(path) = self.path_for_point_mut(path_points[0]) {
                path.delete_points(path_points);
            } else if path_points[0].is_guide() {
                self.guides_mut().retain(|g| !path_points.contains(&g.id));
            }
        }
        self.paths_mut().retain(|p| !p.points().is_empty());
    }

    /// Select all points.
    //NOTE: should this select other things too? Which ones?
    pub fn select_all(&mut self) {
        *self.selection_mut() = self.iter_points().map(|p| p.id).collect();
    }

    pub fn clear_selection(&mut self) {
        self.selection_mut().clear()
    }

    /// If the current selection is a single point, select the next point
    /// on that path.
    pub fn select_next(&mut self) {
        if self.selection.len() != 1 {
            return;
        }
        let id = self.selection.iter().next().copied().unwrap();
        self.selection_mut().clear();
        let id = self
            .paths
            .iter()
            .find(|p| **p == id)
            .map(|path| path.next_point(id).id)
            .unwrap_or(id);
        self.selection_mut().insert(id);
    }

    /// If the current selection is a single point, select the previous point
    /// on that path.
    pub fn select_prev(&mut self) {
        if self.selection.len() != 1 {
            return;
        }
        let id = self.selection.iter().next().copied().unwrap();
        self.selection_mut().clear();
        let id = self
            .paths
            .iter()
            .find(|p| **p == id)
            .map(|path| path.prev_point(id).id)
            .unwrap_or(id);
        self.selection_mut().insert(id);
    }

    pub fn select_path(&mut self, point: Point, toggle: bool) -> bool {
        let path_idx = match self
            .paths
            .iter()
            .position(|p| p.screen_dist(self.viewport, point) < MIN_CLICK_DISTANCE)
        {
            Some(idx) => idx,
            None => return false,
        };

        let points: Vec<_> = self.paths[path_idx].points().to_owned();
        for point in points {
            if !self.selection_mut().insert(point.id) && toggle {
                self.selection_mut().remove(&point.id);
            }
        }
        true
    }

    pub(crate) fn nudge_selection(&mut self, nudge: DVec2) {
        if self.selection.is_empty() {
            return;
        }

        let to_nudge = PathSelection::new(&self.selection);
        for path_points in to_nudge.iter() {
            if let Some(path) = self.path_for_point_mut(path_points[0]) {
                path.nudge_points(path_points, nudge);
            } else if path_points[0].is_guide() {
                for id in path_points {
                    if let Some(guide) = self.guides_mut().iter_mut().find(|g| g.id == *id) {
                        guide.nudge(nudge);
                    }
                }
            }
        }
    }

    pub(crate) fn add_guide(&mut self, point: Point) {
        // if one or two points are selected, use them. else use argument point.
        let guide = match self.selection.len() {
            1 => {
                let id = *self.selection.iter().next().unwrap();
                if !id.is_guide() {
                    let p = self.path_point_for_id(id).map(|pp| pp.point).unwrap();
                    Some(Guide::horiz(p))
                } else {
                    None
                }
            }
            2 => {
                let mut iter = self.selection.iter().cloned();
                let id1 = iter.next().unwrap();
                let id2 = iter.next().unwrap();
                if !id1.is_guide() && !id2.is_guide() {
                    let p1 = self.path_point_for_id(id1).map(|pp| pp.point).unwrap();
                    let p2 = self.path_point_for_id(id2).map(|pp| pp.point).unwrap();
                    Some(Guide::angle(p1, p2))
                } else {
                    None
                }
            }
            _ => None,
        };

        let guide =
            guide.unwrap_or_else(|| Guide::horiz(DPoint::from_screen(point, self.viewport)));
        self.selection_mut().clear();
        self.selection_mut().insert(guide.id);
        self.guides_mut().push(guide);
    }

    /// Convert the current session back into a norad `Glyph`, for saving.
    pub fn to_norad_glyph(&self) -> Glyph {
        let mut glyph = Glyph::new_named("");
        glyph.name = self.name.clone();
        glyph.advance = self.glyph.advance.clone();

        let contours: Vec<_> = self.paths.iter().map(Path::to_norad).collect();
        let components: Vec<_> = self.components.iter().map(Component::to_norad).collect();
        if !contours.is_empty() || !components.is_empty() {
            glyph.outline = Some(Outline {
                components,
                contours,
            });
        }
        let guidelines: Vec<_> = self.guides.iter().map(Guide::to_norad).collect();
        if !guidelines.is_empty() {
            glyph.guidelines = Some(guidelines);
        }
        // codepoints
        glyph
    }
}

fn rect_same(one: &Rect, two: &Rect) -> bool {
    one.x0.same(&two.x0) && one.x1.same(&two.x1) && one.y0.same(&two.y0) && one.y1.same(&two.y1)
}

/// A helper for iterating through a selection in per-path chunks.
struct PathSelection {
    inner: Vec<EntityId>,
}

impl PathSelection {
    fn new(src: &BTreeSet<EntityId>) -> PathSelection {
        let mut inner: Vec<_> = src.iter().copied().collect();
        inner.sort();
        PathSelection { inner }
    }

    fn iter(&self) -> PathSelectionIter {
        PathSelectionIter {
            inner: &self.inner,
            idx: 0,
        }
    }
}

struct PathSelectionIter<'a> {
    inner: &'a [EntityId],
    idx: usize,
}

impl<'a> Iterator for PathSelectionIter<'a> {
    type Item = &'a [EntityId];
    fn next(&mut self) -> Option<&'a [EntityId]> {
        if self.idx >= self.inner.len() {
            return None;
        }
        let path_id = self.inner[self.idx].parent;
        let end_idx = self.inner[self.idx..]
            .iter()
            .position(|p| p.parent != path_id)
            .map(|idx| idx + self.idx)
            .unwrap_or(self.inner.len());
        let range = self.idx..end_idx;
        self.idx = end_idx;
        // probably unnecessary, but we don't expect empty slices
        if range.start == range.end {
            None
        } else {
            Some(&self.inner[range])
        }
    }
}
