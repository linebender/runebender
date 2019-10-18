//! Application state.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use druid::kurbo::{Affine, BezPath, Point};
use druid::{Data, WindowId};
use norad::glyph::{Contour, ContourPoint, Glyph, GlyphName, PointType};
use norad::{FontInfo, FormatVersion, MetaInfo, Ufo};

/// This is by convention.
const DEFAULT_UNITS_PER_EM: f64 = 1000.;

/// A glyph that has been opened is either waiting for a window to connect,
/// or is in a window.
#[derive(Debug, Clone)]
pub enum OpenGlyph {
    Pending,
    Window(WindowId),
}

#[derive(Clone, Default)]
pub struct AppState {
    pub file: FontObject,
    /// glyphs that are already open in an editor window
    pub open_glyphs: Arc<HashMap<GlyphName, OpenGlyph>>,
}

/// A shared map from glyph names to resolved `BezPath`s.
type BezCache = Arc<RefCell<HashMap<String, Arc<BezPath>>>>;

#[derive(Clone)]
pub struct FontObject {
    pub path: Option<PathBuf>,
    pub object: Arc<Ufo>,
    resolved: BezCache,
}

/// The main data type for the grid view.
#[derive(Clone, Data)]
pub struct GlyphSet {
    pub object: Arc<Ufo>,
    resolved: BezCache,
}

/// A glyph, plus access to the main UFO in order to resolve components in that
/// glyph.
#[derive(Clone, Data)]
pub struct GlyphPlus {
    pub glyph: Arc<Glyph>,
    pub ufo: Arc<Ufo>,
    resolved: BezCache,
}

/// Things in `FontInfo` that are relevant while editing or drawing.
#[derive(Clone, Data)]
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
    pub glyph: Arc<Glyph>,
    pub metrics: FontMetrics,
    pub ufo: Arc<Ufo>,
}

impl AppState {
    pub fn set_file(&mut self, object: Ufo, path: impl Into<Option<PathBuf>>) {
        let obj = FontObject {
            path: path.into(),
            object: Arc::new(object),
            resolved: Arc::new(Default::default()),
        };
        self.file = obj;
    }
}

impl GlyphPlus {
    /// Get the fully resolved (including components) bezier path for this glyph.
    pub fn get_bezier(&self) -> Option<Arc<BezPath>> {
        get_bezier(&self.glyph.name, &self.ufo, Some(&self.resolved))
    }
}

/// Given a glyph name, a `Ufo`, and an optional cache, returns the fully resolved
/// (including all sub components) `BezPath` for this glyph.
pub fn get_bezier(name: &str, ufo: &Arc<Ufo>, resolved: Option<&BezCache>) -> Option<Arc<BezPath>> {
    if let Some(resolved) = resolved.and_then(|r| r.borrow().get(name).map(Arc::clone)) {
        return Some(resolved);
    }

    let glyph = ufo.get_glyph(name)?;
    let mut path = path_for_glyph(glyph);
    for comp in glyph
        .outline
        .as_ref()
        .iter()
        .flat_map(|o| o.components.iter())
    {
        match get_bezier(&comp.base, ufo, resolved) {
            Some(comp_path) => {
                let affine: Affine = comp.transform.clone().into();
                for comp_elem in (affine * &*comp_path).elements() {
                    path.push(*comp_elem);
                }
            }
            None => log::warn!("missing component {} in glyph {}", comp.base, name),
        }
    }

    let path = Arc::new(path);
    if let Some(resolved) = resolved {
        resolved.borrow_mut().insert(name.to_string(), path.clone());
    }
    Some(path)
}

impl Data for FontObject {
    fn same(&self, other: &Self) -> bool {
        self.path == other.path
            && other.object.same(&self.object)
            && self.resolved.same(&other.resolved)
    }
}

impl Data for AppState {
    fn same(&self, other: &Self) -> bool {
        self.file.same(&other.file)
    }
}

impl std::default::Default for FontObject {
    fn default() -> FontObject {
        let meta = MetaInfo {
            creator: "Runebender".into(),
            format_version: FormatVersion::V3,
        };

        let font_info = FontInfo {
            family_name: Some(String::from("Untitled")),
            ..Default::default()
        };

        let mut ufo = Ufo::new(meta);
        ufo.font_info = Some(font_info);

        FontObject {
            path: None,
            object: Arc::new(ufo),
            resolved: Arc::new(Default::default()),
        }
    }
}

impl<'a> From<&'a FontInfo> for FontMetrics {
    fn from(src: &'a FontInfo) -> FontMetrics {
        FontMetrics {
            units_per_em: src.units_per_em.unwrap_or(DEFAULT_UNITS_PER_EM),
            descender: src.descender,
            x_height: src.x_height,
            cap_height: src.cap_height,
            ascender: src.ascender,
            italic_angle: src.italic_angle,
        }
    }
}

