//! Application state.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use druid::kurbo::{BezPath, Point, Rect, Shape, Size};
use druid::{Data, Lens, WindowId};
use norad::glyph::{Contour, ContourPoint, Glyph, GlyphName, PointType};
use norad::{FontInfo, Ufo};

use crate::bez_cache::BezCache;
use crate::edit_session::{EditSession, SessionId};

/// This is by convention.
const DEFAULT_UNITS_PER_EM: f64 = 1000.;

/// The top level data structure.
///
/// Currently this just wraps `Workspace`; in the future multiple workspaces
/// will be possible.
#[derive(Clone, Data, Default, Lens)]
pub struct AppState {
    pub workspace: Workspace,
}

/// A workspace is a single font, corresponding to a UFO file on disk.
#[derive(Clone, Lens, Data, Default)]
pub struct Workspace {
    pub font: Arc<FontObject>,
    /// The currently selected glyph (in the main glyph list) if any.
    //TODO: allow multiple selections
    pub selected: Option<GlyphName>,
    /// glyphs that are already open in an editor window
    pub open_glyphs: Arc<HashMap<GlyphName, WindowId>>,
    pub sessions: Arc<HashMap<SessionId, Arc<EditSession>>>,
    session_map: Arc<HashMap<GlyphName, SessionId>>,
    // really just a store of the fully resolved Beziers of all glyphs.
    cache: Arc<BezCache>,
    pub info: SimpleFontInfo,
}

#[derive(Clone, Data)]
pub struct FontObject {
    pub path: Option<Arc<Path>>,
    #[data(ignore)]
    pub ufo: Ufo,
    placeholder: Arc<BezPath>,
}

/// The data type for a grid square.
///
/// Unlike GlyphDetail, this doesn't have a reference to the glyph itself,
/// which is expensive to find in large glyphsets.
#[derive(Debug, Clone, Data, Lens)]
pub(crate) struct GridGlyph {
    pub name: GlyphName,
    pub outline: Arc<BezPath>,
    pub is_placeholder: bool,
    pub is_selected: bool,
    pub upm: f64,
}

/// Detailed information about a specific glyph.
///
/// This is used in the sidepanel, as well as in the editor window.
#[derive(Clone, Data, Lens)]
pub struct GlyphDetail {
    pub glyph: Arc<Glyph>,
    // the full outline, including things like components
    pub outline: Arc<BezPath>,
    metrics: FontMetrics,
    is_placeholder: bool,
}

#[derive(Clone, Data, Lens)]
pub struct SimpleFontInfo {
    metrics: FontMetrics,
    pub family_name: Arc<str>,
    pub style_name: Arc<str>,
}

/// Things in `FontInfo` that are relevant while editing or drawing.
#[derive(Clone, Data, Lens)]
pub struct FontMetrics {
    pub units_per_em: f64,
    pub descender: Option<f64>,
    pub x_height: Option<f64>,
    pub cap_height: Option<f64>,
    pub ascender: Option<f64>,
    pub italic_angle: Option<f64>,
}

/// The state for an editor view.
#[derive(Clone, Data, Lens)]
pub struct EditorState {
    pub metrics: FontMetrics,
    pub font: Workspace,
    pub session: Arc<EditSession>,
}

/// A type constructed by a lens to represent our sidebearings.
#[derive(Debug, Clone, Data, Lens)]
pub struct Sidebearings {
    pub left: f64,
    pub right: f64,
}

impl Workspace {
    /// a lens into a particular editor view.
    pub(crate) fn editor_state(id: SessionId) -> impl Lens<Workspace, EditorState> {
        lenses::EditorState(id)
    }

    /// a lens for getting a `GridGlyph`.
    pub(crate) fn glyph_grid(name: GlyphName) -> impl Lens<Workspace, Option<GridGlyph>> {
        lenses::GridGlyph(name)
    }

    /// A lens or the currently selected glyph
    #[allow(non_upper_case_globals)]
    pub(crate) const selected_glyph: lenses::SelectedGlyph = lenses::SelectedGlyph;

    pub fn set_file(&mut self, ufo: Ufo, path: impl Into<Option<PathBuf>>) {
        let obj = FontObject {
            path: path.into().map(Into::into),
            ufo,
            placeholder: Arc::new(placeholder_outline()),
        };
        self.font = obj.into();
        self.info = SimpleFontInfo::from_font(&self.font);
        self.build_path_cache();
    }

