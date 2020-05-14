//! The toolbar widget

use druid::kurbo::{Affine, BezPath, Circle, Line, Shape, Vec2};
use druid::widget::prelude::*;
use druid::widget::{Painter, WidgetExt};
use druid::{Color, Data, Rect, WidgetPod};

use crate::consts;
use crate::tools::{Pen, Preview, Select, Tool};

//type Action<T> = Box<dyn Fn(&mut EventCtx, &mut T, &Env)>;

const TOOLBAR_ITEM_SIZE: Size = Size::new(40.0, 40.0);
const TOOLBAR_ITEM_PADDING: f64 = 2.0;
const TOOLBAR_ICON_PADDING: f64 = 5.0;
const TOOLBAR_BORDER_STROKE_WIDTH: f64 = 2.0;
const TOOLBAR_ITEM_STROKE_WIDTH: f64 = 1.5;
// TODO: move these to theme
const TOOLBAR_BG_DEFAULT: Color = Color::grey8(0xDD);
const TOOLBAR_BG_SELECTED: Color = Color::grey8(0xAD);

struct ToolbarItem {
    icon: BezPath,
    tool: Box<dyn Tool>,
}

/// The floating toolbar.
///
/// This is a very hacky implementation to get us rolling; it is not very
/// reusable, but can be refactored at a future date.
pub struct Toolbar {
    items: Vec<ToolbarItem>,
    selected: usize,
    widgets: Vec<WidgetPod<bool, Box<dyn Widget<bool>>>>,
    hide_toolbar: bool,
}

impl Toolbar {
    fn new(items: Vec<ToolbarItem>) -> Self {
        let mut widgets = Vec::with_capacity(items.capacity());
        for icon in items.iter().map(|item| item.icon.clone()) {
            let widg = Painter::new(move |ctx, is_selected: &bool, _| {
                let color = if *is_selected {
                    TOOLBAR_BG_SELECTED
                } else {
                    TOOLBAR_BG_DEFAULT
                };
                let frame = ctx.size().to_rect();
                ctx.fill(frame, &color);
                ctx.fill(&icon, &Color::WHITE);
                ctx.stroke(&icon, &Color::BLACK, TOOLBAR_ITEM_STROKE_WIDTH);
            });

            let widg = widg.on_click(|ctx, selected, _| {
                *selected = true;
                ctx.request_paint();
            });
            widgets.push(WidgetPod::new(widg.boxed()));
        }

        Toolbar {
            items,
            widgets,
            selected: 0,
            hide_toolbar: false,
        }
    }
}

impl<T: Data> Widget<T> for Toolbar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, env: &Env) {
        if let Event::Command(cmd) = event {
            //TODO: move to just a like 'SET_TOOL' command or something
            let selected = match cmd.selector {
                consts::cmd::PEN_TOOL => {
                    self.items.iter().position(|item| item.tool.name() == "Pen")
                }
                consts::cmd::SELECT_TOOL => self
                    .items
                    .iter()
                    .position(|item| item.tool.name() == "Select"),
                consts::cmd::PREVIEW_TOOL => self
                    .items
                    .iter()
                    .position(|item| item.tool.name() == "Preview"),
                _ => None,
            };

            self.selected = selected.unwrap_or(self.selected);
            if cmd.selector == consts::cmd::TOGGLE_PREVIEW_TOOL {
                let in_temporary_preview: &bool = cmd.get_object().unwrap();
                self.hide_toolbar = *in_temporary_preview;
                ctx.request_paint();
            }
        }

