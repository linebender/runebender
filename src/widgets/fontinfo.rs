//! A widget for editing top-level font info.
//!
//! This is intended to be shown as a modal panel.

use druid::widget::prelude::*;
use druid::widget::{Button, CrossAxisAlignment, Flex, Label};
use druid::{Color, LensExt, LocalizedString, WidgetExt};

use norad::GlyphName;

use crate::data::{FontMetrics, SimpleFontInfo, Workspace};
use crate::widgets::{EditableLabel, ModalHost};

fn glyphname_label() -> EditableLabel<GlyphName> {
    EditableLabel::new(
        |data: &GlyphName, _: &_| data.to_string(),
        |s| Some(s.into()),
    )
}

pub fn font_info() -> impl Widget<Workspace> {
    Flex::column()
        .with_child(
            Flex::row()
                .with_child(glyphname_label().lens(SimpleFontInfo::family_name))
                .with_spacer(8.0)
                .with_child(glyphname_label().lens(SimpleFontInfo::style_name)),
        )
        .with_spacer(8.0)
        .with_child(
            Flex::row()
                .with_child(Label::new(LocalizedString::new("font-info-cap-height").with_placeholder("Cap height:")))
                .with_spacer(8.0)
                .with_child(
                    option_f64_editlabel()
                        .lens(SimpleFontInfo::metrics.then(FontMetrics::cap_height)),
                ),
        )
        .with_child(
            Flex::row()
                .with_child(Label::new(LocalizedString::new("font-info-x-height").with_placeholder("x-height:")))
                .with_spacer(8.0)
                .with_child(
                    option_f64_editlabel()
                        .lens(SimpleFontInfo::metrics.then(FontMetrics::x_height)),
                ),
        )
        .with_child(
            Flex::row()
                .with_child(Label::new(LocalizedString::new("font-info-ascender").with_placeholder("ascender:")))
                .with_spacer(8.0)
                .with_child(
                    option_f64_editlabel()
                        .lens(SimpleFontInfo::metrics.then(FontMetrics::ascender)),
                ),
        )
        .with_child(
            Flex::row()
                .with_child(Label::new(LocalizedString::new("font-info-descender").with_placeholder("descender:")).center())
                .with_spacer(8.0)
                .with_child(
                    option_f64_editlabel()
                        .lens(SimpleFontInfo::metrics.then(FontMetrics::descender)),
                ),
        )
        .with_flex_spacer(1.0)
        .with_child(
            Button::new(LocalizedString::new("button-done").with_placeholder("Done"))
                .on_click(|ctx, _, _| ctx.submit_command(ModalHost::DISMISS_MODAL, None)),
        )
        .cross_axis_alignment(CrossAxisAlignment::End)
        .fix_height(300.)
        .padding(16.0)
        .background(Color::WHITE)
        .lens(Workspace::info)
}

fn option_f64_editlabel() -> EditableLabel<Option<f64>> {
    EditableLabel::new(
        |d: &Option<f64>, _: &_| d.unwrap_or(0.0).to_string(),
        |s| {
            if s.is_empty() {
                Some(None)
            } else {
                match s.parse::<f64>() {
                    Ok(v) if v == 0.0 => Some(None),
                    Ok(other) => Some(Some(other)),
                    Err(_) => None,
                }
            }
        },
    )
}
