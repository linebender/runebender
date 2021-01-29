//! Easier handling of mouse events.
//!
//! Handling the various permutations and combinations of mouse events is messy,
//! repetitive, and error prone.
//!
//! This module implements a state machine that handles the raw event processing,
//! identifying important events and transitions.
//!
//! # Use
//!
//! The state machine itself is exposed via the [`Mouse`] struct. You are
//! responsible for instantiating this struct, and it is expected to persist
//! between mouse events.
//!
//! To react to state changes, you must implement the [`MouseDelegate`] trait;
//! you only need to implement the methods you are interested in.
//!
//! When a mouse event arrives, you pass the event along with your delegate to
//! the [`Mouse`] struct; if that event causes a state change, the corresponding
//! method on your delegate will be called.
//!
//! # Example
//! ```
//! use runebender_lib::mouse::{Mouse, MouseDelegate};
//! use druid::{Modifiers, MouseEvent, MouseButton, MouseButtons, Point, Vec2};
//!
//! struct SimpleDelegate(usize);
//!
//! impl MouseDelegate<()> for SimpleDelegate {
//!     fn left_click(&mut self, _event: &MouseEvent, _data: &mut ()) {
//!         self.0 += 1;
//!     }
//!
//!     fn cancel(&mut self, _: &mut ()) {}
//! }
//!
//! let event = MouseEvent {
//!     pos: Point::new(20., 20.,),
//!     window_pos: Point::new(20., 20.,),
//!     mods: Modifiers::empty(),
//!     count: 1,
//!     button: MouseButton::Left,
//!     buttons: MouseButtons::new().with(MouseButton::Left),
//!     focus: false,
//!     wheel_delta: Vec2::ZERO,
//! };
//!
//! let mut mouse = Mouse::default();
//! let mut delegate = SimpleDelegate(0);
//! mouse.mouse_down(event.clone(), &mut (), &mut delegate);
//! assert_eq!(delegate.0, 0);
//! mouse.mouse_up(event.clone(), &mut (), &mut delegate);
//! assert_eq!(delegate.0, 1);
//! ```

use druid::kurbo::Point;
use druid::{Modifiers, MouseButton, MouseButtons, MouseEvent};
use std::mem;

const DEFAULT_MIN_DRAG_DISTANCE: f64 = 2.0;

/// Handles raw mouse events, parsing them into gestures that it forwards
/// to a delegate.
#[derive(Debug, Clone)]
pub struct Mouse {
    state: MouseState,
    /// The distance the mouse must travel with a button down for it to
    /// be considered a drag gesture.
    pub min_drag_distance: f64,
}

/// A convenience type for passing around mouse events while keeping track
/// of the event type.
#[derive(Debug, Clone)]
pub enum TaggedEvent {
    Down(MouseEvent),
    Up(MouseEvent),
    Moved(MouseEvent),
}

#[derive(Debug, Clone)]
enum MouseState {
    /// No mouse buttons are active.
    Up(MouseEvent),
    /// A mouse button has been pressed.
    Down(MouseEvent),
    /// The mouse has been moved some threshold distance with a button pressed.
    Drag {
        start: MouseEvent,
        current: MouseEvent,
    },
    /// A state only used as a placeholder during event handling.
    #[doc(hidden)]
    Transition,
}

/// The state of an in-progress drag gesture.
#[derive(Debug, Clone, Copy)]
#[allow(unused)]
pub struct Drag<'a> {
    /// The event that started this drag
    pub start: &'a MouseEvent,
    /// The previous event in this drag
    pub prev: &'a MouseEvent,
    /// The current event in this drag
    pub current: &'a MouseEvent,
}

/// A trait for types that want fine grained information about mouse events.
pub trait MouseDelegate<T> {
    /// Called on any mouse movement.
    #[allow(unused)]
    fn mouse_moved(&mut self, _event: &MouseEvent, _data: &mut T) {}

    /// Called when the left mouse button is pressed.
    #[allow(unused)]
    fn left_down(&mut self, _event: &MouseEvent, _data: &mut T) {}
    /// Called when the left mouse button is released.
    #[allow(unused)]
    fn left_up(&mut self, _event: &MouseEvent, _data: &mut T) {}
    /// Called when the left mouse button is released, if there has not already
    /// been a drag event.
    #[allow(unused)]
    fn left_click(&mut self, _event: &MouseEvent, _data: &mut T) {}

