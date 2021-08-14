//! The toolbar widget

use druid::kurbo::{Affine, BezPath, Line, Shape, Vec2};
use druid::widget::prelude::*;
use druid::widget::{Painter, WidgetExt};
use druid::{Color, Data, HotKey, KeyEvent, Rect, SysMods, WidgetPod};

use crate::consts;
use crate::tools::ToolId;

const TOOLBAR_ITEM_SIZE: Size = Size::new(48.0, 48.0);
const TOOLBAR_ITEM_PADDING: f64 = 2.0;
const TOOLBAR_ICON_PADDING: f64 = 6.0;
const TOOLBAR_BORDER_STROKE_WIDTH: f64 = 2.0;
const TOOLBAR_ITEM_STROKE_WIDTH: f64 = 1.5;
// TODO: move these to theme
const TOOLBAR_BG_DEFAULT: Color = Color::grey8(0xDD);
const TOOLBAR_BG_SELECTED: Color = Color::grey8(0xAD);

struct ToolbarItem {
    icon: BezPath,
    name: ToolId,
    hotkey: HotKey,
}

/// The floating toolbar.
///
/// This is a very hacky implementation to get us rolling; it is not very
/// reusable, but can be refactored at a future date.
pub struct Toolbar {
    items: Vec<ToolbarItem>,
    selected: usize,
    widgets: Vec<WidgetPod<bool, Box<dyn Widget<bool>>>>,
}

/// A wrapper around control UI elements, drawing a drop shadow & rounded rect
pub struct FloatingPanel<W> {
    hide_panel: bool,
    inner: W,
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
        }
    }

    pub fn tool_for_keypress(&self, key: &KeyEvent) -> Option<ToolId> {
        self.items
            .iter()
            .find(|tool| tool.hotkey.matches(key))
            .map(|tool| tool.name)
    }
}

impl<T: Data> Widget<T> for Toolbar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut T, env: &Env) {
        if let Event::Command(cmd) = event {
            if let Some(tool_id) = cmd.get(consts::cmd::SET_TOOL) {
                let sel = self.items.iter().position(|item| item.name == *tool_id);
                self.selected = sel.unwrap_or(self.selected);
                ctx.request_paint();
            }
        }

        for (i, child) in self.widgets.iter_mut().enumerate() {
            let mut is_selected = i == self.selected;
            child.event(ctx, event, &mut is_selected, env);

            if is_selected && i != self.selected {
                let tool = self.items[i].name;
                ctx.submit_command(consts::cmd::SET_TOOL.with(tool));
            }
        }

        // if there's a click here we don't want to pass it down to the child
        if matches!(event, Event::MouseDown(_) | Event::MouseUp(_)) {
            ctx.set_handled();
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
        let mut x_pos = 0.0;

        for child in self.widgets.iter_mut() {
            // data doesn't matter here
            let size = child.layout(ctx, &constraints, &false, env);
            child.set_layout_rect(ctx, &false, env, Rect::from_origin_size((x_pos, 0.0), size));
            x_pos += TOOLBAR_ITEM_SIZE.width + TOOLBAR_ITEM_PADDING;
        }

        // Size doesn't account for stroke etc
        bc.constrain(Size::new(
            x_pos - TOOLBAR_ITEM_PADDING,
            TOOLBAR_ITEM_SIZE.height,
        ))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        for (i, child) in self.widgets.iter_mut().enumerate() {
            let is_selected = i == self.selected;
            child.paint(ctx, &is_selected, env);
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
    }
}

impl<W> FloatingPanel<W> {
    pub fn new(inner: W) -> Self {
        FloatingPanel {
            hide_panel: false,
            inner,
        }
    }

