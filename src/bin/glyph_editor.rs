// Copyright 2019 the Runebender authors.

//! A quick demonstration of loading and displaying a UFO glyph.

use kurbo::{Affine, BezPath, Circle, Line, Vec2};
use norad::glyph::{Contour, ContourPoint, Glyph, PointType};
use piet::{FillRule, RenderContext};

use druid_shell::platform::WindowBuilder;
use druid_shell::win_main;
//use druid_shell::window::MouseButton;

use druid::{
    BoxConstraints, Geometry, HandlerCtx, Id, LayoutCtx, LayoutResult, MouseEvent, PaintCtx,
    Ui, UiMain, UiState, Widget,
};

use druid::widget::MouseButton;

#[path="../widgets/grid.rs"]
mod grid;

#[path="../widgets/glyph.rs"]
mod glyph_widget;
//HACK: currently we just use the point's "overall index" as an id.
type PointId = usize;

const BASELINE_COLOR: u32 =  0x00_80_f0_ff;
const OUTLINE_COLOR: u32 =  0xfa_fa_fa_ff;
const POINT_COLOR_NORMAL: u32 =  0xf0_f0_ea_ff;
const POINT_COLOR_CONTROL: u32 =  0x70_80_7a_ff;
const POINT_COLOR_HOVER: u32 =  0xf0_80_7a_ff;
const POINT_COLOR_DRAG: u32 =  0xff_40_3a_ff;


struct GlyphEditor {
    glyph: Glyph,
    path: BezPath,
    height: f32,
    controls: Vec<(Circle, PointId)>,
    mouse: MouseState,
    /// for mapping a point in the widget to a point in the glyph
    // TODO: how do I get the inverse of an affine?
    translate_fn: Box<dyn Fn(Vec2) -> Vec2>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum MouseState {
    Normal,
    Hover(PointId),
    Drag { point: PointId, start: Vec2, current: Vec2 },
}

impl GlyphEditor {
    fn new(glyph: Glyph) -> Self {
        // assume glyph height is 1000 or 4000 'units'
        // TODO: get the actual height from the UFO file
        let height = if glyph.outline.as_ref()
            .map(|o| o.contours.iter()
                 .flat_map(|c| c.points.iter().map(|p| p.y))
                 .any(|h| h > 1000.))
            .unwrap_or(false) { 4000. } else { 1000. };

        let path = glyph_widget::path_for_glyph(&glyph);
        GlyphEditor {
            glyph,
            height,
            path,
            controls: Vec::new(),
            mouse: MouseState::Normal,
            translate_fn: Box::new(|pt| pt),
        }
    }


    fn update_point(&mut self, point: PointId, new_pos: Vec2) {
        let glyph_point = ( self.translate_fn )(new_pos);
        if let Some(p) = self.glyph.outline.as_mut().iter_mut().flat_map(|o| o.contours.iter_mut()).flat_map(|c| c.points.iter_mut()).nth(point) {
            p.x = glyph_point.x as f32;
            p.y = glyph_point.y as f32;
        }

        //self.path = self.glyph.outline.as_ref().map(|o| (&o.contours)).unwrap_or_default();
        self.path = glyph_widget::path_for_glyph(&self.glyph);
    }

