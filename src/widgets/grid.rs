use kurbo::Rect;
use piet::{FillRule, RenderContext};

use druid::widget::Widget;
use druid::{BoxConstraints, Geometry, LayoutResult, HandlerCtx, Id, LayoutCtx, MouseEvent, PaintCtx, Ui};

//TODO: this should be part of the main lib
#[derive(Debug, Default)]
struct Size { width: f32, height: f32 }
#[derive(Debug, Default)]
struct Point { x: f32, y: f32 }


#[derive(Debug, Default)]
struct LayoutState {
    ix: usize,
    size: Size,
    row_count: usize,
    next_orig: Point,
}

pub struct Grid {
    item_size: Size,
    padding: f32,
    layout: LayoutState,
}

impl Grid {
    pub fn new(item_size: (f32, f32)) -> Self {
        Grid {
            item_size: Size { width: item_size.0, height: item_size.1, },
            padding: 8.0,
            layout: LayoutState::default(),
        }
    }

    /// Add to UI with children.
    pub fn ui(self, children: &[Id], ctx: &mut Ui) -> Id {
        ctx.add(self, children)
    }
}

impl Widget for Grid {
    fn layout(
        &mut self,
        bc: &BoxConstraints,
        children: &[Id],
        size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        // no size means this is the first call of a new layout pass
        if size.is_none() {
            self.layout = LayoutState::default();
            if children.is_empty() {
                return LayoutResult::Size((bc.min_width, bc.min_height));
            }
        }

        let item_w = self.item_size.width;
        let item_h = self.item_size.height;
        let items_per_row = (bc.max_width / (self.item_size.width + self.padding)).floor();
        let padding = (bc.max_width - ((self.item_size.width + self.padding) * items_per_row)) / items_per_row;
        let padding = padding.max(self.padding);

        if self.layout.ix >= children.len() {
            return LayoutResult::Size((self.layout.size.width + padding, self.layout.size.height));
        }

        let next_child = children[self.layout.ix];
        self.layout.ix += 1;

        //for child in _children {
        if self.layout.row_count > 0 && self.layout.next_orig.x + item_w + self.padding > bc.max_width {
            self.layout.row_count = 0;
            self.layout.next_orig.x = 0.;
            self.layout.next_orig.y += self.padding + item_h;
        }
        ctx.position_child(next_child, (self.layout.next_orig.x + padding, self.layout.next_orig.y));
        self.layout.size.width = self.layout.size.width.max(self.layout.next_orig.x + item_w + padding);
        self.layout.size.height = self.layout.size.height.max(self.layout.next_orig.y + item_h);
        self.layout.next_orig.x += padding + item_w;
        self.layout.row_count += 1;
        LayoutResult::RequestChild(next_child, BoxConstraints::tight((self.item_size.width, self.item_size.height)))
    }
}

pub struct Clickable {}

impl Clickable {
    pub fn new() -> Self {
        Clickable {}
    }

    /// Add to UI with children.
    pub fn ui<ID: Into<Option<Id>>>(self, child: ID, ctx: &mut Ui) -> Id {
        if let Some(child) = child.into() {
            ctx.add(self, &[child])
        } else {
            ctx.add(self, &[])
        }
    }
}

impl Widget for Clickable {
    fn mouse(&mut self, event: &MouseEvent, ctx: &mut HandlerCtx) -> bool {
        //eprintln!("event {:?} node: {}", &event, ctx.id);
        if event.count > 0 {
            ctx.set_active(true);
        } else {
            ctx.set_active(false);
            if ctx.is_hot() {
                ctx.send_event(true);
            }
        }
        ctx.invalidate();
        true
    }

    fn layout(
        &mut self,
        bc: &BoxConstraints,
        children: &[Id],
        size: Option<(f32, f32)>,
        ctx: &mut LayoutCtx,
    ) -> LayoutResult {
        if let Some(size) = size {
            // Maybe this is not necessary, rely on default value.
            ctx.position_child(children[0], (0.0, 0.0));
            LayoutResult::Size(size)
        } else if children.is_empty() {
            LayoutResult::Size((bc.max_width, bc.max_height))
        } else {
            LayoutResult::RequestChild(children[0], *bc)
        }
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, geom: &Geometry) {
        let is_active = paint_ctx.is_active();
        let is_hot = paint_ctx.is_hot();
        let bg_color = match (is_active, is_hot) {
            (true, true) => 0x60f068ff,
            (false, true) => 0x5050f8ff,
            _ => 0x000048ff,
        };
        let brush = paint_ctx.render_ctx.solid_brush(bg_color).unwrap();
        let (x, y) = geom.pos;
        let (width, height) = geom.size;
        let rect = Rect::new(
            x as f64,
            y as f64,
            x as f64 + width as f64,
            y as f64 + height as f64,
            );
        paint_ctx.render_ctx.fill(rect, &brush, FillRule::NonZero);
    }

    fn on_hot_changed(&mut self, _hot: bool, ctx: &mut HandlerCtx) {
        ctx.invalidate();
    }
}