    /// return a reference to the inner widget.
    pub fn inner(&self) -> &W {
        &self.inner
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for FloatingPanel<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
        if let Event::Command(cmd) = event {
            if let Some(in_temporary_preview) = cmd.get(consts::cmd::TOGGLE_PREVIEW_TOOL) {
                self.hide_panel = *in_temporary_preview;
                ctx.request_paint();
            }
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        ctx.set_paint_insets((0., 6.0, 6.0, 0.));
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.hide_panel {
            return;
        }
        let frame = ctx.size().to_rect();
        ctx.blurred_rect(frame + Vec2::new(2.0, 2.0), 4.0, &Color::grey(0.5));
        let rounded = frame.to_rounded_rect(5.0);
        ctx.fill(rounded, &TOOLBAR_BG_DEFAULT);
        ctx.with_save(|ctx| {
            ctx.clip(rounded);
            self.inner.paint(ctx, data, env);
        });
        ctx.stroke(rounded, &Color::BLACK, TOOLBAR_BORDER_STROKE_WIDTH);
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        let select = ToolbarItem {
            name: "Select",
            icon: constrain_path(select_path()),
            hotkey: HotKey::new(None, "v"),
        };

        let pen = ToolbarItem {
            name: "Pen",
            icon: constrain_path(pen_path()),
            hotkey: HotKey::new(None, "p"),
        };

        let hyperpen = ToolbarItem {
            name: "HyperPen",
            icon: constrain_path(hyperpen_path()),
            hotkey: HotKey::new(SysMods::Shift, "P"),
        };

        let preview = ToolbarItem {
            name: "Preview",
            icon: constrain_path(preview_path()),
            hotkey: HotKey::new(None, "h"),
        };

        let rectangle = ToolbarItem {
            name: "Rectangle",
            icon: constrain_path(rect_path()),
            hotkey: HotKey::new(None, "u"),
        };

        let ellipse = ToolbarItem {
            name: "Ellipse",
            icon: constrain_path(ellipse_path()),
            hotkey: HotKey::new(SysMods::Shift, "U"),
        };

        let knife = ToolbarItem {
            name: "Knife",
            icon: constrain_path(knife_path()),
            hotkey: HotKey::new(None, "e"),
        };

        let measure = ToolbarItem {
            name: "Measure",
            icon: constrain_path(measure_path()),
            hotkey: HotKey::new(None, "m"),
        };

        Toolbar::new(vec![
            select, pen, hyperpen, knife, preview, measure, rectangle, ellipse,
        ])
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

    bez.move_to((110.0, 500.0));
    bez.line_to((110.0, 380.0));
    bez.line_to((2.0, 410.0));
    bez.line_to((0.0, 410.0));
    bez.line_to((159.0, 0.0));
    bez.line_to((161.0, 0.0));
    bez.line_to((320.0, 410.0));
    bez.line_to((318.0, 410.0));
    bez.line_to((210.0, 380.0));
    bez.line_to((210.0, 500.0));
    bez.line_to((110.0, 500.0));
    bez.close_path();
    bez
}

fn pen_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((40.0, 500.0));
    bez.line_to((240.0, 500.0));
    bez.line_to((240.0, 410.0));
    bez.line_to((40.0, 410.0));
    bez.line_to((40.0, 500.0));
    bez.close_path();

    bez.move_to((40.0, 410.0));
    bez.line_to((240.0, 410.0));
    bez.line_to((239.0, 370.0));
    bez.line_to((280.0, 290.0));
    bez.curve_to((240.0, 220.0), (205.0, 130.0), (195.0, 0.0));
    bez.line_to((85.0, 0.0));
    bez.curve_to((75.0, 130.0), (40.0, 220.0), (0.0, 290.0));
    bez.line_to((40.0, 370.0));
    bez.line_to((40.0, 410.0));
    bez.close_path();

    bez.move_to((140.0, 0.0));
    bez.line_to((140.0, 266.0));

    bez.move_to((173.0, 300.0));
    bez.curve_to((173.0, 283.0), (159.0, 267.0), (140.0, 267.0));
    bez.curve_to((121.0, 267.0), (107.0, 283.0), (107.0, 300.0));
    bez.curve_to((107.0, 317.0), (121.0, 333.0), (140.0, 333.0));
    bez.curve_to((159.0, 333.0), (173.0, 317.0), (173.0, 300.0));
    bez.close_path();
    bez
}

fn hyperpen_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((500.0, 250.0));
    bez.curve_to((500.0, 196.0), (350.0, 160.0), (250.0, 160.0));
    bez.curve_to((150.0, 160.0), (0.0, 193.0), (0.0, 250.0));
    bez.curve_to((0.0, 308.0), (150.0, 340.0), (250.0, 340.0));
    bez.curve_to((350.0, 340.0), (500.0, 298.0), (500.0, 250.0));
    bez.close_path();