    fn ui(self, ctx: &mut Ui) -> Id {
        ctx.add(self, &[])
    }
}

impl Widget for GlyphEditor {
    fn paint(&mut self, ctx: &mut PaintCtx, geom: &Geometry) {
        let baseline = (geom.size.1 * 0.66) as f64;
        let l_pad = 100.;

        let baseline_clr = ctx.render_ctx.solid_brush(BASELINE_COLOR).unwrap();
        let outline_clr = ctx.render_ctx.solid_brush(OUTLINE_COLOR).unwrap();
        let point_clr = ctx.render_ctx.solid_brush(POINT_COLOR_NORMAL).unwrap();
        let control_point_clr = ctx.render_ctx.solid_brush(POINT_COLOR_CONTROL).unwrap();
        let hover_point_clr = ctx.render_ctx.solid_brush(POINT_COLOR_HOVER).unwrap();
        let drag_point_clr = ctx.render_ctx.solid_brush(POINT_COLOR_DRAG).unwrap();

        let scale = (geom.size.1 / self.height * 0.65).min(1.0).max(0.2);
        println!("scale {}", scale);
        let affine = Affine::new([scale as f64, 0.0, 0.0, -scale as f64, l_pad, baseline]);

        let line = Line::new((0., baseline), (geom.size.0 as f64, baseline));
        ctx.render_ctx.stroke(line, &baseline_clr, 0.5, None);
        ctx.render_ctx.stroke(affine * &self.path, &outline_clr, 1.0, None);

        // stash a fn to get our glyph points from our visual points
        // TODO: this would be nice if we could just stash the affine and compute the inverse
        self.translate_fn = Box::new(move |pt| {
            let x = (pt.x  - l_pad) / scale as f64;
            let y = (pt.y - baseline) / -scale as f64;
            (x, y).into()
        });

        self.controls.clear();
        let mut id = 0;

        let mut control_guides = BezPath::new();

        for shape in self.glyph.outline.as_ref().iter().map(|o| o.contours.iter()).flatten() {
            if shape.points.is_empty() { continue };
            let last = shape.points.last().unwrap();
            // the last seen control point, and it whether or not it was 'on curve' or not.
            // because we wrap around, we can start at the end
            let mut last_point = (Vec2::new(last.x as f64, last.y as f64), last.typ == PointType::OffCurve);

            for point in shape.points.iter() {
                println!("{:?}", point);
                let is_control = point.typ == PointType::OffCurve;
                let point: Vec2 = (point.x as f64, point.y as f64).into();

                if last_point.1 != is_control {
                    control_guides.moveto(last_point.0);
                    control_guides.lineto(point);
                }

                last_point = (point, is_control);

                let color = match (self.mouse, is_control) {
                    (MouseState::Drag { point, .. }, _) if point == id => &drag_point_clr,
                    (MouseState::Hover(point), _) if point == id => &hover_point_clr,
                    (_, true)  => &control_point_clr,
                    (_, false) => &point_clr,
                };

                let point = affine * point;
                let rad = (10.0 * scale as f64).min(8.0).max(4.0);
                let circ = Circle::new(point, rad);
                self.controls.push((circ, id));

                ctx.render_ctx.fill(circ, color, FillRule::NonZero);
                id += 1;
            }
        }
        ctx.render_ctx.stroke(affine * control_guides, &control_point_clr, 1.0, None);
    }

    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        eprintln!("{:?}{}: ({}, {})", event.which, event.count, event.x, event.y);
        const MIN_DRAG_DISTANCE: f64 = 5.0;
        let v2 = (event.x as f64, event.y as f64).into();
        let new_state = match self.mouse {
            MouseState::Normal | MouseState::Hover(_) if event.which == MouseButton::Left && event.count == 1 => {
                if let Some((circ, point)) = self.controls.iter().find(|(c, _)| is_inside(*c, v2)) {
                    eprintln!("dragging point {}", point);
                    MouseState::Drag { point: *point, start: circ.center, current: v2 }
                } else {
                    MouseState::Normal
                }
            }
            MouseState::Drag { point, start, current } if event.count == 0 => {
                let success = distance(start, current) >= MIN_DRAG_DISTANCE;
                if success {
                    self.update_point(point, current);
                    MouseState::Hover(point)
                } else {
                    self.update_point(point, start);
                    MouseState::Normal
                }
            }
            _ => MouseState::Normal,
        };

        if new_state != self.mouse {
            ctx.invalidate();
        }

        self.mouse = new_state;
        true
    }

    fn mouse_moved(&mut self, x: f32, y: f32, ctx: &mut HandlerCtx) {
        let v2 = (x as f64, y as f64).into();
        let new_state = match self.mouse {
            MouseState::Drag { point, start, .. } => {
                self.update_point(point, v2);
                MouseState::Drag { point, start, current: v2 }
            }
            _ => match self.controls.iter().find(|(c, _)| is_inside(*c, v2)).map(|(_, id)| id) {
                Some(id) => MouseState::Hover(*id),
                None => MouseState::Normal,
            }
        };

        if new_state != self.mouse {
            ctx.invalidate();
        }

        self.mouse = new_state;
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        _children: &[Id],
        _size: Option<(f32, f32)>,
        _ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        LayoutResult::Size(bc.constrain((100.0, 100.0)))
    }
}

fn build_ui(ui: &mut UiState, glyph: Glyph) {
    let root_id = GlyphEditor::new(glyph).ui(ui);
    ui.set_root(root_id);
}

#[inline]
fn is_inside(circle: Circle, point: Vec2) -> bool {
    distance(point, circle.center) <= circle.radius
}

#[inline]
fn distance(p1: Vec2, p2: Vec2) -> f64 {
    ((p1.x - p2.x).powi(2) + (p1.y - p2.y).powi(2)).sqrt()
}

fn main() {
    let glyph_path = match std::env::args().skip(1).next() {
        Some(arg) => arg,
        None => {
            eprintln!("Please pass a path to a .glif file");
            std::process::exit(1);
        }
    };

    println!("loading {}", glyph_path);
    let glyph = norad::Glyph::load(&glyph_path).expect("failed to load glyph");

    druid_shell::init();

    let mut run_loop = win_main::RunLoop::new();
    let mut builder = WindowBuilder::new();
    let mut state = UiState::new();

    build_ui(&mut state, glyph);

    builder.set_handler(Box::new(UiMain::new(state)));
    builder.set_title("Ufo Toy");
    let window = builder.build().expect("building window");

    window.show();
    run_loop.run();
}

