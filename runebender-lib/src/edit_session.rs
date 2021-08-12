use std::collections::BTreeSet;
use std::sync::Arc;

use druid::kurbo::{BezPath, Point, Rect, Shape, Size, Vec2};
use druid::{Data, Lens};
use norad::glyph::Outline;
use norad::{Glyph, GlyphName};

use crate::component::Component;
use crate::data::Workspace;
use crate::design_space::{DPoint, DVec2, ViewPort};
use crate::guides::Guide;
use crate::path::{Path, Segment};
use crate::point::{EntityId, PathPoint};
use crate::quadrant::Quadrant;
use crate::selection::Selection;

/// Minimum distance in screen units that a click must occur to be considered
/// on a point?
//TODO: this doesn't feel very robust; items themselves should have hitzones?
pub const MIN_CLICK_DISTANCE: f64 = 10.0;
pub const SEGMENT_CLICK_DISTANCE: f64 = 6.0;
/// Amount of bias penalizing on-curve points; we want to break ties in favor
/// of off-curve.
pub const ON_CURVE_PENALTY: f64 = MIN_CLICK_DISTANCE / 2.0;

/// A unique identifier for a session. A session keeps the same identifier
/// even if the name of the glyph changes.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct SessionId(usize);

impl SessionId {
    pub(crate) fn next() -> SessionId {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        SessionId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// The editing state of a particular glyph.
#[derive(Debug, Clone, Data)]
pub struct EditSession {
    #[data(ignore)]
    pub id: SessionId,
    pub name: GlyphName,
    pub glyph: Arc<Glyph>,
    pub paths: Arc<Vec<Path>>,
    pub selection: Selection,
    pub components: Arc<Vec<Component>>,
    pub guides: Arc<Vec<Guide>>,
    pub viewport: ViewPort,
    work_bounds: Rect,
    quadrant: Quadrant,
}

/// A type that is only created by a lens, for our coordinate editing panel
#[derive(Debug, Clone, Copy, Data, Lens)]
pub struct CoordinateSelection {
    /// the number of selected points
    pub count: usize,
    /// the bounding box of the selection
    pub frame: Rect,
    pub quadrant: Quadrant,
}

impl EditSession {
    /// a lens to return info on the current selection
    #[allow(non_upper_case_globals)]
    pub const selected_coord: lenses::CoordSelection = lenses::CoordSelection;

    pub fn new(name: &GlyphName, glyphs: &Workspace) -> Self {
        let name = name.to_owned();
        let glyph = glyphs.font.ufo.get_glyph(&name).unwrap().to_owned();
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

        //FIXME: this is never updated, and shouldn't be relied on
        let work_bounds = glyphs
            .get_bezier(&name)
            .map(|b| b.bounding_box())
            .unwrap_or_default();

        EditSession {
            id: SessionId::next(),
            name,
            glyph,
            paths: Arc::new(paths),
            selection: Selection::new(),
            components: Arc::new(components),
            guides: Arc::new(guides),
            viewport: ViewPort::default(),
            quadrant: Quadrant::Center,
            work_bounds,
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

    pub fn rebuild_glyph(&mut self) {
        let new_glyph = self.to_norad_glyph();
        *Arc::make_mut(&mut self.glyph) = new_glyph;
    }

    /// called if metadata changes elsewhere, such as in the main view.
    pub fn update_glyph_metadata(&mut self, changed: &Arc<Glyph>) {
        let glyph = Arc::make_mut(&mut self.glyph);
        glyph.advance = changed.advance.clone();
    }

    pub fn rename(&mut self, name: GlyphName) {
        self.name = name.clone();
        let glyph = Arc::make_mut(&mut self.glyph);
        glyph.codepoints = crate::glyph_names::codepoints_for_glyph(&name);
        glyph.name = name;
    }

    /// Returns the current layout bounds of the 'work', that is, all the things
    /// that are 'part of the glyph'.
    pub fn work_bounds(&self) -> Rect {
        self.work_bounds
    }

    pub fn paths_mut(&mut self) -> &mut Vec<Path> {
        Arc::make_mut(&mut self.paths)
    }

    pub fn components_mut(&mut self) -> &mut Vec<Component> {
        Arc::make_mut(&mut self.components)
    }

    pub fn guides_mut(&mut self) -> &mut Vec<Guide> {
        Arc::make_mut(&mut self.guides)
    }

    pub fn iter_points(&self) -> impl Iterator<Item = &PathPoint> {
        self.paths.iter().flat_map(|p| p.points().iter())
    }

    pub(crate) fn paths_for_selection(&self) -> Vec<Path> {
        let mut result = Vec::new();
        for paths in self
            .paths
            .iter()
            .map(|p| p.paths_for_selection(&self.selection))
        {
            result.extend(paths);
        }

        result
    }

    // Replaced by hit test methods.
    /*
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
    */

    /// Find the best hit, considering all items.
    pub fn hit_test_all(&self, point: Point, max_dist: Option<f64>) -> Option<EntityId> {
        if let Some(hit) = self.hit_test_filtered(point, max_dist, |_| true) {
            return Some(hit);
        }
        let max_dist = max_dist.unwrap_or(MIN_CLICK_DISTANCE);
        let mut best = None;
        for g in &*self.guides {
            let dist = g.screen_dist(self.viewport, point);
            if dist < max_dist && best.map(|(d, _id)| dist < d).unwrap_or(true) {
                best = Some((dist, g.id))
            }
        }
        best.map(|(_dist, id)| id)
    }

    /// Hit test a point against points.
    ///
    /// This method finds the closest point, but applies a penalty to prioritize
    /// off-curve points.
    ///
    /// A more sophisticated approach would be to reward on-curve points that are
    /// not already selected, but penalize them if they are selected. That would
    /// require a way to plumb in selection info.
    pub fn hit_test_filtered(
        &self,
        point: Point,
        max_dist: Option<f64>,
        mut f: impl FnMut(&PathPoint) -> bool,
    ) -> Option<EntityId> {
        let max_dist = max_dist.unwrap_or(MIN_CLICK_DISTANCE);
        let mut best = None;
        for p in self.iter_points() {
            if f(p) {
                let dist = p.screen_dist(self.viewport, point);
                let score = dist
                    + if p.is_on_curve() {
                        ON_CURVE_PENALTY
                    } else {
                        0.0
                    };
                if dist < max_dist && best.map(|(s, _id)| score < s).unwrap_or(true) {
                    best = Some((score, p.id))
                }
            }
        }
        best.map(|(_score, id)| id)
    }

    /// Hit test a point against the path segments.
    pub fn hit_test_segments(&self, point: Point, max_dist: Option<f64>) -> Option<(Segment, f64)> {
        let max_dist = max_dist.unwrap_or(MIN_CLICK_DISTANCE);
        let dpt = self.viewport.from_screen(point);
        let mut best = None;
        for path in &*self.paths {
            for seg in path.iter_segments() {
                let (t, d2) = seg.nearest(dpt);
                if best.as_ref().map(|(_seg, _t, d)| d2 < *d).unwrap_or(true) {
                    best = Some((seg, t, d2));
                }
            }
        }
        if let Some((seg, t, d2)) = best {
            if d2 * self.viewport.zoom.powi(2) < max_dist.powi(2) {
                return Some((seg, t));
            }
        }
        None
    }

    /// Return the index of the path that is currently drawing. To be currently
    /// drawing, there must be a single currently selected point.
    fn active_path_idx(&self) -> Option<usize> {
        if self.selection.len() == 1 {
            let active = self.selection.iter().next().unwrap();
            self.paths.iter().position(|p| p.contains(active))
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
        self.path_for_point(id)
            .and_then(|path| path.path_point_for_id(id))
    }

    pub fn path_for_point(&self, point: EntityId) -> Option<&Path> {
        self.path_idx_for_point(point)
            .and_then(|idx| self.paths.get(idx))
    }

    pub fn path_for_point_mut(&mut self, point: EntityId) -> Option<&mut Path> {
        let idx = self.path_idx_for_point(point)?;
        self.paths_mut().get_mut(idx)
    }

    fn path_idx_for_point(&self, point: EntityId) -> Option<usize> {
        self.paths.iter().position(|p| p.contains(&point))
    }

    pub(crate) fn add_path(&mut self, path: Path) {
        let point = path.points()[0].id;
        self.paths_mut().push(path);
        self.selection.select_one(point);
    }

    pub fn paste_paths(&mut self, paths: Vec<Path>) {
        self.selection.clear();
        self.selection
            .extend(paths.iter().flat_map(|p| p.points().iter().map(|pt| pt.id)));
        self.paths_mut().extend(paths);
    }

    pub fn toggle_point_type(&mut self, id: EntityId) {
        if let Some(path) = self.path_for_point_mut(id) {
            path.toggle_point_type(id)
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
        let to_delete = self.selection.per_path_selection();
        self.selection.clear();
        // if only deleting points from a single path, we will select a point
        // in that path afterwards.
        let set_sel = to_delete.path_len() == 1;

        for path_points in to_delete.iter() {
            if let Some(path) = self.path_for_point_mut(path_points[0]) {
                if let Some(new_sel) = path.delete_points(path_points) {
                    if set_sel {
                        self.selection.select_one(new_sel);
                    }
                }
            } else if path_points[0].is_guide() {
                self.guides_mut().retain(|g| !path_points.contains(&g.id));
            }
        }
        self.paths_mut().retain(|p| !p.points().is_empty());
    }

    /// Select all points.
    //NOTE: should this select other things too? Which ones?
    pub fn select_all(&mut self) {
        self.selection.clear();
        self.selection = self.iter_points().map(|p| p.id).collect();
    }

    /// returns a rect representing the containing rect of the current selection
    ///
    /// Will return Rect::ZERO if nothing is selected.
    pub(crate) fn selection_dpoint_bbox(&self) -> Rect {
        let mut iter = self
            .selection
            .iter()
            .flat_map(|id| self.path_point_for_id(*id).map(|pt| pt.point.to_raw()));

        let first_point = iter.next().unwrap_or_default();
        let bbox = Rect::ZERO.with_origin(first_point);
        iter.fold(bbox, |bb, pt| bb.union_pt(pt))
    }

    /// If the current selection is a single point, select the next point
    /// on that path.
    pub fn select_next(&mut self) {
        if self.selection.len() != 1 {
            return;
        }
        let id = self.selection.iter().next().copied().unwrap();
        let id = self
            .path_for_point(id)
            .and_then(|path| path.next_point(id).map(|pp| pp.id))
            .unwrap_or(id);
        self.selection.select_one(id);
    }

    /// If the current selection is a single point, select the previous point
    /// on that path.
    pub fn select_prev(&mut self) {
        if self.selection.len() != 1 {
            return;
        }
        let id = self.selection.iter().next().copied().unwrap();
        let id = self
            .path_for_point(id)
            .and_then(|path| path.prev_point(id).map(|pp| pp.id))
            .unwrap_or(id);
        self.selection.select_one(id);
    }

    pub fn select_path(&mut self, id: EntityId, toggle: bool) -> bool {
        let path = match self.paths.iter().find(|path| path.id() == id) {
            Some(path) => path,
            None => return false,
        };
        for point in path.points() {
            if !self.selection.insert(point.id) && toggle {
                self.selection.remove(&point.id);
            }
        }
        true
    }

    pub(crate) fn nudge_selection(&mut self, nudge: DVec2) {
        if self.selection.is_empty() {
            return;
        }

        let to_nudge = self.selection.per_path_selection();
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

    pub(crate) fn nudge_everything(&mut self, nudge: DVec2) {
        for path in self.paths_mut() {
            path.nudge_all_points(nudge);
        }
        for component in self.components_mut() {
            component.nudge(nudge);
        }
    }

    pub(crate) fn adjust_sidebearing(&mut self, delta: f64, is_left: bool) {
        let glyph = Arc::make_mut(&mut self.glyph);
        if let Some(advance) = glyph.advance.as_mut() {
            // clamp the delta; we can't have an advance width < 0.
            let delta = if (advance.width + delta as f32) < 0.0 {
                -advance.width as f64
            } else {
                delta
            };
            advance.width += delta as f32;

            if is_left {
                self.nudge_everything(DVec2::from_raw((delta, 0.0)));
            }
        }
    }

    pub(crate) fn scale_selection(&mut self, scale: Vec2, anchor: DPoint) {
        assert!(scale.x.is_finite() && scale.y.is_finite());
        if !self.selection.is_empty() {
            let sel = self.selection.per_path_selection();
            for path_points in sel.iter() {
                if let Some(path) = self.path_for_point_mut(path_points[0]) {
                    path.scale_points(path_points, scale, anchor);
                }
            }
        }
    }

    pub(crate) fn align_selection(&mut self) {
        let bbox = self.selection_dpoint_bbox();
        // TODO: is_empty() would be cleaner but hasn't landed yet
        if bbox.area() == 0.0 {
            return;
        }
        let (val, set_x) = if bbox.width() < bbox.height() {
            (0.5 * (bbox.x0 + bbox.x1), true)
        } else {
            (0.5 * (bbox.y0 + bbox.y1), false)
        };
        let val = val.round();
        // make borrow checker happy; we could state-split the paths instead, but meh
        let ids: Vec<EntityId> = self.selection.iter().copied().collect();
        for id in ids {
            if let Some(path) = self.path_for_point_mut(id) {
                path.align_point(id, val, set_x);
            }
        }
    }

    pub(crate) fn reverse_contours(&mut self) {
        let mut path_ixs = BTreeSet::new();
        for entity in self.selection.iter() {
            if let Some(path_ix) = self.path_idx_for_point(*entity) {
                path_ixs.insert(path_ix);
            }
        }
        if path_ixs.is_empty() {
            path_ixs.extend(0..self.paths.len());
        }
        let paths = self.paths_mut();
        for ix in path_ixs {
            paths[ix].reverse_contour();
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
        self.selection.select_one(guide.id);
        self.guides_mut().push(guide);
    }

    /// Convert the current session back into a norad `Glyph`, for saving.
    pub fn to_norad_glyph(&self) -> Glyph {
        let mut glyph = Glyph::new_named("");
        glyph.name = self.name.clone();
        glyph.advance = self.glyph.advance.clone();
        glyph.codepoints = self.glyph.codepoints.clone();

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
        glyph
    }
}

impl CoordinateSelection {
    /// a lens to return the point representation of the current selected coord(s)
    #[allow(non_upper_case_globals)]
    pub const quadrant_coord: lenses::QuadrantCoord = lenses::QuadrantCoord;

    /// a lens to return the bbox of the current selection
    #[allow(non_upper_case_globals)]
    pub const quadrant_bbox: lenses::QuadrantBbox = lenses::QuadrantBbox;
}

pub mod lenses {
    use super::*;
    use druid::Lens;

    pub struct CoordSelection;

    impl Lens<EditSession, CoordinateSelection> for CoordSelection {
        fn with<V, F: FnOnce(&CoordinateSelection) -> V>(&self, data: &EditSession, f: F) -> V {
            let count = data.selection.len();
            let frame = data.selection_dpoint_bbox();
            let quadrant = data.quadrant;
            f(&CoordinateSelection {
                count,
                frame,
                quadrant,
            })
        }

        fn with_mut<V, F: FnOnce(&mut CoordinateSelection) -> V>(
            &self,
            data: &mut EditSession,
            f: F,
        ) -> V {
            let count = data.selection.len();
            let frame = data.selection_dpoint_bbox();
            let quadrant = data.quadrant;
            let mut sel = CoordinateSelection {
                count,
                frame,
                quadrant,
            };
            let r = f(&mut sel);
            data.quadrant = sel.quadrant;
            r
        }
    }

    pub struct QuadrantCoord;

    impl Lens<CoordinateSelection, Point> for QuadrantCoord {
        fn with<V, F: FnOnce(&Point) -> V>(&self, data: &CoordinateSelection, f: F) -> V {
            let point = data.quadrant.point_in_dspace_rect(data.frame);
            f(&point)
        }

        fn with_mut<V, F: FnOnce(&mut Point) -> V>(
            &self,
            data: &mut CoordinateSelection,
            f: F,
        ) -> V {
            let point = data.quadrant.point_in_dspace_rect(data.frame);
            let mut point2 = point;
            let r = f(&mut point2);

            if point != point2 {
                let delta = point2 - point;
                data.frame = data.frame.with_origin(data.frame.origin() + delta);
            }
            r
        }
    }

    pub struct QuadrantBbox;

    impl Lens<CoordinateSelection, Size> for QuadrantBbox {
        fn with<V, F: FnOnce(&Size) -> V>(&self, data: &CoordinateSelection, f: F) -> V {
            f(&data.frame.size())
        }

        fn with_mut<V, F: FnOnce(&mut Size) -> V>(
            &self,
            data: &mut CoordinateSelection,
            f: F,
        ) -> V {
            let size = data.frame.size();
            let mut size2 = size;
            let r = f(&mut size2);

            if size != size2 {
                data.frame = data.frame.with_size(size2);
            }
            r
        }
    }
}
