//! Points and sequences of points, used to represent paths.
//!
//! This is intended to be agnostic to whether the path is a bezier or a
//! hyperbezier.

use super::design_space::{DPoint, DVec2, ViewPort};

use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

use druid::kurbo::{Affine, Point};
use druid::Data;
use norad::glyph::{ContourPoint, PointType as NoradPointType};

pub(crate) static HYPERBEZ_AUTO_POINT_KEY: &str = "org.linebender.hyperbezier-auto-point";

const RESERVED_ID_COUNT: IdComponent = 5;
const NO_PARENT_TYPE_ID: IdComponent = 0;
const GUIDE_TYPE_ID: IdComponent = 1;

type IdComponent = usize;

#[derive(Clone, Copy, Data, PartialEq, PartialOrd, Hash, Eq, Ord)]
/// A unique identifier for some entity, such as a point or a component.
///
/// The entity has two parts; the first ('parent') identifies the type of the
/// entity or the identity of its containing path (for points) and the second
/// ('child') identifies the item itself.
///
/// A given id will be unique across the application at any given time.
pub struct EntityId {
    parent: IdComponent,
    point: IdComponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Data)]
pub enum PointType {
    OnCurve { smooth: bool },
    OffCurve { auto: bool },
}

#[derive(Clone, Copy, Data, PartialEq, Serialize, Deserialize)]
pub struct PathPoint {
    #[serde(skip, default = "EntityId::next")]
    pub id: EntityId,
    pub point: DPoint,
    #[serde(rename = "type")]
    pub typ: PointType,
}

impl EntityId {
    /// Returns a new unique id.
    ///
    /// This id will have no associated parent. If you want a parent idea,
    /// use [`EntityId::new_with_parent`].
    pub fn next() -> EntityId {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT_ID: AtomicUsize = AtomicUsize::new(RESERVED_ID_COUNT);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        EntityId {
            parent: NO_PARENT_TYPE_ID,
            point: id,
        }
    }

    pub fn new_with_parent(parent: EntityId) -> Self {
        EntityId {
            parent: parent.point,
            ..EntityId::next()
        }
    }

    #[inline]
    pub fn new_for_guide() -> Self {
        EntityId {
            parent: GUIDE_TYPE_ID,
            ..EntityId::next()
        }
    }

    /// Return the `EntityId` representing this id's parent.
    ///
    /// If this entity's parent has its own parent, it will not be present.
    pub fn parent(self) -> EntityId {
        EntityId {
            parent: NO_PARENT_TYPE_ID,
            point: self.parent,
        }
    }

    pub fn is_guide(self) -> bool {
        self.parent == GUIDE_TYPE_ID
    }

    pub(crate) fn parent_eq(self, other: EntityId) -> bool {
        self.parent == other.parent
    }

    pub(crate) fn is_child_of(self, other: EntityId) -> bool {
        self.parent == other.point
    }
}

impl PointType {
    pub(crate) fn debug_name(self) -> &'static str {
        match self {
            PointType::OnCurve { smooth: false } => ON_CURVE_CORNER,
            PointType::OnCurve { smooth: true } => ON_CURVE_SMOOTH,
            PointType::OffCurve { auto: true } => OFF_CURVE_AUTO,
            PointType::OffCurve { auto: false } => OFF_CURVE_MANUAL,
        }
    }

    pub fn is_on_curve(self) -> bool {
        matches!(self, PointType::OnCurve { .. })
    }

    pub fn is_off_curve(self) -> bool {
        matches!(self, PointType::OffCurve { .. })
    }

    pub fn is_smooth(&self) -> bool {
        matches!(self, PointType::OnCurve { smooth: true })
    }

    /// Toggle smooth to corner and auto to non-auto
    pub fn toggle_type(&mut self) {
        match self {
            PointType::OnCurve { smooth } => *smooth = !*smooth,
            PointType::OffCurve { auto } => *auto = !*auto,
        }
    }

    pub fn from_norad(norad_point: &ContourPoint) -> Self {
        let smooth = norad_point.smooth;
        let auto = norad_point
            .lib()
            .and_then(|lib| {
                lib.get(HYPERBEZ_AUTO_POINT_KEY).map(|is_auto| {
                    is_auto
                        .as_boolean()
                        .expect("invalid hyperbez auto key type?")
                })
            })
            .unwrap_or(false);
        match &norad_point.typ {
            NoradPointType::OffCurve => PointType::OffCurve { auto },
            NoradPointType::QCurve => panic!(
                "quadratics unsupported, we should have \
                         validated input before now"
            ),
            NoradPointType::Move | NoradPointType::Line | NoradPointType::Curve if smooth => {
                PointType::OnCurve { smooth: true }
            }
            _other => PointType::OnCurve { smooth: false },
        }
    }
}