    bez.move_to((500.0, 250.0));
    bez.curve_to((500.0, 107.0), (387.0, 0.0), (250.0, 0.0));
    bez.curve_to((112.0, 0.0), (0.0, 113.0), (0.0, 250.0));
    bez.curve_to((0.0, 388.0), (112.0, 500.0), (249.0, 500.0));
    bez.curve_to((387.0, 500.0), (500.0, 387.0), (500.0, 250.0));
    bez.close_path();

    bez.move_to((410.0, 400.0));
    bez.curve_to((410.0, 280.0), (230.0, 30.0), (160.0, 30.0));
    bez.curve_to((110.0, 30.0), (90.0, 60.0), (90.0, 100.0));
    bez.curve_to((90.0, 220.0), (270.0, 470.0), (340.0, 470.0));
    bez.curve_to((390.0, 470.0), (410.0, 440.0), (410.0, 400.0));
    bez.close_path();

    bez.move_to((410.0, 100.0));
    bez.curve_to((410.0, 60.0), (390.0, 30.0), (340.0, 30.0));
    bez.curve_to((270.0, 30.0), (90.0, 280.0), (90.0, 400.0));
    bez.curve_to((90.0, 440.0), (110.0, 470.0), (160.0, 470.0));
    bez.curve_to((230.0, 470.0), (410.0, 220.0), (410.0, 100.0));
    bez.close_path();
    bez
}

fn knife_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((30.0, 500.0));
    bez.line_to((190.0, 500.0));
    bez.line_to((190.0, 410.0));
    bez.line_to((30.0, 410.0));
    bez.line_to((30.0, 500.0));
    bez.close_path();

    bez.move_to((40.0, 360.0));
    bez.line_to((180.0, 360.0));
    bez.line_to((180.0, 330.0));
    bez.line_to((220.0, 290.0));
    bez.line_to((42.0, 0.0));
    bez.line_to((40.0, 0.0));
    bez.line_to((40.0, 360.0));
    bez.close_path();

    bez.move_to((30.0, 410.0));
    bez.line_to((190.0, 410.0));
    bez.curve_to((205.0, 410.0), (220.0, 405.0), (220.0, 385.0));
    bez.curve_to((220.0, 365.0), (205.0, 360.0), (190.0, 360.0));
    bez.line_to((30.0, 360.0));
    bez.curve_to((15.0, 360.0), (0.0, 365.0), (0.0, 385.0));
    bez.curve_to((0.0, 405.0), (15.0, 410.0), (30.0, 410.0));
    bez.close_path();
    bez
}