    fn build_path_cache(&mut self) {
        let Workspace {
            font,
            cache,
            sessions,
            ..
        } = self;

        Arc::make_mut(cache).reset(&font.ufo, &|name| {
            sessions
                .values()
                .find(|sesh| sesh.name == *name)
                .map(|sesh| &sesh.glyph)
                .or_else(|| font.ufo.get_glyph(name))
        });
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let font_obj = Arc::make_mut(&mut self.font);
        font_obj.update_info(&self.info);
        if let Some(path) = font_obj.path.as_ref() {
            backup_ufo_at_path(path)?;
            log::info!("saving to {:?}", path);
            // flush all open sessions
            for session in self.sessions.values() {
                let glyph = session.to_norad_glyph();
                font_obj
                    .ufo
                    .get_default_layer_mut()
                    .unwrap()
                    .insert_glyph(glyph);
            }
            font_obj.ufo.save(&path)?;
        } else {
            log::error!("save called with no path set");
        }
        Ok(())
    }

    pub fn get_or_create_session(&mut self, glyph_name: &GlyphName) -> Arc<EditSession> {
        self.session_map
            .get(glyph_name)
            .and_then(|id| self.sessions.get(id))
            .cloned()
            .unwrap_or_else(|| {
                let session = Arc::new(EditSession::new(glyph_name, self));
                let session_id = session.id;
                Arc::make_mut(&mut self.sessions).insert(session_id, session.clone());
                Arc::make_mut(&mut self.session_map).insert(glyph_name.clone(), session_id);
                session
            })
    }

    pub(crate) fn get_bezier(&self, name: &GlyphName) -> Option<Arc<BezPath>> {
        self.cache.get(name)
    }

    /// After a glyph is edited this rebuilds the affected beziers.
    pub(crate) fn invalidate_path(&mut self, name: &GlyphName) {
        let Workspace {
            font,
            cache,
            sessions,
            ..
        } = self;
        let to_inval = cache.glyphs_containing_component(name).to_vec();
        for name in std::iter::once(name).chain(to_inval.iter()) {
            Arc::make_mut(cache).rebuild(&name, &|name| {
                sessions
                    .values()
                    .find(|sesh| sesh.name == *name)
                    .map(|sesh| &sesh.glyph)
                    .or_else(|| font.ufo.get_glyph(name))
            });
        }
    }

    /// Returns the upm for this font.
    ///
    /// This is needed to correctly scale the points in the glyph.
    pub fn units_per_em(&self) -> f64 {
        self.font
            .ufo
            .font_info
            .as_ref()
            .and_then(|info| info.units_per_em.map(|v| v.get()))
            .unwrap_or(DEFAULT_UNITS_PER_EM)
    }

    pub fn add_new_glyph(&mut self) -> GlyphName {
        let mut name = String::from("newGlyph");
        let mut counter = 0;

        while self.font.ufo.get_glyph(name.as_str()).is_some() {
            counter += 1;
            name = format!("newGlyph.{}", counter);
        }

        let name: GlyphName = name.into();
        let glyph = norad::Glyph::new_named(name.clone());
        self.font_mut()
            .ufo
            .get_default_layer_mut()
            .unwrap()
            .insert_glyph(glyph);
        name
    }

    pub fn delete_selected_glyph(&mut self) -> Option<Arc<Glyph>> {
        self.selected.take().and_then(|name| {
            self.font_mut()
                .ufo
                .get_default_layer_mut()
                .unwrap()
                .remove_glyph(&name)
        })
    }

    /// Rename a glyph everywhere it might be.
    pub fn rename_glyph(&mut self, old_name: GlyphName, new_name: GlyphName) {
        let font = self.font_mut();
        let mut glyph = match font
            .ufo
            .get_default_layer_mut()
            .unwrap()
            .remove_glyph(&old_name)
        {
            Some(g) => g,
            None => {
                log::warn!("attempted to rename missing glyph '{}'", old_name);
                return;
            }
        };

        {
            let glyph = Arc::make_mut(&mut glyph);
            glyph.codepoints = crate::glyph_names::codepoints_for_glyph(&new_name);
            glyph.name = new_name.clone();
        }

        font.ufo
            .get_default_layer_mut()
            .unwrap()
            .insert_glyph(glyph);

        // and if this is the selected glyph, change that too;
        if self.selected.as_ref() == Some(&old_name) {
            self.selected = Some(new_name.clone())
        }

        // if this glyph is open, rename that too;
        if self.session_map.contains_key(&old_name) {
            let session_map = Arc::make_mut(&mut self.session_map);
            let session_id = session_map.remove(&old_name).unwrap();
            session_map.insert(new_name.clone(), session_id);

            let sessions = Arc::make_mut(&mut self.sessions);
            let mut session = sessions.get_mut(&session_id).unwrap();
            Arc::make_mut(&mut session).rename(new_name.clone());
        }

        if self.open_glyphs.contains_key(&old_name) {
            let open = Arc::make_mut(&mut self.open_glyphs);
            let window = open.remove(&old_name).unwrap();
            open.insert(new_name, window);
        }
    }

