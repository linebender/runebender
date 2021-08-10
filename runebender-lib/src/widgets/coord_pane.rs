//! The floating panel that displays the coordinate of the currently
//! selected point.

use druid::kurbo::Circle;
use druid::widget::{prelude::*, Controller, CrossAxisAlignment, Either, Flex, Label, SizedBox};
use druid::{Color, FontDescriptor, FontFamily, Point, WidgetExt};

use crate::design_space::{DPoint, DVec2};
use crate::edit_session::CoordinateSelection;
use crate::quadrant::Quadrant;
use crate::widgets::EditableLabel;
use crate::{consts, theme, util};

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
        let mut child_data = *data;
        child.event(ctx, event, &mut child_data, env);
        data.quadrant = child_data.quadrant;

        // if another edit has occured in the coordpanel, we turn it into
        // a command so that the Editor can update undo state:
        if child_data.frame.origin() != data.frame.origin() {
            let delta = child_data.frame.origin() - data.frame.origin();
            ctx.submit_command(consts::cmd::NUDGE_SELECTION.with(DVec2::from_raw(delta)));
        } else if child_data.frame.size() != data.frame.size() {
            let scale = util::compute_scale(data.frame.size(), child_data.frame.size());
            let scale_origin = child_data.quadrant.point_in_dspace_rect(data.frame);
            let args = consts::cmd::ScaleSelectionArgs {
                scale,
                origin: DPoint::from_raw(scale_origin),
            };
            ctx.submit_command(consts::cmd::SCALE_SELECTION.with(args));
        }

        // suppress clicks so that the editor doesn't handle them.
        if matches!(event, Event::MouseUp(_) | Event::MouseDown(_)) {
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
                *data = Quadrant::for_point_in_bounds(mouse.pos, ctx.size());
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
        let padding = 8.0;
        let circle_radius = 6.0;
        let rect = frame_size.to_rect().inset(-padding);
        ctx.stroke(rect, &Color::BLACK, 1.0);
        for quadrant in Quadrant::all() {
            let pt = quadrant.point_in_rect(rect);
            let color = if data == quadrant {
                env.get(theme::FOCUS_BACKGROUND_COLOR)
            } else {
                env.get(theme::OFF_CURVE_POINT_OUTER_COLOR)
            };
            let stroke_color = env.get(theme::PATH_FILL_COLOR);
            ctx.fill(Circle::new(pt, circle_radius), &color);
            ctx.stroke(Circle::new(pt, circle_radius), &stroke_color, 1.0);
        }
    }
}

fn build_widget() -> impl Widget<CoordinateSelection> {
    // kurbo types don't derive lens
    let point_x_lens = druid::lens!(Point, x);
    let point_y_lens = druid::lens!(Point, y);

    let size_width_lens = druid::lens!(Size, width);
    let size_height_lens = druid::lens!(Size, height);

    let coord_picker = Either::new(
        |d, _| d.count > 1,
        CoordRepresentationPicker
            .lens(CoordinateSelection::quadrant)
            .fix_width(64.0)
            .padding((0., 0., 8.0, 0.)),
        SizedBox::empty(),
    );

    let coord_label_font: FontDescriptor = FontDescriptor::new(FontFamily::MONOSPACE);
    let coord_font: FontDescriptor = FontDescriptor::new(FontFamily::MONOSPACE);

    let coord_editor = Flex::column()
        .with_child(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Baseline)
                .with_child(
                    Label::new("x")
                        .with_font(coord_label_font.clone())
                        .with_text_size(16.0)
                        .with_text_color(theme::SECONDARY_TEXT_COLOR)
                        .padding((0.0, 0.0, 0.0, 8.0)),
                )
                .with_child(
                    EditableLabel::parse()
                        .with_font(coord_font.clone())
                        .with_text_size(16.0)
                        .lens(point_x_lens)
                        .fix_width(64.0)
                        .padding((0.0, 0.0, 0.0, 8.0)),
                ),
        )
        .with_child(
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Baseline)
                .with_child(
                    Label::new("y")
                        .with_font(coord_label_font.clone())
                        .with_text_size(16.0)
                        .with_text_color(theme::SECONDARY_TEXT_COLOR),
                )
                .with_child(
                    EditableLabel::parse()
                        .with_font(coord_font.clone())
                        .with_text_size(16.0)
                        .lens(point_y_lens)
                        .fix_width(64.0),
                ),
        )
        .lens(CoordinateSelection::quadrant_coord);

    let bbox_info = Either::new(
        |d, _| d.count > 1,
        Flex::column()
            .with_child(
                Flex::row()
                    .with_child(
                        Label::new("w")
                            .with_font(coord_label_font.clone())
                            .with_text_size(16.0)
                            .with_text_color(theme::SECONDARY_TEXT_COLOR)
                            .padding((8.0, 0.0, 0.0, 8.0)),
                    )
                    .with_spacer(0.0)
                    .with_child(
                        EditableLabel::parse()
                            .with_font(coord_font.clone())
                            .with_text_size(16.0)
                            .lens(size_width_lens)
                            .fix_width(64.0)
                            .padding((0.0, 0.0, 0.0, 8.0)),
                    ),
            )
            .with_child(
                Flex::row()
                    .with_child(
                        Label::new("h")
                            .with_font(coord_label_font)
                            .with_text_size(16.0)
                            .with_text_color(theme::SECONDARY_TEXT_COLOR)
                            .padding((8.0, 0.0, 0.0, 0.0)),
                    )
                    .with_spacer(0.0)
                    .with_child(
                        EditableLabel::parse()
                            .with_font(coord_font.clone())
                            .with_text_size(16.0)
                            .lens(size_height_lens)
                            .fix_width(64.0),
                    ),
            )
            .lens(CoordinateSelection::quadrant_bbox),
        SizedBox::empty(),
    );

    let picker_and_editor = Flex::row()
        .with_child(coord_picker)
        .with_child(coord_editor)
        .with_child(bbox_info)
        .padding(8.0);

    // if we have any points selected, show the numerical adjust widget, else an empty widget
    Either::new(|d, _| d.count != 0, picker_and_editor, SizedBox::empty())
}
