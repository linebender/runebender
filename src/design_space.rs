//! 'Design space' is the fixed coordinate space in which we describe glyphs,
//! guides, and other entities.
//!
//! When drawing to the screen or handling mouse input, we need to translate from
//! 'screen space' to design space, taking into account things like the current
//! scroll offset and zoom level.

use std::fmt;
use std::ops::{Add, Sub};

use druid::kurbo::{Affine, Point, Rect, Vec2};
use druid::Data;

/// The position of the view, relative to the design space.
#[derive(Debug, Clone, Copy, PartialEq)]
//TODO: rename to DesignSpace
pub struct ViewPort {
    /// The offset from (0, 0) in view space (the top left corner) and (0, 0) in
    /// design space, which is the intersection of the baseline and the left sidebearing.
    ///
    /// # Note:
    ///
    /// This does not account for zoom. Zoom must be applied when using this to
    /// derive a screen point.
    offset: Vec2,
    pub zoom: f64,
    /// Whether or not the y axis is inverted between view and design space.
    ///
    /// This is always `true`. It exists to make this code more readable.
    pub flipped_y: bool,
}

/// A point in design space.
///
/// This type should only be constructed through a function that has access to,
/// and takes account of, the current viewport.
#[derive(Clone, Copy, Data, PartialEq)]
pub struct DPoint {
    pub x: f64,
    pub y: f64,
}

/// A vector in design space, used for nudging & dragging
#[derive(Debug, Clone, Copy, Data, PartialEq)]
pub struct DVec2 {
    pub x: f64,
    pub y: f64,
}

impl DPoint {
    /// Should only be used with inputs already in design space, such as when
    /// loaded from file.
    pub(crate) fn new(x: f64, y: f64) -> DPoint {
        assert!(
            x.is_finite() && y.is_finite() && x.fract() == 0. && y.fract() == 0.,
            "({}, {})",
            x,
            y
        );
        DPoint { x, y }
    }

    pub fn from_screen(point: Point, vport: ViewPort) -> DPoint {
        vport.from_screen(point)
    }

    pub fn to_screen(self, vport: ViewPort) -> Point {
        vport.to_screen(self)
    }

    /// Create a new `DPoint` from a `Point` in design space. This should only
    /// be used to convert back to a `DPoint` after using `Point` to do vector
    /// math in design space.
    //TODO: don't expose these, implement the fns you need
    fn from_raw(point: impl Into<Point>) -> DPoint {
        let point = point.into();
        DPoint::new(point.x.round(), point.y.round())
    }

    /// Convert a design point directly to a point, without taking screen geometry
    /// into account.
    ///
    /// We don't really want to use this, but it's useful sometimes for using
    /// operations available on `Point`.
    #[doc(hidden)]
    //TODO: reevaluate
    pub(super) fn to_raw(self) -> Point {
        Point::new(self.x, self.y)
    }
}

impl DVec2 {
    fn new(x: f64, y: f64) -> DVec2 {
        assert!(x.is_finite() && y.is_finite() && x.fract() == 0. && y.fract() == 0.);
        DVec2 { x, y }
    }

    pub fn from_raw(vec2: impl Into<Vec2>) -> DVec2 {
        let vec2 = vec2.into();
        DVec2::new(vec2.x.round(), vec2.y.round())
    }

    /// should not be public, used internally so we can reuse math ops
    #[doc(hidden)]
    #[inline]
    pub(super) fn to_raw(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    #[inline]
    pub fn hypot(self) -> f64 {
        self.to_raw().hypot()
    }
}

impl ViewPort {
    pub fn set_offset(&mut self, offset: Vec2) {
        self.offset = offset;
    }

    pub fn affine(&self) -> Affine {
        let y_scale = if self.flipped_y {
            -self.zoom
        } else {
            self.zoom
        };
        let offset = self.offset * self.zoom;
        Affine::new([self.zoom, 0.0, 0.0, y_scale, offset.x, offset.y])
    }

    pub fn inverse_affine(&self) -> Affine {
        self.affine().inverse()
    }

    pub fn from_screen(&self, point: impl Into<Point>) -> DPoint {
        let point = self.inverse_affine() * point.into();
        DPoint::new(point.x.round(), point.y.round())
    }

    pub fn to_screen(&self, point: impl Into<DPoint>) -> Point {
        self.affine() * point.into().to_raw()
    }

    // rects get special treatment because they can't be transformed with an affine
    pub fn rect_to_screen(&self, rect: Rect) -> Rect {
        let p0 = self.to_screen(DPoint::from_raw(rect.origin()));
        let p1 = self.to_screen(DPoint::from_raw((rect.x1, rect.y1)));
        Rect::from_points(p0, p1)
    }
}

impl Add<DVec2> for DPoint {
    type Output = DPoint;

    #[inline]
    fn add(self, other: DVec2) -> Self {
        DPoint::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub<DVec2> for DPoint {
    type Output = DPoint;

    #[inline]
    fn sub(self, other: DVec2) -> Self {
        DPoint::new(self.x - other.x, self.y - other.y)
    }
}

impl Sub<DPoint> for DPoint {
    type Output = DVec2;

    #[inline]
    fn sub(self, other: DPoint) -> DVec2 {
        DVec2::new(self.x - other.x, self.y - other.y)
    }
}

impl Add for DVec2 {
    type Output = DVec2;

    #[inline]
    fn add(self, other: DVec2) -> DVec2 {
        DVec2::new((self.x + other.x).round(), (self.y + other.y).round())
    }
}

impl Sub for DVec2 {
    type Output = DVec2;

    #[inline]
    fn sub(self, other: DVec2) -> DVec2 {
        DVec2::new(self.x - other.x, self.y - other.y)
    }
}

impl From<(f64, f64)> for DPoint {
    fn from(src: (f64, f64)) -> DPoint {
        DPoint::new(src.0.round(), src.1.round())
    }
}

impl fmt::Debug for DPoint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "D({:?}, {:?})", self.x, self.y)
    }
}

impl fmt::Display for DPoint {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "D(")?;
        fmt::Display::fmt(&self.x, formatter)?;
        write!(formatter, ", ")?;
        fmt::Display::fmt(&self.y, formatter)?;
        write!(formatter, ")")
    }
}

impl fmt::Display for DVec2 {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "D𝐯=(")?;
        fmt::Display::fmt(&self.x, formatter)?;
        write!(formatter, ", ")?;
        fmt::Display::fmt(&self.y, formatter)?;
        write!(formatter, ")")
    }
}

impl Data for ViewPort {
    fn same(&self, other: &ViewPort) -> bool {
        self.offset.x.same(&other.offset.x)
            && self.offset.y.same(&other.offset.y)
            && self.zoom.same(&other.zoom)
    }
}

impl std::default::Default for ViewPort {
    fn default() -> Self {
        ViewPort {
            offset: Vec2::ZERO,
            zoom: 1.0,
            flipped_y: true,
        }
    }
}