    pub fn update_glyph_metadata(&mut self, changed: &Arc<Glyph>) {
        // update the active session, if one exists
        if let Some(session_id) = self.session_map.get(&changed.name) {
            let sessions = Arc::make_mut(&mut self.sessions);
            let session = sessions.get_mut(&session_id).unwrap();
            let session = Arc::make_mut(session);
            session.update_glyph_metadata(changed);
        }
        // update the UFO;
        if let Some(glyph) = self.font_mut().ufo.get_glyph_mut(&changed.name) {
            Arc::make_mut(glyph).advance = changed.advance.clone();
        }
    }

    pub fn font_mut(&mut self) -> &mut FontObject {
        Arc::make_mut(&mut self.font)
    }
}

#[allow(non_upper_case_globals)]
impl GlyphDetail {
    /// A lens for retrieving the glyph's codepoint
    pub const codepoint: lenses::Codepoint = lenses::Codepoint;

    /// A lens for the glyph's advance.
    pub const advance: lenses::Advance = lenses::Advance;

    /// A lens for the glyph's name.
    pub const glyph_name: lenses::GlyphName = lenses::GlyphName;

    /// Get the fully resolved (including components) bezier path for this glyph.
    ///
    /// Returns the placeholder glyph if this glyph has no outline.
    pub fn get_bezier(&self) -> Arc<BezPath> {
        self.outline.clone()
    }

    /// Returns `true` if this glyph uses a placeholder path.
    pub fn is_placeholder_glyph(&self) -> bool {
        self.is_placeholder
    }

    /// Returns the first `char` in this glyph's codepoint list.
    pub fn get_codepoint(&self) -> Option<char> {
        self.glyph
            .codepoints
            .as_ref()
            .and_then(|v| v.first())
            .cloned()
    }

    /// The bounds of the metric square, in design space. (0, 0) is at the
    /// left edge of the baseline, and y is up.
    pub(crate) fn layout_bounds(&self) -> Rect {
        layout_bounds(&self.glyph, &self.metrics)
    }

    /// The upm for the font this glyph belongs to.
    pub fn upm(&self) -> f64 {
        self.metrics.units_per_em
    }
}

impl EditorState {
    /// a lens to return info on the current selection
    #[allow(non_upper_case_globals)]
    pub const sidebearings: lenses::Sidebearings = lenses::Sidebearings;

    /// The bounds of the metric square, in design space. (0, 0) is at the
    /// left edge of the baseline, and y is up.
    pub(crate) fn layout_bounds(&self) -> Rect {
        layout_bounds(&self.session.glyph, &self.metrics)
    }

    /// Returns a `Rect` representing, in the coordinate space of the canvas,
    /// the total region occupied by outlines, components, anchors, and the metric
    /// bounds.
    pub fn content_region(&self) -> Rect {
        let result = self.layout_bounds().union(self.session.work_bounds());
        Rect::from_points((result.x0, -result.y0), (result.x1, -result.y1))
    }

    pub fn session_mut(&mut self) -> &mut EditSession {
        Arc::make_mut(&mut self.session)
    }

    fn compute_sidebearings(&self) -> Sidebearings {
        let content_region = self
            .font
            .get_bezier(&self.session.name)
            .map(|p| p.bounding_box())
            .unwrap_or_default();
        let layout_bounds = self.layout_bounds();
        // the content region if it contains components and a scale transform
        // can need rounding.
        let left = content_region.min_x().round();
        let right = layout_bounds.width() - content_region.max_x().round();

        Sidebearings { left, right }
    }
}