        for (i, child) in self.widgets.iter_mut().enumerate() {
            let mut is_selected = i == self.selected;
            child.event(ctx, event, &mut is_selected, env);

            if is_selected && i != self.selected {
                self.selected = i;
                //FIXME: this is dumb
                let cmd = match self.items[self.selected].tool.name() {
                    "Pen" => consts::cmd::PEN_TOOL,
                    "Select" => consts::cmd::SELECT_TOOL,
                    "Preview" => consts::cmd::PREVIEW_TOOL,
                    other => {
                        log::warn!("unknown tool '{}'", other);
                        return;
                    }
                };
                ctx.submit_command(cmd, None);
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, _data: &T, env: &Env) {
        for (i, child) in self.widgets.iter_mut().enumerate() {
            let is_selected = i == self.selected;
            child.lifecycle(ctx, event, &is_selected, env);
        }
    }

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &T, _data: &T, _env: &Env) {
        //todo!()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        let constraints = BoxConstraints::tight(TOOLBAR_ITEM_SIZE);
        let mut x_pos = TOOLBAR_ITEM_PADDING;

        for child in self.widgets.iter_mut() {
            // data doesn't matter here
            let size = child.layout(ctx, &constraints, &false, env);
            child.set_layout_rect(ctx, &false, env, Rect::from_origin_size((x_pos, 0.0), size));
            x_pos += TOOLBAR_ITEM_SIZE.width + TOOLBAR_ITEM_PADDING;
        }

        // Size doesn't account for stroke etc
        bc.constrain(Size::new(x_pos, TOOLBAR_ITEM_SIZE.height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        if self.hide_toolbar {
            return;
        }
        let frame = self
            .widgets
            .first()
            .map(|w| w.layout_rect())
            .unwrap_or_default();
        let frame = self
            .widgets
            .iter()
            .fold(frame, |acc, w| acc.union(w.layout_rect()));
        ctx.blurred_rect(frame + Vec2::new(2.0, 2.0), 4.0, &Color::grey(0.5));
        let rounded = frame.to_rounded_rect(5.0);
        ctx.fill(rounded, &TOOLBAR_BG_DEFAULT);
        for (i, child) in self.widgets.iter_mut().enumerate() {
            let is_selected = i == self.selected;
            child.paint_with_offset(ctx, &is_selected, env);
        }

        let stroke_inset = TOOLBAR_BORDER_STROKE_WIDTH / 2.0;
        for child in self.widgets.iter().skip(1) {
            let child_frame = child.layout_rect();
            let line = Line::new(
                (child_frame.min_x() - stroke_inset, child_frame.min_y()),
                (child_frame.min_x() - stroke_inset, child_frame.max_y()),
            );
            ctx.stroke(line, &Color::BLACK, TOOLBAR_BORDER_STROKE_WIDTH);
        }
        ctx.stroke(rounded, &Color::BLACK, TOOLBAR_BORDER_STROKE_WIDTH);
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        let select = ToolbarItem {
            icon: constrain_path(select_path()),
            tool: Box::new(Select::default()),
        };

        let pen = ToolbarItem {
            icon: constrain_path(pen_path()),
            tool: Box::new(Pen::default()),
        };

        let preview = ToolbarItem {
            icon: constrain_path(preview_path()),
            tool: Box::new(Preview::default()),
        };
        Toolbar::new(vec![select, pen, preview])
    }
}

fn constrain_path(mut path: BezPath) -> BezPath {
    let path_size = path.bounding_box().size();
    let icon_size = TOOLBAR_ITEM_SIZE.max_side() - TOOLBAR_ICON_PADDING * 2.0;
    let scale = icon_size / path_size.max_side();
    path.apply_affine(Affine::scale(scale));
    let center_offset = (TOOLBAR_ITEM_SIZE - (path_size * scale)).to_vec2() / 2.0;
    path.apply_affine(Affine::translate(center_offset));
    path
}

fn select_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((111.0, 483.0));
    bez.line_to((202.0, 483.0));
    bez.line_to((202.0, 328.0));
    bez.line_to((312.0, 361.0));
    bez.line_to((156.0, 0.0));
    bez.line_to((0.0, 360.0));
    bez.line_to((111.0, 330.0));
    bez.line_to((111.0, 483.0));
    bez.close_path();

    bez.apply_affine(Affine::rotate(-0.5));
    let origin = bez.bounding_box().origin();
    bez.apply_affine(Affine::translate(-origin.to_vec2()));
    bez
}

fn pen_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((97.0, 0.0));
    bez.line_to((214.0, 0.0));
    bez.line_to((273.0, 241.0));
    bez.line_to((315.0, 321.0));
    bez.line_to((260.0, 438.0));
    bez.line_to((260.0, 621.0));
    bez.line_to((50.0, 621.0));
    bez.line_to((50.0, 438.0));
    bez.line_to((0.0, 321.0));
    bez.line_to((45.0, 241.0));
    bez.line_to((97.0, 0.0));
    bez.close_path();

    bez.move_to((155.0, 311.0));
    bez.line_to((155.0, 0.0));
    bez.close_path();
    let circle = Circle::new((155.0, 361.0), 50.0);
    bez.extend(circle.to_bez_path(0.1));
    bez
}

fn preview_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((304.5, 576.5));
    bez.curve_to((304.5, 576.5), (300.5, 406.5), (302.5, 386.5));
    bez.curve_to((316.5, 264.5), (475.5, 281.5), (487.5, 219.5));
    bez.curve_to((491.5, 200.5), (468.5, 192.5), (444.5, 199.5));
    bez.curve_to((420.5, 206.5), (300.5, 257.5), (301.5, 238.5));
    bez.curve_to((302.5, 214.5), (387.5, 176.5), (412.5, 117.5));
    bez.curve_to((437.5, 58.5), (369.5, 88.5), (359.5, 103.5));
    bez.curve_to((349.5, 118.5), (283.5, 198.5), (262.5, 223.5));
    bez.curve_to((241.5, 248.5), (240.5, 237.5), (248.5, 218.5));
    bez.curve_to((256.5, 199.5), (263.5, 130.5), (298.5, 84.5));
    bez.curve_to((333.5, 38.5), (252.5, 15.5), (227.5, 48.5));
    bez.curve_to((202.5, 81.5), (219.5, 219.5), (214.5, 237.5));
    bez.curve_to((214.5, 237.5), (215.5, 246.5), (199.5, 240.5));
    bez.curve_to((183.5, 234.5), (171.5, 135.5), (183.5, 95.5));
    bez.curve_to((195.5, 55.5), (162.5, -46.5), (128.5, 24.5));
    bez.curve_to((94.5, 95.5), (142.5, 220.5), (145.5, 248.5));
    bez.curve_to((148.5, 276.5), (129.5, 296.5), (108.5, 260.5));
    bez.curve_to((87.5, 224.5), (16.5, 142.5), (3.5, 155.5));
    bez.curve_to((-9.5, 168.5), (14.5, 263.5), (54.5, 308.5));
    bez.curve_to((94.5, 353.5), (161.5, 323.5), (163.5, 381.5));
    bez.line_to((164.5, 577.5));
    bez.line_to((304.5, 576.5));
    bez.close_path();

    bez
}
