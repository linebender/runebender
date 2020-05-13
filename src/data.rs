//! Application state.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use druid::kurbo::{Affine, BezPath, Point, Rect, Size};
use druid::{Data, Lens, WindowId};
use norad::glyph::{Contour, ContourPoint, Glyph, GlyphName, PointType};
use norad::{FontInfo, Ufo};

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
    pub info: SimpleFontInfo,
}

#[derive(Clone, Data)]
pub struct FontObject {
    pub path: Option<Arc<Path>>,
    #[data(ignore)]
    pub ufo: Ufo,
    placeholder: Arc<BezPath>,
}

/// A glyph, plus access to the main UFO in order to resolve components in that
/// glyph.
#[derive(Clone, Data, Lens)]
pub struct GlyphPlus {
    pub glyph: Arc<Glyph>,
    outline: Arc<BezPath>,
    is_placeholder: bool,
    pub is_selected: bool,
    units_per_em: f64,
}

//TODO: this is currently just used for editing font attributes, and isn't propogated
//FIXME: use this as source of truth, synch back to UFO
#[derive(Clone, Default, Data, Lens)]
pub struct SimpleFontInfo {
    metrics: FontMetrics,
    pub family_name: String,
    pub style_name: String,
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
#[derive(Clone, Data)]
pub struct EditorState {
    pub metrics: FontMetrics,
    pub font: Workspace,
    pub session: Arc<EditSession>,
}

impl Workspace {
    pub fn set_file(&mut self, ufo: Ufo, path: impl Into<Option<PathBuf>>) {
        let obj = FontObject {
            path: path.into().map(Into::into),
            ufo,
            placeholder: Arc::new(placeholder_outline()),
        };
        self.font = obj.into();
        self.info = SimpleFontInfo::from_font(&self.font);
    }