impl FontObject {
    /// Update the actual `FontInfo` from the generated `SimpleFontInfo`
    #[allow(clippy::float_cmp)]
    fn update_info(&mut self, info: &SimpleFontInfo) {
        // we don't want to change anything if we don't have to:
        let existing_info = SimpleFontInfo::from_font(self);
        if !existing_info.same(&info) {
            let font_info = self.ufo.font_info.get_or_insert_with(Default::default);
            // we don't want to set anything that hasn't changed.
            if existing_info.family_name != info.family_name {
                font_info.family_name = Some(info.family_name.to_string());
            }
            if existing_info.style_name != info.style_name {
                font_info.style_name = Some(info.style_name.to_string());
            }
            if existing_info.metrics.units_per_em != info.metrics.units_per_em {
                font_info.units_per_em = info.metrics.units_per_em.try_into().ok();
            }
            if existing_info.metrics.descender != info.metrics.descender {
                font_info.descender = info.metrics.descender.map(Into::into);
            }
            if existing_info.metrics.ascender != info.metrics.ascender {
                font_info.ascender = info.metrics.ascender.map(Into::into);
            }
            if existing_info.metrics.x_height != info.metrics.x_height {
                font_info.x_height = info.metrics.x_height.map(Into::into);
            }
            if existing_info.metrics.cap_height != info.metrics.cap_height {
                font_info.cap_height = info.metrics.cap_height.map(Into::into);
            }
            if existing_info.metrics.italic_angle != info.metrics.italic_angle {
                font_info.italic_angle = info.metrics.italic_angle.map(Into::into);
            }
        }
    }
}

use std::convert::TryInto;

impl Default for FontObject {
    fn default() -> FontObject {
        let font_info = FontInfo {
            family_name: Some(String::from("Untitled")),
            ..Default::default()
        };

        let mut ufo = Ufo::new();
        ufo.font_info = Some(font_info);

        FontObject {
            path: None,
            ufo,
            placeholder: Arc::new(placeholder_outline()),
        }
    }
}

impl SimpleFontInfo {
    fn from_font(font: &FontObject) -> Self {
        SimpleFontInfo {
            family_name: font
                .ufo
                .font_info
                .as_ref()
                .and_then(|f| f.family_name.as_ref().map(|s| s.as_str().into()))
                .unwrap_or_else(|| "Untitled".into()),
            style_name: font
                .ufo
                .font_info
                .as_ref()
                .and_then(|f| f.style_name.as_ref().map(|s| s.as_str().into()))
                .unwrap_or_else(|| "Regular".into()),
            metrics: font
                .ufo
                .font_info
                .as_ref()
                .map(FontMetrics::from)
                .unwrap_or_default(),
        }
    }
}

impl Default for SimpleFontInfo {
    fn default() -> Self {
        SimpleFontInfo {
            metrics: Default::default(),
            family_name: "".into(),
            style_name: "".into(),
        }
    }
}

impl<'a> From<&'a FontInfo> for FontMetrics {
    fn from(src: &'a FontInfo) -> FontMetrics {
        FontMetrics {
            units_per_em: src
                .units_per_em
                .map(|v| v.get())
                .unwrap_or(DEFAULT_UNITS_PER_EM),
            descender: src.descender.map(|v| v.get()),
            x_height: src.x_height.map(|v| v.get()),
            cap_height: src.cap_height.map(|v| v.get()),
            ascender: src.ascender.map(|v| v.get()),
            italic_angle: src.italic_angle.map(|v| v.get()),
        }
    }
}

impl Default for FontMetrics {
    fn default() -> Self {
        FontMetrics {
            units_per_em: DEFAULT_UNITS_PER_EM,
            descender: None,
            x_height: None,
            cap_height: None,
            ascender: None,
            italic_angle: None,
        }
    }
}

mod lenses {
    use std::sync::Arc;

    use druid::{Data, Lens};
    use norad::GlyphName as GlyphName_;

    use super::{
        EditorState as EditorState_, GlyphDetail, GridGlyph as GridGlyph_, SessionId,
        Sidebearings as Sidebearings_, Workspace,
    };

    /// Workspace -> EditorState
    pub struct EditorState(pub SessionId);

    /// Workspace -> GridGlyph
    pub struct GridGlyph(pub GlyphName_);

    /// Workspace -> GlyphPlus
    pub struct SelectedGlyph;

    /// GlyphPlus => GlyphName_
    pub struct GlyphName;

    /// GlyphPlus -> char
    pub struct Codepoint;

    pub struct Advance;