impl std::default::Default for FontMetrics {
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

        use super::super::{AppState, EditorState as EditorState_, GlyphSet as GlyphSet_};
        use crate::lens2::Lens2;
        use norad::GlyphName;

        /// AppState -> GlyphSet_
        pub struct GlyphSet;

        /// AppState -> EditorState
        pub struct EditorState(pub GlyphName);

        impl Lens2<AppState, GlyphSet_> for GlyphSet {
            fn get<V, F: FnOnce(&GlyphSet_) -> V>(&self, data: &AppState, f: F) -> V {
                let glyphs = GlyphSet_ {
                    object: Arc::clone(&data.file.object),
                    resolved: Arc::clone(&data.file.resolved),
                };
                f(&glyphs)
            }
            fn with_mut<V, F: FnOnce(&mut GlyphSet_) -> V>(&self, data: &mut AppState, f: F) -> V {
                let mut glyphs = GlyphSet_ {
                    object: Arc::clone(&data.file.object),
                    resolved: Arc::clone(&data.file.resolved),
                };
                f(&mut glyphs)
            }
        }

        impl Lens2<AppState, EditorState_> for EditorState {
            fn get<V, F: FnOnce(&EditorState_) -> V>(&self, data: &AppState, f: F) -> V {
                let glyph = data
                    .file
                    .object
                    .get_glyph(&self.0)
                    .expect("missing glyph in lens2");
                let metrics = data
                    .file
                    .object
                    .font_info
                    .as_ref()
                    .map(Into::into)
                    .unwrap_or_default();
                let glyph = EditorState_ {
                    glyph: Arc::clone(glyph),
                    ufo: Arc::clone(&data.file.object),
                    metrics,
                };
                f(&glyph)
            }

            fn with_mut<V, F: FnOnce(&mut EditorState_) -> V>(
                &self,
                data: &mut AppState,
                f: F,
            ) -> V {
                //FIXME: this is creating a new copy and then throwing it away
                //this is just so that the signatures work for now, we aren't actually doing any
                //mutating
                let glyph = data
                    .file
                    .object
                    .get_glyph(&self.0)
                    .expect("missing glyph in lens2");
                let metrics = data
                    .file
                    .object
                    .font_info
                    .as_ref()
                    .map(Into::into)
                    .unwrap_or_default();
                let mut glyph = EditorState_ {
                    glyph: Arc::clone(glyph),
                    ufo: Arc::clone(&data.file.object),
                    metrics,
                };
                f(&mut glyph)
            }
        }
    }

    pub mod glyph_set {
        use norad::GlyphName;
        use std::sync::Arc;

        use super::super::{GlyphPlus, GlyphSet as GlyphSet_};
        use crate::lens2::Lens2;

        /// GlyphSet_ -> GlyphPlus
        pub struct Glyph(pub GlyphName);

        impl Lens2<GlyphSet_, GlyphPlus> for Glyph {
            fn get<V, F: FnOnce(&GlyphPlus) -> V>(&self, data: &GlyphSet_, f: F) -> V {
                let glyph = data
                    .object
                    .get_glyph(&self.0)
                    .expect("missing glyph in lens2");
                let glyph = GlyphPlus {
                    glyph: Arc::clone(glyph),
                    ufo: Arc::clone(&data.object),
                    resolved: Arc::clone(&data.resolved),
                };
                f(&glyph)
            }

            fn with_mut<V, F: FnOnce(&mut GlyphPlus) -> V>(&self, data: &mut GlyphSet_, f: F) -> V {
                //FIXME: this is creating a new copy and then throwing it away
                //this is just so that the signatures work for now, we aren't actually doing any
                //mutating
                let glyph = data
                    .object
                    .get_glyph(&self.0)
                    .expect("missing glyph in lens2");
                let mut glyph = GlyphPlus {
                    glyph: Arc::clone(glyph),
                    ufo: Arc::clone(&data.object),
                    resolved: Arc::clone(&data.resolved),
                };
                f(&mut glyph)
            }
        }
    }
}

/// Convert this glyph's path from the UFO representation into a `kurbo::BezPath`
/// (which we know how to draw.)
pub fn path_for_glyph(glyph: &Glyph) -> BezPath {
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
                    eprintln!("TODO: handle qcurve");
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

    let mut path = BezPath::new();
    if let Some(outline) = glyph.outline.as_ref() {
        outline
            .contours
            .iter()
            .for_each(|c| add_contour(&mut path, c));
    }
    path
}