    /// Called when the mouse moves a minimum distance with the left mouse
    /// button pressed.
    #[allow(unused)]
    fn left_drag_began(&mut self, _drag: Drag, _data: &mut T) {}
    /// Called when the mouse moves after a drag gesture has been recognized.
    #[allow(unused)]
    fn left_drag_changed(&mut self, _drag: Drag, _data: &mut T) {}
    /// Called when the left mouse button is released, after a drag gesture
    /// has been recognized.
    #[allow(unused)]
    fn left_drag_ended(&mut self, _drag: Drag, _data: &mut T) {}

    /// Called when the right mouse button is pressed.
    #[allow(unused)]
    fn right_down(&mut self, _event: &MouseEvent, _data: &mut T) {}
    /// Called when the right mouse button is released.
    #[allow(unused)]
    fn right_up(&mut self, _event: &MouseEvent, _data: &mut T) {}
    /// Called when the right mouse button is released, if there has not already
    /// been a drag event.
    #[allow(unused)]
    fn right_click(&mut self, _event: &MouseEvent, _data: &mut T) {}

    /// Called when the mouse moves a minimum distance with the right mouse
    /// button pressed.
    #[allow(unused)]
    fn right_drag_began(&mut self, _drag: Drag, _data: &mut T) {}
    /// Called when the mouse moves after a drag gesture has been recognized.
    #[allow(unused)]
    fn right_drag_changed(&mut self, _drag: Drag, _data: &mut T) {}
    /// Called when the right mouse button is released, after a drag gesture
    /// has been recognized.
    #[allow(unused)]
    fn right_drag_ended(&mut self, _drag: Drag, _data: &mut T) {}

    #[allow(unused)]
    fn other_down(&mut self, _event: &MouseEvent, _data: &mut T) {}
    #[allow(unused)]
    fn other_up(&mut self, _event: &MouseEvent, _data: &mut T) {}
    #[allow(unused)]
    fn other_click(&mut self, _event: &MouseEvent, _data: &mut T) {}

    #[allow(unused)]
    fn other_drag_began(&mut self, _drag: Drag, _data: &mut T) {}
    #[allow(unused)]
    fn other_drag_changed(&mut self, _drag: Drag, _data: &mut T) {}
    #[allow(unused)]
    fn other_drag_ended(&mut self, _drag: Drag, _data: &mut T) {}

    #[allow(unused)]
    fn cancel(&mut self, data: &mut T);
}

impl std::default::Default for Mouse {
    fn default() -> Mouse {
        Mouse {
            min_drag_distance: DEFAULT_MIN_DRAG_DISTANCE,
            state: MouseState::Up(MouseEvent {
                pos: Point::ZERO,
                window_pos: Point::ZERO,
                mods: Modifiers::default(),
                count: 0,
                button: MouseButton::None,
                buttons: MouseButtons::default(),
                focus: false,
                wheel_delta: Default::default(),
            }),
        }
    }
}

impl TaggedEvent {
    pub fn inner(&self) -> &MouseEvent {
        match self {
            TaggedEvent::Down(m) => m,
            TaggedEvent::Up(m) => m,
            TaggedEvent::Moved(m) => m,
        }
    }
}

impl Mouse {
    /// reset any settable internal state to its default value.
    pub fn reset(&mut self) {
        self.min_drag_distance = DEFAULT_MIN_DRAG_DISTANCE;
    }

    /// The current position of  the mouse.
    #[allow(dead_code)]
    pub fn pos(&self) -> Point {
        match &self.state {
            MouseState::Up(e) => e.pos,
            MouseState::Down(e) => e.pos,
            MouseState::Drag { current, .. } => current.pos,
            _ => panic!("transition is not an actual state :/"),
        }
    }

    pub fn mouse_event<T>(
        &mut self,
        event: TaggedEvent,
        data: &mut T,
        delegate: &mut dyn MouseDelegate<T>,
    ) {
        match event {
            TaggedEvent::Up(event) => self.mouse_up(event, data, delegate),
            TaggedEvent::Down(event) => self.mouse_down(event, data, delegate),
            TaggedEvent::Moved(event) => self.mouse_moved(event, data, delegate),
        }
    }