    pub struct Sidebearings;

    impl Lens<Workspace, EditorState_> for EditorState {
        fn with<V, F: FnOnce(&EditorState_) -> V>(&self, data: &Workspace, f: F) -> V {
            let metrics = data.info.metrics.clone();
            let session = data.sessions.get(&self.0).cloned().unwrap();
            let glyph = EditorState_ {
                font: data.clone(),
                metrics,
                session,
            };
            f(&glyph)
        }
        fn with_mut<V, F: FnOnce(&mut EditorState_) -> V>(&self, data: &mut Workspace, f: F) -> V {
            //FIXME: this is creating a new copy and then throwing it away
            //this is just so that the signatures work for now, we aren't actually doing any
            let metrics = data.info.metrics.clone();
            let session = data.sessions.get(&self.0).unwrap().to_owned();
            let mut glyph = EditorState_ {
                font: data.clone(),
                metrics,
                session,
            };
            let v = f(&mut glyph);
            if !data
                .sessions
                .get(&self.0)
                .map(|s| s.same(&glyph.session))
                .unwrap_or(true)
            {
                let name = glyph.session.name.clone();
                Arc::make_mut(&mut data.sessions).insert(self.0, glyph.session);
                data.invalidate_path(&name);
            }
            v
        }
    }

    impl Lens<EditorState_, Sidebearings_> for Sidebearings {
        fn with<V, F: FnOnce(&Sidebearings_) -> V>(&self, data: &EditorState_, f: F) -> V {
            let sidebearings = data.compute_sidebearings();
            f(&sidebearings)
        }

        fn with_mut<V, F: FnOnce(&mut Sidebearings_) -> V>(
            &self,
            data: &mut EditorState_,
            f: F,
        ) -> V {
            let mut sidebearings = data.compute_sidebearings();
            f(&mut sidebearings)
        }
    }

    impl Lens<Workspace, Option<GridGlyph_>> for GridGlyph {
        fn with<V, F: FnOnce(&Option<GridGlyph_>) -> V>(&self, data: &Workspace, f: F) -> V {
            let outline = data.get_bezier(&self.0);

            let is_selected = data.selected.as_ref() == Some(&self.0);
            let glyph = Some(GridGlyph_ {
                name: self.0.clone(),
                is_placeholder: outline.is_none(),
                outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                upm: data.units_per_em(),
                is_selected,
            });
            f(&glyph)
        }

        fn with_mut<V, F: FnOnce(&mut Option<GridGlyph_>) -> V>(
            &self,
            data: &mut Workspace,
            f: F,
        ) -> V {
            let outline = data.get_bezier(&self.0);
            let is_selected = data.selected.as_ref() == Some(&self.0);
            let mut glyph = Some(GridGlyph_ {
                name: self.0.clone(),
                is_placeholder: outline.is_none(),
                outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                upm: data.units_per_em(),
                is_selected,
            });
            let r = f(&mut glyph);
            // we track selections by having the grid item set this flag,
            // and then we propogate that up to the workspace here.
            if glyph.as_ref().map(|g| g.is_selected).unwrap_or(false) {
                data.selected = Some(self.0.clone());
            }
            r
        }
    }

    impl Lens<GlyphDetail, Option<char>> for Codepoint {
        fn with<V, F: FnOnce(&Option<char>) -> V>(&self, data: &GlyphDetail, f: F) -> V {
            let c = data.get_codepoint();
            f(&c)
        }

        fn with_mut<V, F: FnOnce(&mut Option<char>) -> V>(
            &self,
            data: &mut GlyphDetail,
            f: F,
        ) -> V {
            let mut c = data.get_codepoint();
            let r = f(&mut c);
            let old = data.get_codepoint();
            if c != old {
                let glyph = Arc::make_mut(&mut data.glyph);
                match c {
                    Some(c) => glyph.codepoints = Some(vec![c]),
                    None => glyph.codepoints = None,
                }
            }
            r
        }
    }

    impl Lens<Workspace, Option<GlyphDetail>> for SelectedGlyph {
        fn with<V, F: FnOnce(&Option<GlyphDetail>) -> V>(&self, data: &Workspace, f: F) -> V {
            let selected = data.selected.as_ref().map(|name| {
                let glyph = data
                    .font
                    .ufo
                    .get_glyph(name)
                    .expect("missing glyph in lens");
                let outline = data.get_bezier(&glyph.name);
                let is_placeholder = outline.is_none();
                let metrics = data.info.metrics.clone();
                GlyphDetail {
                    glyph: Arc::clone(glyph),
                    outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                    metrics,
                    is_placeholder,
                }
            });
            f(&selected)
        }

