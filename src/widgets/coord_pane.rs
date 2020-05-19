//! The floating panel that displays the coordinate of the currently
//! selected point.

use std::sync::Arc;

use druid::widget::{prelude::*, Flex, Label, SizedBox};
use druid::{Color, LensExt, WidgetExt, WidgetPod};

use crate::design_space::DPoint;
use crate::edit_session::EditSession;
use crate::widgets::{EditableLabel, Maybe};

/// A panel for editing the selected coordinate
pub struct CoordPane {
    inner: WidgetPod<Arc<EditSession>, Box<dyn Widget<Arc<EditSession>>>>,
    current_type: SelectionType,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum SelectionType {
    None,
    Single,
    Multi,
}

impl CoordPane {
    pub fn new() -> Self {
        CoordPane {
            inner: WidgetPod::new(SizedBox::empty().boxed()),
            current_type: SelectionType::None,
        }
    }

    fn rebuild_inner(&mut self, session: &Arc<EditSession>) {
        self.current_type = SelectionType::from_session(session);
        let new_widget = match self.current_type {
            SelectionType::None => SizedBox::empty()
                .width(40.0)
                .height(40.0)
                .background(Color::rgb(1.0, 0.4, 0.7))
                .boxed(),
            SelectionType::Single => single_point_selected()
                .background(Color::rgb(0.4, 1.0, 0.7))
                .boxed(),
            SelectionType::Multi => SizedBox::empty()
                .width(40.0)
                .height(40.0)
                .background(Color::rgb(0.7, 0.2, 1.0))
                .boxed(),
        };
        self.inner = WidgetPod::new(new_widget);
    }
}

impl Widget<Arc<EditSession>> for CoordPane {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Arc<EditSession>, env: &Env) {
        self.inner.event(ctx, event, data, env);
        if matches!(event,Event::MouseUp(_) | Event::MouseDown(_)) {
            ctx.set_handled();
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Arc<EditSession>,
        env: &Env,
    ) {
        if self.current_type.will_change(data) {
            self.rebuild_inner(data);
            ctx.children_changed();
        }
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        _old_data: &Arc<EditSession>,
        data: &Arc<EditSession>,
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
        data: &Arc<EditSession>,
        env: &Env,
    ) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        self.inner.set_layout_rect(ctx, data, env, size.to_rect());
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Arc<EditSession>, env: &Env) {
        self.inner.paint(ctx, data, env);
    }
}

impl SelectionType {
    fn from_session(session: &Arc<EditSession>) -> Self {
        match session.selection.len() {
            0 => Self::None,
            1 => Self::Single,
            _ => Self::Multi,
        }
    }

    fn will_change(self, session: &Arc<EditSession>) -> bool {
        self != Self::from_session(session)
    }
}

fn single_point_selected() -> impl Widget<Arc<EditSession>> {
    Maybe::or_empty(|| {
        Flex::row()
            .with_child(Label::new("x:"))
            .with_spacer(8.0)
            .with_child(EditableLabel::parse().lens(DPoint::x).fix_width(40.0))
            .with_child(Label::new("y:"))
            .with_spacer(8.0)
            .with_child(EditableLabel::parse().lens(DPoint::y).fix_width(40.0))
    })
    .lens(EditSession::single_selection.in_arc())
    .debug_paint_layout()
}

impl Default for CoordPane {
    fn default() -> Self {
        CoordPane::new()
    }
}
