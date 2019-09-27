//! The top-level widget for the main glyph list window.

use std::rc::Rc;

use druid::kurbo::{Affine, BezPath, Point, Rect, Shape, Size};
use druid::piet::{Color, RenderContext};
use druid::{
    BaseState, BoxConstraints, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx, Widget,
    WidgetPod,
};

//use norad::Ufo;
use norad::glyph::{Contour, ContourPoint, Glyph, PointType};

use crate::data::{lenses, AppState};

pub struct GlyphGrid {
    children: Vec<WidgetPod<AppState, GridItem>>,
}

//const TOTAL_HEIGHT: f64 = 2000.;
const GLYPH_SIZE: f64 = 100.;
const GLYPH_COLOR: Color = Color::rgb8(0x6a, 0x6a, 0x6a);
const HIGHLIGHT_COLOR: Color = Color::rgb8(0xfa, 0xfa, 0xfa);

impl Widget<AppState> for GlyphGrid {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &AppState, env: &Env) {
        ctx.render_ctx.clear(Color::WHITE);
        for child in &mut self.children {
            child.paint_with_offset(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &AppState,
        env: &Env,
    ) -> Size {
        let width = (bc.max().width / GLYPH_SIZE).floor() * GLYPH_SIZE;
        eprintln!("width {}", width);
        let mut x: f64 = 0.;
        let mut y: f64 = 0.;

        let child_bc = BoxConstraints::tight(Size::new(GLYPH_SIZE, GLYPH_SIZE));

        for child in &mut self.children {
            if x > 0. && x + GLYPH_SIZE > width {
                y += GLYPH_SIZE;
                x = 0.;
            }
            child.layout(ctx, &child_bc, data, env);
            child.set_layout_rect(Rect::from_origin_size((x, y), (GLYPH_SIZE, GLYPH_SIZE)));
            x += GLYPH_SIZE;
        }
        Size::new(width, y + GLYPH_SIZE)
    }

    fn event(&mut self, _event: &Event, _ctx: &mut EventCtx, _data: &mut AppState, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, _old: Option<&AppState>, new: &AppState, _env: &Env) {
        if let Some(layer) = new.file.as_ref().and_then(|f| f.object.get_default_layer()) {
            if layer.inner.borrow().contents.len() != self.children.len() {
                self.children.clear();
                for key in layer.inner.borrow().contents.keys() {
                    self.children.push(WidgetPod::new(GridItem::new(key)));
                }
            }
            ctx.invalidate();
        }
    }
}

impl GlyphGrid {
    pub fn new() -> GlyphGrid {
        GlyphGrid {
            children: Vec::new(),
        }
    }
}

struct GridItem {
    name: String,
    inner: GridInner,
}

struct GridInner;

impl GridItem {
    fn new(name: impl Into<String>) -> Self {
        GridItem {
            name: name.into(),
            inner: GridInner,
        }
    }

    fn get_glyph(&self, data: &AppState) -> Option<Rc<Glyph>> {
        data.file
            .as_ref()?
            .object
            .get_default_layer()?
            .get_glyph(&self.name)
            .ok()
    }
}

impl Widget<AppState> for GridItem {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, d: &AppState, env: &Env) {
        let glyph = self.get_glyph(d).expect("missing glyph");
        self.inner.paint(ctx, state, &glyph, env)
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        d: &AppState,
        env: &Env,
    ) -> Size {
        let glyph = self.get_glyph(d).expect("missing glyph");
        self.inner.layout(ctx, bc, &glyph, env)
    }

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut AppState, env: &Env) {
        let mut glyph = self.get_glyph(data).expect("missing glyph");
        self.inner.event(event, ctx, &mut glyph, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old: Option<&AppState>, new: &AppState, env: &Env) {
        let old = old.map(|old| self.get_glyph(old).expect("missing glyph"));
        let new = self.get_glyph(new).expect("missing glyph");
        self.inner.update(ctx, old.as_ref(), &new, env)
    }
}

impl Widget<Rc<Glyph>> for GridInner {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &Rc<Glyph>, _env: &Env) {
        let path = path_for_glyph(data);
        let bb = path.bounding_box();
        let geom = Rect::ZERO.with_size(state.size());
        let scale = geom.height() as f64 / 1000.;
        let scale = scale * 0.85; // some margins around glyphs
        let scaled_width = bb.width() * scale as f64;
        let l_pad = ((geom.width() as f64 - scaled_width) / 2.).round();
        let baseline = (geom.height() * 0.16) as f64;
        let affine = Affine::new([
            scale as f64,
            0.0,
            0.0,
            -scale as f64,
            l_pad,
            geom.height() - baseline,
        ]);

        let glyph_body_color = if state.is_active() {
            HIGHLIGHT_COLOR
        } else {
            GLYPH_COLOR
        };
        //let fill = ctx.render_ctx.solid_brush(glyph_body_color).unwrap();
        ctx.render_ctx.fill(affine * &path, &glyph_body_color);
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _d: &Rc<Glyph>,
        _env: &Env,
    ) -> Size {
        bc.max()
    }

    fn event(&mut self, _event: &Event, _ctx: &mut EventCtx, _data: &mut Rc<Glyph>, _env: &Env) {}

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old: Option<&Rc<Glyph>>,
        _new: &Rc<Glyph>,
        _env: &Env,
    ) {
        ctx.invalidate();
    }
}

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
