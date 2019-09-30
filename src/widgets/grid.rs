//! The top-level widget for the main glyph list window.

use std::rc::Rc;

use druid::kurbo::{Affine, BezPath, Point, Rect, Shape, Size};
use druid::piet::{
    Color, FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout,
    TextLayoutBuilder,
};
use druid::{
    theme, BaseState, BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, PaintCtx, UpdateCtx,
    Widget, WidgetPod,
};

use norad::glyph::{Contour, ContourPoint, Glyph, PointType};
use norad::Ufo;

use crate::data::lenses;
use crate::lens2::Lens2Wrap;

pub struct GlyphGrid {
    children: Vec<WidgetPod<Rc<Ufo>, Lens2Wrap<Rc<Glyph>, lenses::app_state::Glyph, GridInner>>>,
}

const GLYPH_SIZE: f64 = 100.;
const TEXT_BG_COLOR: Color = Color::rgba8(0xd7, 0xd8, 0xd2, 0xad);
const GLYPH_COLOR: Color = Color::rgb8(0x6a, 0x6a, 0x6a);
const HIGHLIGHT_COLOR: Color = Color::rgb8(0x04, 0x3b, 0xaf);

impl Widget<Rc<Ufo>> for GlyphGrid {
    fn paint(&mut self, ctx: &mut PaintCtx, _state: &BaseState, data: &Rc<Ufo>, env: &Env) {
        ctx.render_ctx.clear(Color::WHITE);
        for child in &mut self.children {
            child.paint_with_offset(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Rc<Ufo>,
        env: &Env,
    ) -> Size {
        let width = (bc.max().width / GLYPH_SIZE).floor() * GLYPH_SIZE;
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

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, data: &mut Rc<Ufo>, env: &Env) {
        for child in &mut self.children {
            child.event(event, ctx, data, env)
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old: Option<&Rc<Ufo>>, new: &Rc<Ufo>, _env: &Env) {
        if new.glyph_count() != self.children.len() {
            let units_per_em = new
                .font_info
                .as_ref()
                .and_then(|info| info.units_per_em.clone())
                .unwrap_or(1000.);
            let widget = GridInner { units_per_em };
            self.children.clear();
            for key in new.iter_names() {
                self.children.push(WidgetPod::new(Lens2Wrap::new(
                    widget,
                    lenses::app_state::Glyph(key),
                )));
            }
        }
        ctx.invalidate();
    }
}

impl GlyphGrid {
    pub fn new() -> GlyphGrid {
        GlyphGrid {
            children: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct GridInner {
    units_per_em: f64,
}

impl Widget<Rc<Glyph>> for GridInner {
    fn paint(&mut self, ctx: &mut PaintCtx, state: &BaseState, data: &Rc<Glyph>, env: &Env) {
        let path = path_for_glyph(data);
        let bb = path.bounding_box();
        let geom = Rect::ZERO.with_size(state.size());
        let scale = geom.height() as f64 / self.units_per_em;
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
        ctx.render_ctx.fill(affine * &path, &glyph_body_color);

        if state.is_hot() {
            ctx.render_ctx.stroke(affine * &path, &HIGHLIGHT_COLOR, 1.0);
            ctx.render_ctx.stroke(geom, &HIGHLIGHT_COLOR, 1.0);
        }

        let font_size = env.get(theme::TEXT_SIZE_NORMAL);
        let name_color = if state.is_hot() {
            HIGHLIGHT_COLOR
        } else {
            GLYPH_COLOR
        };
        let text = get_text_layout(&mut ctx.text(), data.name.as_str(), env);
        let xpos = geom.x0 + (geom.width() - text.width()) * 0.5;
        let ypos = geom.y0 + geom.height() - font_size * 0.25;
        let pos = (xpos, ypos);

        //draw a semi-translucent background
        let text_bg_rect = Rect::from_origin_size(
            (pos.0 as f64, (pos.1 - font_size * 0.75) as f64),
            (text.width() as f64, font_size as f64),
        );

        ctx.render_ctx.fill(&text_bg_rect, &TEXT_BG_COLOR);
        // draw the text
        ctx.render_ctx.draw_text(&text, pos, &name_color)
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

    fn event(&mut self, event: &Event, ctx: &mut EventCtx, _data: &mut Rc<Glyph>, _env: &Env) {
        match event {
            Event::MouseDown(_) => {
                ctx.set_active(true);
                ctx.invalidate();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.invalidate();
                    if ctx.is_hot() {
                        log::info!("grid item mouse up");
                    }
                }
            }
            Event::HotChanged(_) => {
                ctx.invalidate();
            }
            _ => (),
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old: Option<&Rc<Glyph>>,
        new: &Rc<Glyph>,
        _env: &Env,
    ) {
        if old.map(|old| !old.same(new)).unwrap_or(true) {
            ctx.invalidate();
        }
    }
}

fn get_text_layout(text_ctx: &mut PietText, text: &str, env: &Env) -> PietTextLayout {
    let font_name = env.get(theme::FONT_NAME);
    let font_size = env.get(theme::TEXT_SIZE_NORMAL);
    // TODO: caching of both the format and the layout
    let font = text_ctx
        .new_font_by_name(font_name, font_size)
        .unwrap()
        .build()
        .unwrap();
    text_ctx
        .new_text_layout(&font, text)
        .unwrap()
        .build()
        .unwrap()
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
