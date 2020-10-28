//! The floating panel that displays the sidebearings, advance, and other
//! glyph metrics

use druid::kurbo::Affine;
use druid::widget::{prelude::*, Controller, Flex, Painter};
use druid::WidgetExt;

use crate::data::{EditorState, Sidebearings};
use crate::design_space::DVec2;
use crate::widgets::EditableLabel;
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
        let d_x = if child_data.left != data.left {
            child_data.left - data.left
        } else if child_data.right != data.right {
            data.right - child_data.right
        } else {
            0.0
        };

        if d_x != 0.0 {
            let delta = DVec2::from_raw((d_x, 0.0));
            ctx.submit_command(consts::cmd::NUDGE_EVERYTHING.with(delta));
        }
        // suppress clicks so that the editor doesn't handle them.
        if matches!(event,Event::MouseUp(_) | Event::MouseDown(_)) {
            ctx.set_handled();
        }
    }
}

fn build_widget() -> impl Widget<EditorState> {
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
            Painter::new(|ctx, data: &EditorState, env| {
                let frame = ctx.size().to_rect();
                if let Some(outline) = data.font.get_bezier(&data.session.name) {
                    let max_dim = data.metrics.units_per_em;
                    let min_avail = frame.height().min(frame.width());
                    let scale = min_avail / max_dim;
                    let baseline = data.metrics.ascender.unwrap_or(0.0) * scale;
                    let affine = Affine::new([scale, 0.0, 0.0, -scale, 0.0, baseline]);
                    let path = affine * outline.as_ref();
                    ctx.fill(path, &env.get(theme::SECONDARY_TEXT_COLOR));
                }
            })
            .fix_width(40.0)
            .fix_height(40.0),
        )
        .with_child(
            EditableLabel::parse()
                .with_font(theme::UI_DETAIL_FONT)
                .lens(Sidebearings::right)
                .controller(GlyphPane)
                .lens(EditorState::sidebearings)
                .fix_width(40.0),
        )
        .padding(4.0)
}
