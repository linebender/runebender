//! A collection of children laid out in a grid.

use druid::widget::Widget;
use druid::{BoxConstraints, LayoutResult, Id, LayoutCtx, Ui};

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
