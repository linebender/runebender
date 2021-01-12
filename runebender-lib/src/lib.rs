//! The core library of the runebender font editor.

#![allow(clippy::rc_buffer)]

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lopdf;

mod app_delegate;
mod bez_cache;
mod clipboard;
mod component;
mod consts;
mod design_space;
mod draw;
mod edit_session;
mod glyph_names;
mod guides;
mod path;
mod plist;
mod point;
mod quadrant;
mod selection;
mod tools;
mod undo;
mod util;

pub mod data;
pub mod menus;
pub mod mouse;
pub mod theme;
pub mod widgets;

pub use app_delegate::Delegate;
pub use util::create_blank_font;
