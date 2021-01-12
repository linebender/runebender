use crate::design_space::DVec2;
use crate::util;
use druid::{Data, Point, Rect, Size, Vec2};

/// Divisions of a 2D plane.
///
/// These coorespond to nine anchor points, and are used for things like
/// calculating the position of selection handles, as well as in the coordinate
/// panel.
#[derive(Debug, Clone, Copy, PartialEq, Data)]
pub enum Quadrant {
    Center,
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

static ALL_QUADRANTS: &[Quadrant] = &[
    Quadrant::TopLeft,
    Quadrant::Top,
    Quadrant::TopRight,
    Quadrant::Left,
    Quadrant::Center,
    Quadrant::Right,
    Quadrant::BottomLeft,
    Quadrant::Bottom,
    Quadrant::BottomRight,
];

impl Quadrant {
    /// Return all `Quadrant`s, suitable for iterating.
    pub fn all() -> &'static [Quadrant] {
        ALL_QUADRANTS
    }

    /// Return the position opposite this one; TopRight to BottomLeft, e.g.
    ///
    /// This is used when dragging a selection handle; you anchor the point
    /// opposite the selected handle.
    pub fn inverse(self) -> Quadrant {
        self.invert_y().invert_x()
    }

    fn invert_y(self) -> Quadrant {
        match self {
            Quadrant::TopRight => Quadrant::BottomRight,
            Quadrant::TopLeft => Quadrant::BottomLeft,
            Quadrant::Top => Quadrant::Bottom,
            Quadrant::BottomRight => Quadrant::TopRight,
            Quadrant::BottomLeft => Quadrant::TopLeft,
            Quadrant::Bottom => Quadrant::Top,
            other => other,
        }
    }

    fn invert_x(self) -> Quadrant {
        match self {
            Quadrant::TopRight => Quadrant::TopLeft,
            Quadrant::TopLeft => Quadrant::TopRight,
            Quadrant::Left => Quadrant::Right,
            Quadrant::Right => Quadrant::Left,
            Quadrant::BottomRight => Quadrant::BottomLeft,
            Quadrant::BottomLeft => Quadrant::BottomRight,
            other => other,
        }
    }

    pub(crate) fn modifies_x_axis(self) -> bool {
        !matches!(self, Quadrant::Top | Quadrant::Bottom | Quadrant::Center)
    }

    pub(crate) fn modifies_y_axis(self) -> bool {
        !matches!(self, Quadrant::Left | Quadrant::Right | Quadrant::Center)
    }

    /// given a point and a size, return the quadrant containing that point.
    pub fn for_point_in_bounds(pt: Point, size: Size) -> Self {
        let zone_x = size.width / 3.0;
        let zone_y = size.height / 3.0;
        let mouse_x = match pt.x {
            x if x < zone_x => 0,
            x if x >= zone_x && x < zone_x * 2.0 => 1,
            x if x >= zone_x * 2.0 => 2,
            _ => unreachable!(),
        };

        let mouse_y = match pt.y {
            y if y < zone_y => 0,
            y if y >= zone_y && y < zone_y * 2.0 => 1,
            y if y >= zone_y * 2.0 => 2,
            _ => unreachable!(),
        };

        match (mouse_x, mouse_y) {
            (0, 0) => Quadrant::TopLeft,
            (1, 0) => Quadrant::Top,
            (2, 0) => Quadrant::TopRight,
            (0, 1) => Quadrant::Left,
            (1, 1) => Quadrant::Center,
            (2, 1) => Quadrant::Right,
            (0, 2) => Quadrant::BottomLeft,
            (1, 2) => Quadrant::Bottom,
            (2, 2) => Quadrant::BottomRight,
            _ => unreachable!(),
        }
    }

    /// Given a bounds, return the point corresponding to this quadrant.
    pub fn point_in_rect(self, bounds: Rect) -> Point {
        let size = bounds.size();
        let origin_offset = bounds.origin().to_vec2();
        let rel_point = match self {
            Quadrant::TopLeft => Point::new(0., 0.),
            Quadrant::Top => Point::new(size.width / 2.0, 0.),
            Quadrant::TopRight => Point::new(size.width, 0.),
            Quadrant::Left => Point::new(0., size.height / 2.0),
            Quadrant::Center => Point::new(size.width / 2.0, size.height / 2.0),
            Quadrant::Right => Point::new(size.width, size.height / 2.0),
            Quadrant::BottomLeft => Point::new(0.0, size.height),
            Quadrant::Bottom => Point::new(size.width / 2.0, size.height),
            Quadrant::BottomRight => Point::new(size.width, size.height),
        };
        rel_point + origin_offset
    }

    /// Given a rect in *design space* (that is, y-up), return the point
    /// corresponding to this quadrant.
    pub(crate) fn point_in_dspace_rect(self, rect: Rect) -> Point {
        self.invert_y().point_in_rect(rect)
    }

    /// Return the x&y suitable for transforming `rect` given a drag
    /// originating at this quadrant.
    ///
    /// This can be negative in either direction if the drag crosses the
    /// opposite quadrant point.
    pub(crate) fn scale_dspace_rect(self, rect: Rect, drag: DVec2) -> Vec2 {
        // axis locking should have already happened
        assert_eq!(drag, self.lock_delta(drag));
        let start_point = self.point_in_dspace_rect(rect);
        let origin_point = self.inverse().point_in_dspace_rect(rect);
        let origin_delta = origin_point - start_point;
        let cur_delta = origin_point - (start_point + drag.to_raw());
        util::compute_scale(origin_delta.to_size(), cur_delta.to_size())
    }

    /// When dragging from a control handle, side handles lock an axis.
    pub(crate) fn lock_delta(self, delta: DVec2) -> DVec2 {
        match self {
            Quadrant::Top | Quadrant::Bottom => delta.zero_x(),
            Quadrant::Left | Quadrant::Right => delta.zero_y(),
            _ => delta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadrant_pos() {
        let rect = Rect::new(10.0, 10., 100., 100.);
        assert_eq!(
            Quadrant::BottomLeft.point_in_dspace_rect(rect),
            rect.origin()
        );
        assert_eq!(
            Quadrant::Center.point_in_dspace_rect(rect),
            Point::new(55.0, 55.0)
        );
        assert_eq!(
            Quadrant::TopRight.point_in_dspace_rect(rect),
            Point::new(100.0, 100.0)
        );
        assert_eq!(
            Quadrant::Top.point_in_dspace_rect(rect),
            Point::new(55.0, 100.0)
        );
    }
}
