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

//! A label that can be edited.
//!
//! This is a bit hacky, and depends on implementation details of other widgets.

use druid::widget::prelude::*;
use druid::widget::{LabelText, TextBox};
use druid::{Color, Data, FontDescriptor, HotKey, KbKey, KeyOrValue, Selector, TextAlignment};

// we send this to ourselves if another widget takes focus, in order
// to validate and move out of editing mode
//const LOST_FOCUS: Selector = Selector::new("druid.builtin.EditableLabel-lost-focus");
const CANCEL_EDITING: Selector = Selector::new("druid.builtin.EditableLabel-cancel-editing");
const COMPLETE_EDITING: Selector = Selector::new("druid.builtin.EditableLabel-complete-editing");

/// A label with text that can be edited.
///
/// Edits are not applied to the data until editing finishes, usually when the
/// user presses <return>. If the new text generates a valid value, it is set;
/// otherwise editing continues.
///
/// Editing can be abandoned by pressing <esc>.
pub struct EditableLabel<T> {
    label: LabelText<T>,
    old_buffer: String,
    buffer: String,
    editing: bool,
    text_box: TextBox<String>,
    on_completion: Box<dyn Fn(&str) -> Option<T>>,
}

impl<T: Data + std::fmt::Display + std::str::FromStr> EditableLabel<T> {
    /// Create a new `EditableLabel` that uses `to_string` to display a value and
    /// `FromStr` to validate the input.
    pub fn parse() -> Self {
        Self::new(|data: &T, _: &_| data.to_string(), |s| s.parse().ok())
    }
}

impl<T: Data> EditableLabel<T> {
    /// Create a new `EditableLabel`.
    ///
    /// The first argument creates a label; it should probably be a dynamic
    /// or localized string.
    ///
    /// The second argument is a closure used to compute the data from the
    /// contents of the string. This is called when the user presses return,
    /// or otherwise tries to navigate away from the label; if it returns
    /// `Some<T>` then that is set as the new data, and the edit ends. If it
    /// returns `None`, then the edit continues.
    pub fn new(
        text: impl Into<LabelText<T>>,
        on_completion: impl Fn(&str) -> Option<T> + 'static,
    ) -> Self {
        EditableLabel {
            label: text.into(),
            buffer: String::new(),
            old_buffer: String::new(),
            text_box: TextBox::new(),
            editing: false,
            on_completion: Box::new(on_completion),
        }
    }

    /// Builder-style method to set the placeholder text.
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.text_box = self.text_box.with_placeholder(text);
        self
    }

    /// Builder-style method for setting the font.
    ///
    /// The argument can be a [`FontDescriptor`] or a [`Key<FontDescriptor>`]
    /// that refers to a font defined in the [`Env`].
    ///
    /// [`Env`]: ../struct.Env.html
    /// [`FontDescriptor`]: ../struct.FontDescriptor.html
    /// [`Key<FontDescriptor>`]: ../struct.Key.html
    pub fn with_font(mut self, font: impl Into<KeyOrValue<FontDescriptor>>) -> Self {
        self.text_box.set_font(font);
        self
    }

    /// Builder-style method to override the text size.
    pub fn with_text_size(mut self, size: f64) -> Self {
        self.text_box.set_text_size(size);
        self
    }

    /// Builder-style method to set  the text color.
    pub fn with_text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.text_box.set_text_color(color);
        self
    }

    /// Builder-style method to set the [`TextAlignment`].
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.text_box.set_text_alignment(alignment);
        self
    }

    fn complete(&mut self, ctx: &mut EventCtx, data: &mut T) {
        if let Some(new) = (self.on_completion)(&self.buffer) {
            *data = new;
            self.editing = false;
            ctx.request_layout();
            if ctx.has_focus() {
                ctx.resign_focus();
            }
        } else {
            // don't tab away from here if we're editing
            if !ctx.has_focus() {
                ctx.request_focus();
            }
            // ideally we would flash the background or something
        }
    }

    fn cancel(&mut self, ctx: &mut EventCtx) {
        self.editing = false;
        ctx.request_layout();
        ctx.resign_focus();
    }

    fn begin(&mut self, ctx: &mut EventCtx) {
        self.editing = true;
        ctx.request_layout();
    }
}

impl<T: Data> Widget<T> for EditableLabel<T> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if self.editing {
            match event {
                Event::Command(cmd) if cmd.is(COMPLETE_EDITING) => self.complete(ctx, data),
                Event::Command(cmd) if cmd.is(CANCEL_EDITING) => self.cancel(ctx),
                Event::KeyDown(k_e) if HotKey::new(None, KbKey::Enter).matches(k_e) => {
                    ctx.set_handled();
                    self.complete(ctx, data);
                }
                Event::KeyDown(k_e) if HotKey::new(None, KbKey::Escape).matches(k_e) => {
                    ctx.set_handled();
                    self.cancel(ctx);
                }
                event => {
                    self.text_box.event(ctx, event, &mut self.buffer, env);
                    ctx.request_paint();
                }
            }
            ctx.request_update();
        } else if let Event::MouseDown(_) = event {
            self.begin(ctx);
            self.text_box.event(ctx, event, &mut self.buffer, env);
        }
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.label.resolve(data, env);
            self.buffer = self.label.display_text().to_string();
            self.old_buffer = self.buffer.clone();
        }
        self.text_box.lifecycle(ctx, event, &self.buffer, env);

        if let LifeCycle::FocusChanged(focus) = event {
            // if the user focuses elsewhere, we need to reset ourselves
            if !focus {
                ctx.submit_command(COMPLETE_EDITING.to(ctx.widget_id()));
            } else if !self.editing {
                self.editing = true;
                self.label.resolve(data, env);
                self.buffer = self.label.display_text().to_string();
                ctx.request_layout();
            }
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        // if we're editing, the only data changes possible are from external
        // sources, since we don't mutate the data until editing completes;
        // so in this case, we want to use the new data, and cancel editing.
        if !data.same(old_data) {
            ctx.submit_command(CANCEL_EDITING.to(ctx.widget_id()));
        }

        let in_edit_mode = self.editing && data.same(old_data);
        if in_edit_mode {
            self.text_box
                .update(ctx, &self.old_buffer, &self.buffer, env);
        } else {
            self.label.resolve(data, env);
            let data_changed = self.label.display_text().as_ref() != self.buffer.as_str();
            if data_changed {
                let new_text = self.label.display_text().to_string();
                self.text_box.update(ctx, &self.buffer, &new_text, env);
                self.old_buffer = std::mem::replace(&mut self.buffer, new_text);
                ctx.request_layout();
            } else if ctx.env_changed() {
                self.text_box.update(ctx, &self.buffer, &self.buffer, env);
                ctx.request_layout();
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &T, env: &Env) -> Size {
        self.text_box.layout(ctx, bc, &self.buffer, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, _data: &T, env: &Env) {
        //FIXME: we want to paint differently when we aren't editing
        //if self.editing {
        //self.text_box.paint(ctx, &self.buffer, env);
        //} else {
        self.text_box.paint(ctx, &self.buffer, env);
        //}
    }
}
