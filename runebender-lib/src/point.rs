//! Points and sequences of points, used to represent paths.
//!
//! This is intended to be agnostic to whether the path is a bezier or a
//! hyperbezier.

use super::design_space::{DPoint, DVec2, ViewPort};

use druid::kurbo::{Affine, Point};
use druid::Data;
use norad::glyph::PointType as NoradPointType;

const RESERVED_ID_COUNT: IdComponent = 5;
const NO_PARENT_TYPE_ID: IdComponent = 0;
const GUIDE_TYPE_ID: IdComponent = 1;

type IdComponent = usize;

#[derive(Debug, Clone, Copy, Data, PartialEq, PartialOrd, Hash, Eq, Ord)]
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

#[derive(Debug, Clone, Copy, PartialEq, Data, Deserialize, Serialize)]
pub enum PointType {
    OnCurve { smooth: bool },
    OffCurve { auto: bool },
}

#[derive(Clone, Copy, Data, PartialEq, Serialize, Deserialize)]
pub struct PathPoint {
    #[serde(skip, default = "EntityId::next")]
    pub id: EntityId,
    pub point: DPoint,
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

    pub fn from_norad(norad_type: &NoradPointType, smooth: bool) -> Self {
        match norad_type {
            NoradPointType::OffCurve => PointType::OffCurve { auto: false },
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

    pub fn auto(path: EntityId, point: DPoint) -> PathPoint {
        let id = EntityId::new_with_parent(path);
        PathPoint {
            id,
            point,
            typ: PointType::OffCurve { auto: true },
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

    pub fn to_kurbo(&self) -> Point {
        self.point.to_raw()
    }

    /// The distance, in screen space, from this `PathPoint` to `point`, a point
    /// in screen space.
    pub fn screen_dist(&self, vport: ViewPort, point: Point) -> f64 {
        self.point.to_screen(vport).distance(point)
    }

    /// Convert this point to point in screen space.
    pub fn to_screen(&self, vport: ViewPort) -> Point {
        self.point.to_screen(vport)
    }
}

impl std::fmt::Debug for PathPoint {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}: {:.2} {:?}", self.id, self.point, self.typ)
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "id{}.{}", self.parent, self.point)
    }
}