    pub fn save(&mut self) -> Result<(), Box<dyn Error>> {
        let font_obj = Arc::make_mut(&mut self.font);
        if let Some(path) = font_obj.path.as_ref() {
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
            let tmp_file_name = format!(
                "{}.savefile",
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
            );
            let tmp_path = path.with_file_name(tmp_file_name);
            if tmp_path.exists() {
                fs::remove_dir_all(&tmp_path)?;
            }
            font_obj.ufo.save(&tmp_path)?;
            if path.exists() {
                fs::remove_dir_all(path)?;
            }
            // see docs for fs::rename; the target directory must exist on unix.
            // http://doc.rust-lang.org/1.39.0/std/fs/fn.rename.html
            if cfg!(unix) {
                fs::create_dir(path)?;
            }
            fs::rename(&tmp_path, path)?;
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

    /// Given a glyph name, a `Ufo`, and an optional cache, returns the fully resolved
    /// (including all sub components) `BezPath` for this glyph.
    pub fn get_bezier(&self, name: &GlyphName) -> Option<Arc<BezPath>> {
        let glyph = self
            .session_map
            .get(name)
            .and_then(|name| self.sessions.get(name).map(|s| &s.glyph))
            .or_else(|| self.font.ufo.get_glyph(name))?;
        let path = path_for_glyph(glyph)?;
        Some(self.resolve_components(glyph, path))
    }

    /// takes a glyph outline and appends the outlines of any components,
    /// resolving them as necessary, and caching the results.
    fn resolve_components(&self, glyph: &Glyph, mut bez: BezPath) -> Arc<BezPath> {
        for comp in glyph
            .outline
            .as_ref()
            .iter()
            .flat_map(|o| o.components.iter())
        {
            match self.get_bezier(&comp.base) {
                Some(comp_path) => {
                    let affine: Affine = comp.transform.clone().into();
                    for comp_elem in (affine * &*comp_path).elements() {
                        bez.push(*comp_elem);
                    }
                }
                None => log::warn!("missing component {} in glyph {}", comp.base, glyph.name),
            }
        }

        Arc::new(bez)
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

impl GlyphPlus {
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
    pub fn codepoint(&self) -> Option<char> {
        self.glyph
            .codepoints
            .as_ref()
            .and_then(|v| v.first())
            .cloned()
    }

    /// The upm for the font this glyph belongs to.
    pub fn upm(&self) -> f64 {
        self.units_per_em
    }
}

impl EditorState {
    /// Returns a rect representing the metric bounds of this glyph; that is,
    /// taking into account the font metrics (ascender, descender) as well as the
    /// glyph's width.
    ///
    /// This rect is in the same coordinate space as the glyph; the origin
    /// is at the intersection of the baseline and the left sidebearing,
    /// and y is up.
    fn layout_bounds(&self) -> Rect {
        let upm = self.metrics.units_per_em;
        let ascender = self.metrics.ascender.unwrap_or(upm * 0.8);
        let descender = self.metrics.descender.unwrap_or(upm * -0.2);
        let width = self
            .session
            .glyph
            .advance
            .as_ref()
            .map(|a| a.width as f64)
            .unwrap_or(upm * 0.5);

        let work_size = Size::new(width, ascender + descender.abs());
        let work_origin = Point::new(0., descender);
        Rect::from_origin_size(work_origin, work_size)
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
}

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
                .and_then(|f| f.family_name.clone())
                .unwrap_or_else(|| "Untitled".to_string()),
            style_name: font
                .ufo
                .font_info
                .as_ref()
                .and_then(|f| f.style_name.clone())
                .unwrap_or_else(|| "Regular".to_string()),
            metrics: font
                .ufo
                .font_info
                .as_ref()
                .map(FontMetrics::from)
                .unwrap_or_default(),
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

pub mod lenses {
    pub mod app_state {
        use std::sync::Arc;

        use druid::{Data, Lens};
        use norad::GlyphName as GlyphName_;

        use super::super::{EditorState as EditorState_, GlyphPlus, SessionId, Workspace};

        /// Workspace -> EditorState
        pub struct EditorState(pub SessionId);

        /// GlyphSet_ -> GlyphPlus
        pub struct Glyph(pub GlyphName_);

        /// GlyphPlus => GlyphName_
        pub struct GlyphName;

        /// GlyphPlus -> char
        pub struct Codepoint;

        pub struct SelectedGlyph;

        pub struct Advance;

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

            fn with_mut<V, F: FnOnce(&mut EditorState_) -> V>(
                &self,
                data: &mut Workspace,
                f: F,
            ) -> V {
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
                    Arc::make_mut(&mut data.sessions).insert(self.0.clone(), glyph.session);
                }
                v
            }
        }

        impl Lens<Workspace, Option<GlyphPlus>> for Glyph {
            fn with<V, F: FnOnce(&Option<GlyphPlus>) -> V>(&self, data: &Workspace, f: F) -> V {
                let glyph = data.font.ufo.get_glyph(&self.0).map(|g| {
                    let outline = data.get_bezier(&g.name);
                    let is_selected = data.selected.as_ref() == Some(&self.0);
                    GlyphPlus {
                        glyph: Arc::clone(g),
                        is_placeholder: outline.is_none(),
                        outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                        units_per_em: data.units_per_em(),
                        is_selected,
                    }
                });
                f(&glyph)
            }

            fn with_mut<V, F: FnOnce(&mut Option<GlyphPlus>) -> V>(
                &self,
                data: &mut Workspace,
                f: F,
            ) -> V {
                //FIXME: this is creating a new copy and then throwing it away
                //this is just so that the signatures work for now, we aren't actually doing any
                //mutating
                let mut glyph = data.font.ufo.get_glyph(&self.0).map(|glyph| {
                    let outline = data.get_bezier(&glyph.name);
                    let is_placeholder = outline.is_none();
                    let is_selected = data.selected.as_ref() == Some(&self.0);
                    GlyphPlus {
                        glyph: Arc::clone(glyph),
                        outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                        units_per_em: data.units_per_em(),
                        is_placeholder,
                        is_selected,
                    }
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

        impl Lens<GlyphPlus, Option<char>> for Codepoint {
            fn with<V, F: FnOnce(&Option<char>) -> V>(&self, data: &GlyphPlus, f: F) -> V {
                let c = data.codepoint();
                f(&c)
            }

            fn with_mut<V, F: FnOnce(&mut Option<char>) -> V>(
                &self,
                data: &mut GlyphPlus,
                f: F,
            ) -> V {
                let mut c = data.codepoint();
                let r = f(&mut c);
                let old = data.codepoint();
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

        impl Lens<Workspace, Option<GlyphPlus>> for SelectedGlyph {
            fn with<V, F: FnOnce(&Option<GlyphPlus>) -> V>(&self, data: &Workspace, f: F) -> V {
                let selected = data.selected.as_ref().map(|name| {
                    let glyph = data
                        .font
                        .ufo
                        .get_glyph(name)
                        .expect("missing glyph in lens");
                    let outline = data.get_bezier(&glyph.name);
                    let is_placeholder = outline.is_none();
                    let is_selected = data.selected.as_ref() == Some(&name);
                    GlyphPlus {
                        glyph: Arc::clone(glyph),
                        outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                        units_per_em: data.units_per_em(),
                        is_placeholder,
                        is_selected,
                    }
                });
                f(&selected)
            }

            fn with_mut<V, F: FnOnce(&mut Option<GlyphPlus>) -> V>(
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
                    let is_selected = data.selected.as_ref() == Some(&name);
                    GlyphPlus {
                        glyph: Arc::clone(glyph),
                        outline: outline.unwrap_or_else(|| data.font.placeholder.clone()),
                        units_per_em: data.units_per_em(),
                        is_placeholder,
                        is_selected,
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

        impl Lens<GlyphPlus, f32> for Advance {
            fn with<V, F: FnOnce(&f32) -> V>(&self, data: &GlyphPlus, f: F) -> V {
                let advance = data.glyph.advance.as_ref().map(|a| a.width).unwrap_or(0.);
                f(&advance)
            }

            #[allow(clippy::float_cmp)]
            fn with_mut<V, F: FnOnce(&mut f32) -> V>(&self, data: &mut GlyphPlus, f: F) -> V {
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

        impl Lens<GlyphPlus, GlyphName_> for GlyphName {
            fn with<V, F: FnOnce(&GlyphName_) -> V>(&self, data: &GlyphPlus, f: F) -> V {
                f(&data.glyph.name)
            }

            fn with_mut<V, F: FnOnce(&mut GlyphName_) -> V>(
                &self,
                data: &mut GlyphPlus,
                f: F,
            ) -> V {
                // THIS DOESN'T DO ANYTHING! all the mutation happens
                // as a result of the RENAME_GLYPH command.
                let mut s = data.glyph.name.clone();
                f(&mut s)
            }
        }
    }
}

/// Convert this glyph's path from the UFO representation into a `kurbo::BezPath`
/// (which we know how to draw.)
fn path_for_glyph(glyph: &Glyph) -> Option<BezPath> {
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
