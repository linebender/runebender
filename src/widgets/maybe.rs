// Copyright 2020 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A widget for optional data, with different `Some` and `None` children.

use druid::{
    BoxConstraints, Data, Env, Event, EventCtx, LayoutCtx, LifeCycle, LifeCycleCtx, PaintCtx, Size,
    UpdateCtx, Widget, WidgetExt, WidgetPod,
};

use druid::widget::SizedBox;

/// A widget that switches between two possible child views, for `Data` that
/// is `Option<T>`.
pub struct Maybe<T> {
    some_maker: Box<dyn Fn() -> Box<dyn Widget<T>>>,
    none_maker: Box<dyn Fn() -> Box<dyn Widget<()>>>,
    widget: MaybeWidget<T>,
}

enum MaybeWidget<T> {
    Some(WidgetPod<T, Box<dyn Widget<T>>>),
    None(WidgetPod<(), Box<dyn Widget<()>>>),
}

impl<T: Data> Maybe<T> {
    /// Create a new `Maybe` widget with a `Some` and a `None` branch.
    pub fn new<W1, W2>(
        // we make these generic so that the caller doesn't have to explicitly
        // box. We don't technically *need* to box, but it seems simpler.
        some_maker: impl Fn() -> W1 + 'static,
        none_maker: impl Fn() -> W2 + 'static,
    ) -> Maybe<T>
    where
        W1: Widget<T> + 'static,
        W2: Widget<()> + 'static,
    {
        let widget = MaybeWidget::Some(WidgetPod::new(some_maker().boxed()));
        Maybe {
            some_maker: Box::new(move || some_maker().boxed()),
            none_maker: Box::new(move || none_maker().boxed()),
            widget,
        }
    }

    /// Create a new `Maybe` widget where the `None` branch is an empty widget.
    #[allow(dead_code)]
    pub fn or_empty<W1: Widget<T> + 'static>(some_maker: impl Fn() -> W1 + 'static) -> Maybe<T> {
        Self::new(some_maker, || SizedBox::empty())
    }

    fn rebuild_widget(&mut self, is_some: bool) {
        if is_some {
            self.widget = MaybeWidget::Some(WidgetPod::new((self.some_maker)()));
        } else {
            self.widget = MaybeWidget::None(WidgetPod::new((self.none_maker)()));
        }
    }
}

impl<T: Data> Widget<Option<T>> for Maybe<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut Option<T>, env: &Env) {
        match data.as_mut() {
            Some(d) => self.widget.unwrap_some().event(ctx, event, d, env),
            None => self.widget.unwrap_none().event(ctx, event, &mut (), env),
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Option<T>,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = event {
            if data.is_some() != self.widget.is_some() {
                // only possible at launch, because we default to `Some`
                self.rebuild_widget(data.is_some());
            }
        }
        match data.as_ref() {
            Some(d) => self.widget.unwrap_some().lifecycle(ctx, event, d, env),
            None => self
                .widget
                .unwrap_none()
                .lifecycle(ctx, event, &mut (), env),
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &Option<T>, data: &Option<T>, env: &Env) {
        if old_data.is_some() != data.is_some() {
            self.rebuild_widget(data.is_some());
            ctx.children_changed();
        } else {
            match data {
                Some(new) => self.widget.unwrap_some().update(ctx, new, env),
                None => self.widget.unwrap_none().update(ctx, &(), env),
            }
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &Option<T>,
        env: &Env,
    ) -> Size {
        match data.as_ref() {
            Some(d) => {
                let size = self.widget.unwrap_some().layout(ctx, bc, d, env);
                self.widget.unwrap_some().set_layout_rect(size.to_rect());
                size
            }
            None => {
                let size = self.widget.unwrap_none().layout(ctx, bc, &(), env);
                self.widget.unwrap_none().set_layout_rect(size.to_rect());
                size
            }
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &Option<T>, env: &Env) {
        match data.as_ref() {
            Some(d) => self.widget.unwrap_some().paint(ctx, d, env),
            None => self.widget.unwrap_none().paint(ctx, &(), env),
        }
    }
}

impl<T> MaybeWidget<T> {
    fn is_some(&self) -> bool {
        match self {
            Self::Some(_) => true,
            Self::None(_) => false,
        }
    }

    fn unwrap_some(&mut self) -> &mut WidgetPod<T, Box<dyn Widget<T>>> {
        match self {
            Self::Some(widget) => widget,
            Self::None(_) => panic!("Called MaybeWidget::unwrap_some on a `None` value."),
        }
    }

    fn unwrap_none(&mut self) -> &mut WidgetPod<(), Box<dyn Widget<()>>> {
        match self {
            Self::None(widget) => widget,
            Self::Some(_) => panic!("Called MaybeWidget::unwrap_none on a `Some` value."),
        }
    }
}
