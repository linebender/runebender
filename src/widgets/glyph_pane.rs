//! The floating panel that displays the sidebearings, advance, and other
//! glyph metrics

use druid::widget::{prelude::*, Controller, Flex};
use druid::{LensExt, WidgetExt};

use crate::data::{EditorState, GlyphDetail, Sidebearings};
use crate::widgets::{EditableLabel, GlyphPainter};
use crate::{consts, theme};

/// A panel for editing the selected coordinate
pub struct GlyphPane;

impl GlyphPane {
    // this is not a blessed pattern
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> impl Widget<EditorState> {
        build_widget()
    }
}

impl<W: Widget<Sidebearings>> Controller<Sidebearings, W> for GlyphPane {
    #[allow(clippy::float_cmp)]
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Sidebearings,
        env: &Env,
    ) {
        let mut child_data = data.clone();
        child.event(ctx, event, &mut child_data, env);

        // if an edit has occured in the panel, we turn it into
        // a command so that the Editor can update undo state:
        let (d_x, is_left) = if child_data.left != data.left {
            (child_data.left - data.left, true)
        } else if child_data.right != data.right {
            (child_data.right - data.right, false)
        } else {
            (0.0, false)
        };

        if d_x != 0.0 {
            let args = consts::cmd::AdjustSidebearing {
                delta: d_x,
                is_left,
            };
            ctx.submit_command(consts::cmd::ADJUST_SIDEBEARING.with(args));
        }
        // suppress clicks so that the editor doesn't handle them.
        if matches!(event,Event::MouseUp(_) | Event::MouseDown(_)) {
            ctx.set_handled();
        }
    }
}

fn build_widget() -> impl Widget<EditorState> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_child(
                    EditableLabel::parse()
                        .with_font(theme::UI_DETAIL_FONT)
                        .lens(Sidebearings::left)
                        .controller(GlyphPane)
                        .lens(EditorState::sidebearings)
                        .fix_width(40.0),
                )
                .with_child(
                    GlyphPainter::new()
                        .color(theme::SECONDARY_TEXT_COLOR)
                        .draw_layout_frame(true)
                        .fix_height(40.0)
                        .padding((0., 8.0))
                        .lens(EditorState::detail_glyph),
                )
                .with_child(
                    EditableLabel::parse()
                        .with_font(theme::UI_DETAIL_FONT)
                        .lens(Sidebearings::right)
                        .controller(GlyphPane)
                        .lens(EditorState::sidebearings)
                        .fix_width(40.0),
                ),
        )
        .with_child(
            EditableLabel::parse()
                .with_font(theme::UI_DETAIL_FONT)
                .lens(EditorState::detail_glyph.then(GlyphDetail::advance))
                .fix_width(40.0),
        )
        .padding(4.0)
}
