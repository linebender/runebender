//! 'Design space' is the fixed coordinate space in which we describe glyphs,
//! guides, and other entities.
//!
//! When drawing to the screen or handling mouse input, we need to translate from
//! 'screen space' to design space, taking into account things like the current
//! scroll offset and zoom level.

use std::fmt;
use std::ops::{Add, Mul, Sub};

use druid::kurbo::{Point, TranslateScale, Vec2};
use druid::{Data, WheelEvent};

const MIN_ZOOM: f64 = 0.2;
const MAX_ZOOM: f64 = 50.;
const MIN_SCROLL: f64 = -5000.;
const MAX_SCROLL: f64 = 5000.;

/// The position of the view, relative to the design space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewPort {
    pub offset: Vec2,
    pub zoom: f64,
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
        let point = vport.transform().inverse() * point;
        DPoint::new(point.x.round(), point.y.round())
    }

    pub fn to_screen(self, vport: ViewPort) -> Point {
        vport.transform() * self.to_raw()
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

    fn from_raw(vec2: impl Into<Vec2>) -> DVec2 {
        let vec2 = vec2.into();
        DVec2::new(vec2.x.round(), vec2.y.round())
    }

    /// should not be public, used internally so we can reuse math ops
    #[inline]
    fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }

    #[inline]
    pub fn to_screen(self, vport: ViewPort) -> Vec2 {
        self.to_vec2() * vport.zoom
    }

    pub fn hypot(self) -> f64 {
        self.to_vec2().hypot()
    }

    pub fn normalize(self) -> DVec2 {
        DVec2::from_raw(self.to_vec2().normalize())
    }
}

impl ViewPort {
    fn scroll(&mut self, event: &WheelEvent) {
        let mut delta = event.delta;
        if event.mods.shift {
            if delta.x > delta.y {
                delta.y = 0.;
            } else {
                delta.x = 0.;
            }
        }
        let x = (self.offset.x - event.delta.x)
            .min(MAX_SCROLL)
            .max(MIN_SCROLL);
        let y = (self.offset.y - event.delta.y)
            .min(MAX_SCROLL)
            .max(MIN_SCROLL);
        self.offset = Vec2::new(x.round(), y.round());
    }

    fn zoom(&mut self, event: &WheelEvent, mouse: Point) {
        let delta = if event.delta.x.abs() > event.delta.y.abs() {
            event.delta.x
        } else {
            event.delta.y
        };
        // the deltas we get are big, and make zooming jumpy
        let delta = delta.round() * 0.02;
        if delta == 0. {
            return;
        }

        // We want the mouse to stay fixed in design space _and_ in screen space.
        // 1. get the pre-zoom design space location of the mouse
        let pre_mouse = self.transform().inverse() * mouse;
        let next_zoom = (self.zoom + delta).max(MIN_ZOOM).min(MAX_ZOOM);
        if (next_zoom - self.zoom).abs() < 0.001 {
            // don't jump around near our boundaries.
            return;
        }
        self.zoom = next_zoom;
        // 2. get the post-zoom screen-space location of pre_mouse
        let post_mouse = self.transform() * pre_mouse;
        let mouse_delta = mouse - post_mouse;

        //eprintln!("{:.4}: ({:.2}, {:.2}), ({:.2}, {:.2}), ({:.2}, {:.2})", self.zoom, pre_mouse.x, pre_mouse.y, post_mouse.x, post_mouse.y, mouse_delta.x, mouse_delta.y);
        self.offset += mouse_delta;
    }

    pub fn pan(&mut self, delta: Vec2) {
        self.offset += delta
    }

    pub fn transform(&self) -> TranslateScale {
        TranslateScale::new(self.offset, self.zoom)
    }

    fn design_point(&self, point: impl Into<Point>) -> DPoint {
        DPoint::from_screen(point.into(), *self)
    }

    pub fn to_screen(&self, point: impl Into<Point>) -> Point {
        DPoint::from_raw(point.into()).to_screen(*self)
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
        DVec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for DVec2 {
    type Output = DVec2;

    #[inline]
    fn sub(self, other: DVec2) -> DVec2 {
        DVec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Mul<f64> for DVec2 {
    type Output = DVec2;

    #[inline]
    fn mul(self, other: f64) -> DVec2 {
        DVec2 {
            x: self.x * other,
            y: self.y * other,
        }
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
        }
    }
}
