//! The floating panel that displays the coordinate of the currently
//! selected point.

use druid::kurbo::{Circle, Vec2};
use druid::widget::{prelude::*, Either, Flex, Label, SizedBox};
use druid::{Color, Point, WidgetExt, WidgetPod};

use crate::edit_session::{CoordinateSelection, Quadrant};
use crate::widgets::EditableLabel;

/// A panel for editing the selected coordinate
pub struct CoordPane {
    inner: WidgetPod<CoordinateSelection, Box<dyn Widget<CoordinateSelection>>>,
    current_type: SelectionType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SelectionType {
    None,
    Single,
    Multi,
}

/// A widget for picking how to represent a multi-point selection.
struct CoordRepresentationPicker;

impl CoordPane {
    pub fn new() -> Self {
        CoordPane {
            inner: WidgetPod::new(SizedBox::empty().boxed()),
            current_type: SelectionType::None,
        }
    }

    fn rebuild_inner(&mut self, selection: &CoordinateSelection) {
        self.current_type = SelectionType::from_selection(selection);
        let new_widget = match self.current_type {
            SelectionType::None => SizedBox::empty().boxed(),
            SelectionType::Single => single_point_selected().boxed(),
            SelectionType::Multi => single_point_selected().boxed(),
        };
        self.inner = WidgetPod::new(new_widget);
    }
}

impl Widget<CoordinateSelection> for CoordPane {
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut CoordinateSelection,
        env: &Env,
    ) {
        self.inner.event(ctx, event, data, env);
        if matches!(event,Event::MouseUp(_) | Event::MouseDown(_)) {
            ctx.set_handled();
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &CoordinateSelection,
        env: &Env,
    ) {
        if matches!(event, LifeCycle::WidgetAdded) || self.current_type.will_change(data) {
            self.rebuild_inner(data);
            ctx.children_changed();
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: &CoordinateSelection,
        data: &CoordinateSelection,
        env: &Env,
    ) {
        if self.current_type.will_change(data) {
            self.rebuild_inner(data);
            ctx.children_changed();
        } else {
            self.inner.update(ctx, data, env);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &CoordinateSelection,
        env: &Env,
    ) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        self.inner.set_layout_rect(ctx, data, env, size.to_rect());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &CoordinateSelection, env: &Env) {
        self.inner.paint(ctx, data, env);
    }
}

impl Widget<Quadrant> for CoordRepresentationPicker {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Quadrant, _env: &Env) {
        match event {
            Event::MouseDown(mouse) if mouse.button.is_left() => {
                ctx.set_active(true);
                *data = Quadrant::for_point_in_size(mouse.pos, ctx.size());
                dbg!(data);
                ctx.request_paint();
            }
            Event::MouseUp(_) => {
                if ctx.is_active() {
                    ctx.set_active(false);
                    ctx.request_paint();
                }
            }
            _ => (),
        }
    }

    fn lifecycle(&mut self, _: &mut LifeCycleCtx, _: &LifeCycle, _: &Quadrant, _: &Env) {}

    fn update(&mut self, _ctx: &mut UpdateCtx, _old_data: &Quadrant, _data: &Quadrant, _env: &Env) {
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _: &Quadrant, _: &Env) -> Size {
        let side_len = bc.max().min_side();
        Size::new(side_len, side_len)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Quadrant, env: &Env) {
        let frame_size = ctx.size();
        let padding = 5.0;
        let circle_radius = 2.0;
        let rect = frame_size.to_rect().inset(-padding);
        ctx.stroke(rect, &Color::BLACK, 1.0);
        for quadrant in Quadrant::all() {
            let pt = quadrant.pos_in_size(rect.size());
            let pt = pt + Vec2::new(5.0, 5.0);
            let color = if data == quadrant {
                env.get(druid::theme::SELECTION_COLOR)
            } else {
                Color::BLACK
            };
            ctx.fill(Circle::new(pt, circle_radius), &color);
        }
    }
}

impl SelectionType {
    fn from_selection(session: &CoordinateSelection) -> Self {
        match session.count {
            0 => Self::None,
            1 => Self::Single,
            _ => Self::Multi,
        }
    }

    fn will_change(self, session: &CoordinateSelection) -> bool {
        self != Self::from_selection(session)
    }
}

fn single_point_selected() -> impl Widget<CoordinateSelection> {
    let point_x_lens = druid::lens!(Point, x);
    let point_y_lens = druid::lens!(Point, y);

    let coord_picker = Either::new(
        |d, _| d.count > 1,
        CoordRepresentationPicker
            .lens(CoordinateSelection::quadrant)
            .fix_width(40.0)
            .padding((0., 0., 8.0, 0.)),
        SizedBox::empty(),
    );

    let coord_editor = Flex::column()
        .with_child(
            Flex::row()
                .with_child(Label::new("x:").with_text_size(12.0))
                .with_spacer(4.0)
                .with_child(EditableLabel::parse().lens(point_x_lens).fix_width(40.0)),
        )
        .with_child(
            Flex::row()
                .with_child(Label::new("y:").with_text_size(12.0))
                .with_spacer(4.0)
                .with_child(EditableLabel::parse().lens(point_y_lens).fix_width(40.0)),
        )
        .lens(CoordinateSelection::quadrant_coord);

    Flex::row()
        .with_child(coord_picker)
        .with_child(coord_editor)
        .padding(4.0)
}

impl Default for CoordPane {
    fn default() -> Self {
        CoordPane::new()
    }
}