        fn with_mut<V, F: FnOnce(&mut Option<GlyphDetail>) -> V>(
            &self,
            data: &mut Workspace,
            f: F,
        ) -> V {
            let mut selected = data.selected.as_ref().map(|name| {
                let glyph = data
                    .font
                    .ufo
                    .get_glyph(name)
                    .expect("missing glyph in lens");
                let outline = data.get_bezier(&glyph.name);
                let is_placeholder = outline.is_none();
                let metrics = data.info.metrics.clone();
                GlyphDetail {
                    glyph: Arc::clone(glyph),
                    outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                    metrics,
                    is_placeholder,
                }
            });
            let r = f(&mut selected);
            if let Some(selected) = selected {
                let is_same = data
                    .font
                    .ufo
                    .get_glyph(&selected.glyph.name)
                    .map(|g| g.same(&selected.glyph))
                    .unwrap_or(true);
                if !is_same {
                    data.update_glyph_metadata(&selected.glyph);
                    data.selected = Some(selected.glyph.name.clone());
                }
            }
            r
        }
    }

    impl Lens<GlyphDetail, f32> for Advance {
        fn with<V, F: FnOnce(&f32) -> V>(&self, data: &GlyphDetail, f: F) -> V {
            let advance = data.glyph.advance.as_ref().map(|a| a.width).unwrap_or(0.);
            f(&advance)
        }

        #[allow(clippy::float_cmp)]
        fn with_mut<V, F: FnOnce(&mut f32) -> V>(&self, data: &mut GlyphDetail, f: F) -> V {
            let advance = data.glyph.advance.as_ref().map(|a| a.width).unwrap_or(0.);
            let mut advance2 = advance;
            let result = f(&mut advance2);
            if advance2 != advance {
                let glyph = Arc::make_mut(&mut data.glyph);
                if advance2 == 0. {
                    glyph.advance = None;
                } else {
                    let mut advance = glyph.advance.clone().unwrap_or_default();
                    advance.width = advance2;
                    glyph.advance = Some(advance);
                }
            }
            result
        }
    }

    impl Lens<GlyphDetail, GlyphName_> for GlyphName {
        fn with<V, F: FnOnce(&GlyphName_) -> V>(&self, data: &GlyphDetail, f: F) -> V {
            f(&data.glyph.name)
        }

        fn with_mut<V, F: FnOnce(&mut GlyphName_) -> V>(&self, data: &mut GlyphDetail, f: F) -> V {
            // THIS DOESN'T DO ANYTHING! all the mutation happens
            // as a result of the RENAME_GLYPH command.
            let mut s = data.glyph.name.clone();
            f(&mut s)
        }
    }
}

//FIXME: put this in some `GlyphExt` trait or something
/// Convert this glyph's path from the UFO representation into a `kurbo::BezPath`
/// (which we know how to draw.)
pub(crate) fn path_for_glyph(glyph: &Glyph) -> Option<BezPath> {
    /// An outline can have multiple contours, which correspond to subpaths
    fn add_contour(path: &mut BezPath, contour: &Contour) {
        let mut close: Option<&ContourPoint> = None;

        if contour.points.is_empty() {
            return;
        }

        let first = &contour.points[0];
        path.move_to((first.x as f64, first.y as f64));
        if first.typ != PointType::Move {
            close = Some(first);
        }

        let mut idx = 1;
        let mut controls = Vec::with_capacity(2);

        let mut add_curve = |to_point: Point, controls: &mut Vec<Point>| {
            match controls.as_slice() {
                &[] => path.line_to(to_point),
                &[a] => path.quad_to(a, to_point),
                &[a, b] => path.curve_to(a, b, to_point),
                _illegal => panic!("existence of second point implies first"),
            };
            controls.clear();
        };

        while idx < contour.points.len() {
            let next = &contour.points[idx];
            let point = Point::new(next.x as f64, next.y as f64);
            match next.typ {
                PointType::OffCurve => controls.push(point),
                PointType::Line => {
                    debug_assert!(controls.is_empty(), "line type cannot follow offcurve");
                    add_curve(point, &mut controls);
                }
                PointType::Curve => add_curve(point, &mut controls),
                PointType::QCurve => {
                    log::warn!("quadratic curves are currently ignored");
                    add_curve(point, &mut controls);
                }
                PointType::Move => debug_assert!(false, "illegal move point in path?"),
            }
            idx += 1;
        }

        if let Some(to_close) = close.take() {
            add_curve((to_close.x as f64, to_close.y as f64).into(), &mut controls);
        }
    }

    if let Some(outline) = glyph.outline.as_ref() {
        let mut path = BezPath::new();
        outline
            .contours
            .iter()
            .for_each(|c| add_contour(&mut path, c));
        Some(path)
    } else {
        None
    }
}

