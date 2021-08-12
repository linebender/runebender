use druid::kurbo::{Line, ParamCurve, ParamCurveNearest, Point, Vec2};
use druid::Data;

use crate::design_space::{DPoint, DVec2, ViewPort};
use crate::point::EntityId;

#[derive(Debug, Clone, Data)]
pub struct Guide {
    pub id: EntityId,
    pub guide: GuideLine,
}

/// A guideline.
#[derive(Debug, Clone, Data)]
pub enum GuideLine {
    Horiz(DPoint),
    Vertical(DPoint),
    Angle { p1: DPoint, p2: DPoint },
}

impl Guide {
    fn new(guide: GuideLine) -> Self {
        let id = EntityId::new_for_guide();
        Guide { id, guide }
    }

    pub fn horiz(p1: DPoint) -> Self {
        Guide::new(GuideLine::Horiz(p1))
    }

    pub fn vertical(p1: DPoint) -> Self {
        Guide::new(GuideLine::Vertical(p1))
    }

    pub fn angle(p1: DPoint, p2: DPoint) -> Self {
        Guide::new(GuideLine::Angle { p1, p2 })
    }

    pub fn toggle_vertical_horiz(&mut self, new_point: DPoint) {
        let new = match self.guide {
            GuideLine::Horiz(_) => GuideLine::Vertical(new_point),
            GuideLine::Vertical(_) => GuideLine::Horiz(new_point),
            GuideLine::Angle { p1, p2 } => GuideLine::Angle { p1, p2 },
        };
        self.guide = new;
    }

    pub fn screen_dist(&self, vport: ViewPort, point: Point) -> f64 {
        self.nearest_screen_point(vport, point).distance(point)
    }

    pub(crate) fn nearest_screen_point(&self, vport: ViewPort, point: Point) -> Point {
        match self.guide {
            GuideLine::Horiz(p) => {
                let Point { y, .. } = p.to_screen(vport);
                Point::new(point.x, y)
            }
            GuideLine::Vertical(p) => {
                let Point { x, .. } = p.to_screen(vport);
                Point::new(x, point.y)
            }
            GuideLine::Angle { p1, p2 } => {
                //FIXME: this line is not infinite, which it should be.
                let p1 = p1.to_screen(vport);
                let p2 = p2.to_screen(vport);
                let vec = (p2 - p1).normalize();
                let p1 = p2 - vec * 5000.; // an arbitrary number
                let p2 = p2 + vec * 5000.;
                let line = Line::new(p1, p2);
                let (t, _) = line.nearest(point, 0.1);
                line.eval(t)
            }
        }
    }

    pub fn nudge(&mut self, nudge: DVec2) {
        match self.guide {
            GuideLine::Horiz(ref mut p) => p.y += nudge.y,
            GuideLine::Vertical(ref mut p) => p.x += nudge.x,
            GuideLine::Angle {
                ref mut p1,
                ref mut p2,
            } => {
                p1.x += nudge.x;
                p2.x += nudge.x;
                p1.y += nudge.y;
                p2.y += nudge.y;
            }
        }
    }

    pub fn from_norad(src: &norad::Guideline) -> Self {
        use norad::Line;

        let guide = match src.line {
            Line::Vertical(x) => GuideLine::Vertical(DPoint::new(x as f64, 0.)),
            Line::Horizontal(y) => GuideLine::Horiz(DPoint::new(0., y as f64)),
            Line::Angle { x, y, degrees } => {
                let p1 = DPoint::new(x as f64, y as f64);
                let p2 = p1.to_raw() + Vec2::from_angle(degrees as f64);
                let p2 = DPoint::new(p2.x, p2.y);
                GuideLine::Angle { p1, p2 }
            }
        };

        let id = EntityId::new_for_guide();
        Guide { id, guide }
    }

    pub fn to_norad(&self) -> norad::Guideline {
        let line = match self.guide {
            GuideLine::Horiz(p) => norad::Line::Horizontal(p.y as f32),
            GuideLine::Vertical(p) => norad::Line::Vertical(p.x as f32),
            GuideLine::Angle { p1, p2 } => {
                let x = p1.x as f32;
                let y = p1.y as f32;
                let angle = p2 - p1;
                let degrees = angle.to_raw().atan2() as f32;
                norad::Line::Angle { x, y, degrees }
            }
        };

        norad::Guideline::new(line, None, None, None, None)
    }
}