fn preview_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((130.0, 500.0));
    bez.line_to((310.0, 500.0));
    bez.line_to((310.0, 410.0));
    bez.curve_to((336.0, 375.0), (360.0, 351.0), (360.0, 310.0));
    bez.line_to((360.0, 131.0));
    bez.curve_to((360.0, 89.0), (352.0, 70.0), (336.0, 70.0));
    bez.curve_to((316.0, 70.0), (310.0, 85.0), (310.0, 101.0));
    bez.curve_to((310.0, 60.0), (309.0, 20.0), (280.0, 20.0));
    bez.curve_to((260.0, 20.0), (250.0, 36.0), (250.0, 60.0));
    bez.curve_to((250.0, 26.0), (242.0, 0.0), (216.0, 0.0));
    bez.curve_to((192.0, 0.0), (180.0, 16.0), (180.0, 75.0));
    bez.curve_to((180.0, 48.0), (169.0, 30.0), (150.0, 30.0));
    bez.curve_to((130.0, 30.0), (120.0, 53.0), (120.0, 75.0));
    bez.line_to((120.0, 250.0));
    bez.curve_to((120.0, 270.0), (110.0, 270.0), (100.0, 270.0));
    bez.curve_to((85.0, 270.0), (77.0, 264.0), (70.0, 250.0));
    bez.curve_to((45.0, 199.0), (32.0, 190.0), (20.0, 190.0));
    bez.curve_to((8.0, 190.0), (0.0, 197.0), (0.0, 210.0));
    bez.curve_to((0.0, 234.0), (19.0, 313.0), (30.0, 330.0));
    bez.curve_to((41.0, 347.0), (87.0, 383.0), (130.0, 410.0));
    bez.line_to((130.0, 500.0));
    bez.close_path();

    bez.move_to((130.0, 410.0));
    bez.line_to((310.0, 410.0));

    bez.move_to((180.0, 75.0));
    bez.line_to((180.0, 210.0));

    bez.move_to((250.0, 60.0));
    bez.line_to((250.0, 210.0));

    bez.move_to((310.0, 101.0));
    bez.line_to((310.0, 220.0));
    bez
}

fn measure_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((0.0, 500.0));
    bez.line_to((140.0, 500.0));
    bez.line_to((140.0, 0.0));
    bez.line_to((0.0, 0.0));
    bez.line_to((0.0, 500.0));
    bez.close_path();

    bez.move_to((190.0, 0.0));
    bez.line_to((330.0, 0.0));

    bez.move_to((190.0, 500.0));
    bez.line_to((330.0, 500.0));

    bez.move_to((210.0, 100.0));
    bez.line_to((310.0, 100.0));
    bez.line_to((260.0, 10.0));
    bez.line_to((210.0, 100.0));
    bez.close_path();

    bez.move_to((210.0, 400.0));
    bez.line_to((310.0, 400.0));
    bez.line_to((260.0, 490.0));
    bez.line_to((210.0, 400.0));
    bez.close_path();

    bez.move_to((260.0, 100.0));
    bez.line_to((260.0, 400.0));

    bez.move_to((70.0, 350.0));
    bez.line_to((140.0, 350.0));

    bez.move_to((100.0, 400.0));
    bez.line_to((140.0, 400.0));

    bez.move_to((50.0, 450.0));
    bez.line_to((140.0, 450.0));

    bez.move_to((100.0, 300.0));
    bez.line_to((140.0, 300.0));

    bez.move_to((50.0, 250.0));
    bez.line_to((140.0, 250.0));

    bez.move_to((70.0, 150.0));
    bez.line_to((140.0, 150.0));

    bez.move_to((100.0, 200.0));
    bez.line_to((140.0, 200.0));

    bez.move_to((100.0, 100.0));
    bez.line_to((140.0, 100.0));

    bez.move_to((50.0, 50.0));
    bez.line_to((140.0, 50.0));
    bez
}

fn rect_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((0.0, 500.0));
    bez.line_to((220.0, 500.0));
    bez.line_to((220.0, 0.0));
    bez.line_to((0.0, 0.0));
    bez.line_to((0.0, 500.0));
    bez.close_path();
    bez
}

fn ellipse_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((110.0, 0.0));
    bez.curve_to((50.0, 0.0), (0.0, 100.0), (0.0, 240.0));
    bez.curve_to((0.0, 380.0), (50.0, 480.0), (110.0, 480.0));
    bez.curve_to((170.0, 480.0), (220.0, 380.0), (220.0, 240.0));
    bez.curve_to((220.0, 100.0), (170.0, 0.0), (110.0, 0.0));
    bez.close_path();
    bez
}
