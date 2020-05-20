//! The floating panel that displays the coordinate of the currently
//! selected point.

use druid::kurbo::{Circle, Vec2};
use druid::widget::{prelude::*, Controller, Either, Flex, Label, SizedBox};
use druid::{Color, Point, WidgetExt};

use crate::edit_session::{CoordinateSelection, Quadrant};
use crate::widgets::EditableLabel;

/// A panel for editing the selected coordinate
pub struct CoordPane;

impl CoordPane {
    // this is not a blessed pattern
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> impl Widget<CoordinateSelection> {
        build_widget().controller(CoordPane)
    }
}

impl<W: Widget<CoordinateSelection>> Controller<CoordinateSelection, W> for CoordPane {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut CoordinateSelection,
        env: &Env,
    ) {
        child.event(ctx, event, data, env);
        // suppress clicks so that the editor doesn't handle them.
        if matches!(event,Event::MouseUp(_) | Event::MouseDown(_)) {
            ctx.set_handled();
        }
    }
}

/// A widget for picking how to represent a multi-point selection.
struct CoordRepresentationPicker;

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
        let circle_radius = 2.5;
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

fn build_widget() -> impl Widget<CoordinateSelection> {
    // kurbo types don't derive lens
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

    let picker_and_editor = Flex::row()
        .with_child(coord_picker)
        .with_child(coord_editor)
        .padding(4.0);

    // if we have any points selected, show the numerical adjust widget, else an empty widget
    Either::new(|d, _| d.count != 0, picker_and_editor, SizedBox::empty())
}