impl PathPoint {
    pub fn off_curve(path: EntityId, point: DPoint) -> PathPoint {
        let id = EntityId::new_with_parent(path);
        PathPoint {
            id,
            point,
            typ: PointType::OffCurve { auto: false },
        }
    }

    pub fn on_curve(path: EntityId, point: DPoint) -> PathPoint {
        let id = EntityId::new_with_parent(path);
        PathPoint {
            id,
            point,
            typ: PointType::OnCurve { smooth: false },
        }
    }

    pub fn on_curve_smooth(path: EntityId, point: DPoint) -> PathPoint {
        let id = EntityId::new_with_parent(path);
        PathPoint {
            id,
            point,
            typ: PointType::OnCurve { smooth: true },
        }
    }

    pub fn hyper_off_curve(path: EntityId, point: DPoint, auto: bool) -> PathPoint {
        let id = EntityId::new_with_parent(path);
        PathPoint {
            id,
            point,
            typ: PointType::OffCurve { auto },
        }
    }

    pub fn is_on_curve(&self) -> bool {
        self.typ.is_on_curve()
    }

    pub fn is_auto(&self) -> bool {
        matches!(self.typ, PointType::OffCurve { auto: true })
    }

    pub fn is_off_curve(&self) -> bool {
        self.typ.is_off_curve()
    }

    pub fn is_smooth(&self) -> bool {
        self.typ.is_smooth()
    }

    pub fn toggle_type(&mut self) {
        self.typ.toggle_type();
    }

    pub fn reparent(&mut self, new_parent: EntityId) {
        self.id.parent = new_parent.point;
    }

    /// Apply the provided transform to the point.
    ///
    /// The `anchor` argument is a point that should be treated as the origin
    /// when applying the transform, which is used for things like scaling from
    /// a fixed point.
    pub fn transform(&mut self, affine: Affine, anchor: DVec2) {
        let anchor = anchor.to_raw();
        let current = self.point.to_raw() - anchor;
        let new = affine * current + anchor;
        self.point = DPoint::from_raw(new);
    }

    pub fn to_kurbo(self) -> Point {
        self.point.to_raw()
    }

    /// The distance, in screen space, from this `PathPoint` to `point`, a point
    /// in screen space.
    pub fn screen_dist(&self, vport: ViewPort, point: Point) -> f64 {
        self.point.to_screen(vport).distance(point)
    }

    /// Convert this point to point in screen space.
    #[allow(clippy::wrong_self_convention)]
    pub fn to_screen(&self, vport: ViewPort) -> Point {
        self.point.to_screen(vport)
    }
}

const ON_CURVE_CORNER: &str = "OnCurve";
const ON_CURVE_SMOOTH: &str = "OnCurveSmooth";
const OFF_CURVE_MANUAL: &str = "OffCurve";
const OFF_CURVE_AUTO: &str = "OffCurveAuto";

impl Serialize for PointType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            PointType::OnCurve { smooth: false } => ON_CURVE_CORNER,
            PointType::OnCurve { smooth: true } => ON_CURVE_SMOOTH,
            PointType::OffCurve { auto: false } => OFF_CURVE_MANUAL,
            PointType::OffCurve { auto: true } => OFF_CURVE_AUTO,
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for PointType {
    fn deserialize<D>(deserializer: D) -> Result<PointType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let type_string: &str = Deserialize::deserialize(deserializer)?;
        match type_string {
            ON_CURVE_SMOOTH => Ok(PointType::OnCurve { smooth: true }),
            ON_CURVE_CORNER => Ok(PointType::OnCurve { smooth: false }),
            OFF_CURVE_MANUAL => Ok(PointType::OffCurve { auto: false }),
            OFF_CURVE_AUTO => Ok(PointType::OffCurve { auto: true }),
            other => Err(D::Error::custom(format!("invalid point type '{}'", other))),
        }
    }
}

impl std::fmt::Debug for PathPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Point({:?}): {}({:.2})",
            self.id,
            self.typ.debug_name(),
            self.point
        )
    }
}

impl std::fmt::Debug for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "id{}.{}", self.parent, self.point)
    }
}