/// Returns a rect representing the metric bounds of this glyph; that is,
/// taking into account the font metrics (ascender, descender) as well as the
/// glyph's width.
///
/// This rect is in the same coordinate space as the glyph: y is up, and
/// (0, 0)  is at the intersection of the baseline and the left sidebearing.
fn layout_bounds(glyph: &Glyph, metrics: &FontMetrics) -> Rect {
    let upm = metrics.units_per_em;
    let ascender = metrics.ascender.unwrap_or(upm * 0.8);
    let descender = metrics.descender.unwrap_or(upm * -0.2);
    let width = glyph.advance.as_ref().map(|a| a.width as f64);
    let width = width.unwrap_or(upm / 2.0);

    let work_size = Size::new(width, ascender + descender.abs());
    let work_origin = Point::new(0., descender);
    Rect::from_origin_size(work_origin, work_size)
}

/// a poorly drawn question mark glyph, used as a placeholder
fn placeholder_outline() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((51.0, 482.0));
    bez.line_to((112.0, 482.0));
    bez.curve_to((112.0, 482.0), (134.0, 631.0), (246.0, 631.0));
    bez.curve_to((308.0, 631.0), (356.0, 625.0), (356.0, 551.0));
    bez.curve_to((356.0, 487.0), (202.0, 432.0), (202.0, 241.0));
    bez.line_to((275.0, 241.0));
    bez.curve_to((276.0, 417.0), (430.0, 385.0), (430.0, 562.0));
    bez.curve_to((430.0, 699.0), (301.0, 700.0), (246.0, 700.0));
    bez.curve_to((201.0, 700.0), (51.0, 653.0), (51.0, 482.0));
    bez.close_path();

    bez.move_to((202.0, 172.0));
    bez.line_to((275.0, 172.0));
    bez.line_to((275.0, 105.0));
    bez.line_to((203.0, 105.0));
    bez.line_to((202.0, 172.0));
    bez.close_path();
    bez
}

/// Move the contents of the file at `path` to another location.
///
/// If `path` exists, returns the backup location on success.
fn backup_ufo_at_path(path: &Path) -> Result<Option<PathBuf>, std::io::Error> {
    if !path.exists() {
        return Ok(None);
    }
    let backup_dir = format!(
        "{}_backups",
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
    );
    let mut backup_dir = path.with_file_name(backup_dir);
    if !backup_dir.exists() {
        fs::create_dir(&backup_dir)?;
    }

    let backup_date = chrono::Local::now();
    let date_str = backup_date.format("%Y-%m-%d_%H&%M%S.ufo");
    backup_dir.push(date_str.to_string());
    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir)?;
    }
    fs::rename(path, &backup_dir)?;
    Ok(Some(backup_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn font_info_changes() {
        let mut fontobj = FontObject::default();
        let font_info = fontobj.ufo.font_info.clone().unwrap();
        assert!(font_info.style_name.is_none());
        assert!(font_info.descender.is_none());
        assert_eq!(font_info.family_name, Some("Untitled".to_string()));

        let mut info = SimpleFontInfo::from_font(&fontobj);
        info.metrics.descender = Some(420.);
        info.style_name = "Extra Cheese".into();

        fontobj.update_info(&info);
        let font_info = fontobj.ufo.font_info.clone().unwrap();
        assert_eq!(font_info.style_name, Some("Extra Cheese".to_string()));
        assert_eq!(font_info.descender, Some(420.0.into()));

        // see that it also works if there's _no_ font info:
        fontobj.ufo.font_info = None;

        fontobj.update_info(&info);
        let font_info = fontobj.ufo.font_info.clone().unwrap();
        assert_eq!(font_info.style_name, Some("Extra Cheese".to_string()));
        assert_eq!(font_info.descender, Some(420.0.into()));
    }
}
