//! The toolbar widget

use druid::kurbo::{Affine, BezPath, Circle, Line, Shape, Vec2};
use druid::widget::prelude::*;
use druid::widget::{Painter, WidgetExt};
use druid::{Color, Data, HotKey, KeyEvent, Rect, SysMods, WidgetPod};

use crate::consts;
use crate::tools::ToolId;

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
            hotkey: HotKey::new(SysMods::Shift, "u"),
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
            select, pen, knife, preview, measure, rectangle, ellipse,
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
    bez.extend(circle.path_elements(0.1));
    bez
}

fn preview_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((320.0, 500.0)); // Bottom right: wrist
    bez.line_to((140.0, 500.0)); // Bottom left: wrist
    bez.line_to((140.0, 440.0));
    bez.line_to((30.0, 320.0));
    bez.line_to((0.0, 185.0));
    bez.line_to((20.0, 175.0)); // Top: thumb
    bez.line_to((40.0, 185.0));
    bez.line_to((75.0, 255.0));
    bez.line_to((110.0, 275.0));
    bez.line_to((125.0, 220.0));
    bez.line_to((120.0, 50.0)); // Top left: index finger
    bez.line_to((130.0, 40.0));
    bez.line_to((155.0, 40.0));
    bez.line_to((165.0, 50.0));
    bez.line_to((185.0, 215.0));
    bez.line_to((190.0, 15.0)); // Top left: middle finger
    bez.line_to((205.0, 0.0));
    bez.line_to((220.0, 0.0));
    bez.line_to((235.0, 15.0));
    bez.line_to((250.0, 210.0));
    bez.line_to((260.0, 210.0));
    bez.line_to((270.0, 110.0));
    bez.line_to((270.0, 30.0)); // Top left: ring finger
    bez.line_to((280.0, 20.0));
    bez.line_to((305.0, 20.0));
    bez.line_to((315.0, 30.0));
    bez.line_to((320.0, 130.0));
    bez.line_to((315.0, 225.0));
    bez.line_to((325.0, 230.0));
    bez.line_to((340.0, 165.0));
    bez.line_to((345.0, 90.0)); // Top left: little finger
    bez.line_to((355.0, 80.0));
    bez.line_to((355.0, 80.0));
    bez.line_to((385.0, 90.0));
    bez.line_to((385.0, 200.0));
    bez.line_to((365.0, 320.0));
    bez.line_to((320.0, 420.0));
    bez.line_to((320.0, 500.0));
    bez.close_path();
    bez
}

fn rect_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((0.0, 0.0));
    bez.line_to((246.0, 0.0));
    bez.line_to((246.0, 246.0));
    bez.line_to((0.0, 246.0));
    bez.line_to((0.0, 0.0));
    bez.close_path();

    bez.move_to((404.0, 404.0));
    bez.line_to((140.0, 404.0));
    bez.line_to((140.0, 140.0));
    bez.line_to((404.0, 140.0));
    bez.line_to((404.0, 404.0));
    bez.close_path();
    bez
}

fn knife_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((174.0, 647.0));
    bez.line_to((50.0, 647.0));
    bez.line_to((50.0, 218.0));
    bez.line_to((174.0, 218.0));
    bez.line_to((174.0, 647.0));
    bez.close_path();

    bez.move_to((50.0, 183.0));
    bez.line_to((50.0, 0.0));
    bez.line_to((175.0, 183.0));
    bez.line_to((50.0, 183.0));
    bez.close_path();

    bez.move_to((0.0, 311.0));
    bez.curve_to((0.0, 281.0), (30.0, 261.0), (50.0, 261.0));
    bez.curve_to((74.0, 261.0), (100.0, 289.0), (100.0, 311.0));
    bez.curve_to((100.0, 333.0), (71.0, 361.0), (50.0, 361.0));
    bez.curve_to((29.0, 361.0), (0.0, 340.0), (0.0, 311.0));
    bez.close_path();
    bez
}

fn ellipse_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((73.0, 0.0));
    bez.curve_to((33.0, 0.0), (0.0, 49.0), (0.0, 109.0));
    bez.curve_to((0.0, 170.0), (33.0, 219.0), (73.0, 219.0));
    bez.curve_to((113.0, 219.0), (146.0, 170.0), (146.0, 109.0));
    bez.curve_to((146.0, 49.0), (113.0, 0.0), (73.0, 0.0));
    bez.close_path();

    bez.move_to((243.0, 109.0));
    bez.curve_to((243.0, 79.0), (196.0, 54.0), (137.0, 54.0));
    bez.curve_to((78.0, 54.0), (31.0, 79.0), (31.0, 109.0));
    bez.curve_to((31.0, 140.0), (78.0, 165.0), (137.0, 165.0));
    bez.curve_to((196.0, 165.0), (243.0, 140.0), (243.0, 109.0));
    bez.close_path();
    bez
}

fn measure_path() -> BezPath {
    let mut bez = BezPath::new();

    bez.move_to((0.0, 0.0));
    bez.line_to((50.0, 0.0));
    bez.line_to((50.0, 200.0));
    bez.line_to((0.0, 200.0));
    bez.close_path();

    bez.move_to((50.0, 175.0));
    bez.line_to((30.0, 175.0));
    bez.close_path();

    bez.move_to((50.0, 150.0));
    bez.line_to((25.0, 150.0));
    bez.close_path();

    bez.move_to((50.0, 125.0));
    bez.line_to((30.0, 125.0));
    bez.close_path();

    bez.move_to((50.0, 100.0));
    bez.line_to((20.0, 100.0));
    bez.close_path();

    bez.move_to((50.0, 75.0));
    bez.line_to((30.0, 75.0));
    bez.close_path();

    bez.move_to((50.0, 50.0));
    bez.line_to((25.0, 50.0));
    bez.close_path();

    bez.move_to((50.0, 25.0));
    bez.line_to((30.0, 25.0));
    bez.close_path();
    bez
}
