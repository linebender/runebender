//! The sidebar of the main glyph list/grid view.

use druid::kurbo::Line;
use druid::{
    BoxConstraints, Color, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx,
    PaintCtx, Rect, RenderContext, Size, UpdateCtx, Widget, WidgetPod,
};

use druid::widget::{Controller, Flex, Label, SizedBox, WidgetExt};

use norad::GlyphName;

use crate::data::{GlyphDetail, Workspace};
use crate::theme;
use crate::widgets::{EditableLabel, GlyphPainter, Maybe};

const SELECTED_GLYPH_BOTTOM_PADDING: f64 = 10.0;
const SELECTED_GLYPH_HEIGHT: f64 = 100.0;

// So that accents don't paint too much over other widgets
const GLYPH_TOP_PADDING: f64 = SELECTED_GLYPH_HEIGHT * 0.2;

pub struct Sidebar {
    selected_glyph: WidgetPod<Workspace, Box<dyn Widget<Workspace>>>,
}

fn selected_glyph_widget() -> impl Widget<GlyphDetail> {
    Flex::column()
        .with_child(
            EditableLabel::new(
                |s: &GlyphName, _: &_| s.to_string(),
                |s| {
                    crate::glyph_names::validate_and_standardize_name(s)
                        .ok()
                        .map(Into::into)
                },
            )
            .controller(RenameController)
            .lens(GlyphDetail::glyph_name),
        )
        .with_child(
            Maybe::new(
                || {
                    Label::dynamic(|d: &char, _| format!("{} (U+{:04X})", d, *d as u32))
                        .with_text_color(theme::SECONDARY_TEXT_COLOR)
                        .with_font(theme::UI_DETAIL_FONT)
                },
                || {
                    Label::new("____")
                        .with_text_color(theme::SECONDARY_TEXT_COLOR)
                        .with_font(theme::UI_DETAIL_FONT)
                },
            )
            .lens(GlyphDetail::codepoint),
        )
        .with_spacer(GLYPH_TOP_PADDING)
        .with_child(GlyphPainter::new().fix_height(SELECTED_GLYPH_HEIGHT))
        .with_child(
            EditableLabel::parse()
                .fix_width(45.)
                .lens(GlyphDetail::advance),
        )
        .with_child(
            Flex::row()
                .with_child(
                    Label::new("kern group")
                        .with_text_color(theme::SECONDARY_TEXT_COLOR)
                        .with_font(theme::UI_DETAIL_FONT),
                )
                .with_flex_spacer(1.0)
                .with_child(
                    Label::new("kern group")
                        .with_text_color(theme::SECONDARY_TEXT_COLOR)
                        .with_font(theme::UI_DETAIL_FONT),
                )
                .padding((8.0, 0.0)),
        )
}

impl Sidebar {
    pub fn new() -> Sidebar {
        Sidebar {
            selected_glyph: WidgetPod::new(
                Maybe::new(
                    || selected_glyph_widget().boxed(),
                    || SizedBox::empty().expand_width().boxed(),
                )
                .lens(Workspace::selected_glyph)
                .background(Color::grey8(0xCC))
                .boxed(),
            ),
        }
    }
}

impl Widget<Workspace> for Sidebar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Workspace, env: &Env) {
        self.selected_glyph.event(ctx, event, data, env)
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Workspace,
        env: &Env,
    ) {
        self.selected_glyph.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &Workspace, data: &Workspace, env: &Env) {
        self.selected_glyph.update(ctx, data, env);
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Workspace,
        env: &Env,
    ) -> Size {
        let child_size = self.selected_glyph.layout(ctx, bc, data, env);
        let my_size = bc.max();
        let extra_y = my_size.height - child_size.height;
        let extra_x = my_size.width - child_size.width;
        let child_y = (extra_y - SELECTED_GLYPH_BOTTOM_PADDING).max(0.0);
        let child_origin = (extra_x / 2.0, child_y);
        let rect = Rect::from_origin_size(child_origin, child_size);
        self.selected_glyph.set_layout_rect(ctx, data, env, rect);
        my_size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Workspace, env: &Env) {
        let rect = ctx.size().to_rect();
        ctx.fill(rect, &env.get(theme::SIDEBAR_BACKGROUND));

        self.selected_glyph.paint(ctx, data, env);

        // to get clean strokes we have to *not* align on pixel boundaries
        let max_x = rect.max_x() - 0.5;
        let line = Line::new((max_x, 0.0), (max_x, rect.max_y()));
        ctx.stroke(line, &env.get(theme::SIDEBAR_EDGE_STROKE), 1.0);
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

/// A simple controller that checks for when our name changes, and then sends
/// a command to rename this glyph everywhere.
struct RenameController;

impl<W: Widget<GlyphName>> Controller<GlyphName, W> for RenameController {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut GlyphName,
        env: &Env,
    ) {
        let pre_data = data.clone();
        child.event(ctx, event, data, env);
        if !pre_data.same(data) {
            let args = crate::consts::cmd::RenameGlyphArgs {
                old: pre_data,
                new: data.clone(),
            };
            let cmd = crate::consts::cmd::RENAME_GLYPH.with(args);
            ctx.submit_command(cmd);
        }
    }
}