    pub fn mouse_moved<T>(
        &mut self,
        event: MouseEvent,
        data: &mut T,
        delegate: &mut dyn MouseDelegate<T>,
    ) {
        let prev_state = mem::replace(&mut self.state, MouseState::Transition);
        self.state = match prev_state {
            MouseState::Up(_) => {
                delegate.mouse_moved(&event, data);
                MouseState::Up(event)
            }
            MouseState::Down(prev) => {
                if prev.pos.distance(event.pos) > self.min_drag_distance {
                    let drag = Drag::new(&prev, &prev, &event);
                    if prev.button.is_left() {
                        delegate.left_drag_began(drag, data)
                    } else if prev.button.is_right() {
                        delegate.right_drag_began(drag, data)
                    } else {
                        delegate.other_drag_began(drag, data)
                    };
                    MouseState::Drag {
                        start: prev,
                        current: event,
                    }
                } else {
                    MouseState::Down(prev)
                }
            }
            MouseState::Drag { start, current } => {
                let drag = Drag::new(&start, &current, &event);
                if start.button.is_left() {
                    delegate.left_drag_changed(drag, data)
                } else if start.button.is_right() {
                    delegate.right_drag_changed(drag, data)
                } else {
                    delegate.other_drag_changed(drag, data)
                };
                MouseState::Drag {
                    start,
                    current: event,
                }
            }
            MouseState::Transition => panic!("ahhhhhhh"),
        };
    }

    pub fn mouse_down<T>(
        &mut self,
        event: MouseEvent,
        data: &mut T,
        delegate: &mut dyn MouseDelegate<T>,
    ) {
        let prev_state = mem::replace(&mut self.state, MouseState::Transition);
        self.state = match prev_state {
            MouseState::Up(_) => {
                if event.button.is_left() {
                    delegate.left_down(&event, data)
                } else if event.button.is_right() {
                    delegate.right_down(&event, data)
                } else {
                    delegate.other_down(&event, data)
                };
                MouseState::Down(event)
            }
            MouseState::Down(prev) => {
                assert!(prev.button != event.button);
                // if a second button is pressed while we're handling an event
                // we just ignore it. At some point we could consider an event for this.
                MouseState::Down(prev)
            }
            MouseState::Drag { start, .. } => {
                if start.button != event.button {
                    log::warn!("mouse click while drag in progress; not correctly receiving mouse up events?");
                }
                MouseState::Drag {
                    start,
                    current: event,
                }
            }
            MouseState::Transition => panic!("ahhhhhhh"),
        };
    }

    pub fn mouse_up<T>(
        &mut self,
        event: MouseEvent,
        data: &mut T,
        delegate: &mut dyn MouseDelegate<T>,
    ) {
        let prev_state = mem::replace(&mut self.state, MouseState::Transition);
        self.state = match prev_state {
            MouseState::Up(_) => MouseState::Up(event),
            MouseState::Down(prev) => {
                if event.button == prev.button {
                    if prev.button.is_left() {
                        delegate.left_click(&event, data);
                        delegate.left_up(&event, data);
                    } else if prev.button.is_right() {
                        delegate.right_click(&event, data);
                        delegate.right_up(&event, data);
                    } else {
                        delegate.other_click(&event, data);
                        delegate.other_up(&event, data);
                    };
                    MouseState::Up(event)
                } else {
                    MouseState::Down(prev)
                }
            }
            MouseState::Drag { start, current } => {
                if event.button == start.button {
                    let drag = Drag {
                        start: &start,
                        current: &event,
                        prev: &current,
                    };
                    if start.button.is_left() {
                        delegate.left_drag_ended(drag, data);
                        delegate.left_up(&event, data);
                    } else if start.button.is_right() {
                        delegate.right_drag_ended(drag, data);
                        delegate.right_up(&event, data);
                    } else {
                        delegate.other_drag_ended(drag, data);
                        delegate.other_up(&event, data);
                    };
                    MouseState::Up(event)
                } else {
                    MouseState::Drag { start, current }
                }
            }
            MouseState::Transition => panic!("ahhhhhhh"),
        };
    }

    pub fn cancel<T>(&mut self, data: &mut T, delegate: &mut dyn MouseDelegate<T>) {
        let prev_state = mem::replace(&mut self.state, MouseState::Transition);
        let last_event = match prev_state {
            MouseState::Down(event) => event,
            MouseState::Up(event) => event,
            MouseState::Drag { current, .. } => current,
            MouseState::Transition => panic!("ahhhhhhh"),
        };
        delegate.cancel(data);
        self.state = MouseState::Up(last_event);
    }
}

impl<'a> Drag<'a> {
    fn new(start: &'a MouseEvent, prev: &'a MouseEvent, current: &'a MouseEvent) -> Drag<'a> {
        Drag {
            start,
            prev,
            current,
        }
    }
}
