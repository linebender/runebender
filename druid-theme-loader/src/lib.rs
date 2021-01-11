//! Loading themes from files.
//!
//! This module consists of two parts: A macro for declaring 'themes'
//! (a typed collection of keys and values) and a widget for loading declared
//! themes from files on disk.
//!
//! ## File format
//!
//! Currently, this module uses a custom text format for themes. This format
//! is simple: each non-blank line must contain a key/value pair, separated
//! by a colon. You can currently declare either floats or colors. If this
//! idea is adopted, we may want to find a better format, as this one will
//! not scale very well as we add support for more value types.
//!
//! ## Value types:
//!
//! - **Colors** must be hex strings in one of the following formats: `rgb`,
//! `rgba`, `rrggbb`, `rrggbbaa`, with our without a leading `#`.
//! - **Floats** must be a string that can be parsed by `f64::from_str`.
//!
//! ## Example file
//!
//! ```text
//! BACKGROUND_COLOR:   #fda
//! TEXT_COLOR:         #121121
//! TITLE_PADDING:      16
//! ```
//!
//! ## Live reloading
//!
//! If you would like your app to update the theme when it is edited, you need
//! to enable the `notify` feature of this crate in your manifest.

mod parse;
mod widget;
pub use widget::ThemeLoader;

use druid::Env;

type RawTheme<'a> = std::collections::HashMap<&'a str, &'a str>;

/// A macro for declaring a set of [`Key`]s that can be loaded from a file.
///
/// This macro is used alongside the [`ThemeLoader`] widget in order to load
/// a theme from a file, and to optionally reload the theme when that file changes.
///
/// To use this, you must first declare all of the [`Key`]s that your theme
/// will use, with their correct types. All of these keys must be present and
/// in the theme file, and have the correct values.
///
/// After using this macro to generate a struct that implements the [`LoadableTheme`]
/// trait, you can pass that struct, along with a path to the theme file, to
/// the [`ThemeLoader`] widget. This should sit at the base of your widget tree,
/// and will insert the values from the file into the [`Env`].
///
/// Note: currently this only supports [`Color`] and `f64` values.
///
/// # Examples
///
/// ```
/// use druid::{Color, Key, Data, Widget};
/// use druid_theme_loader::{loadable_theme, ThemeLoader};
///
/// // first declare your theme keys;
/// pub const BACKGROUND_COLOR: Key<Color> = Key::new("druid.nursery.theme.bg-color");
/// pub const TEXT_COLOR: Key<Color> = Key::new("druid.nursery.theme.text-color");
/// pub const TITLE_PADDING: Key<f64> = Key::new("druid.nursery.theme.title-padding");
///
///
/// // declares a new struct, MyTheme.
/// loadable_theme!(pub MyTheme {
///     BACKGROUND_COLOR,
///     TEXT_COLOR,
///     TITLE_PADDING
/// });
///
/// // once you have declared a theme, you can use it with the ThemeLoader widget
/// fn themed_widget<T: Data>(w: impl Widget<T>) -> impl Widget<T> {
///     ThemeLoader::new("./themes/my_theme.txt", MyTheme, w)
/// }
/// ```
///
/// [`Key`]: druid::Key
/// [`Color`]: druid::Color
#[macro_export]
macro_rules! loadable_theme {
    ($vis:vis $ty:ident { $($key:ident),+ } ) => {

        $vis struct $ty;

        impl $crate::LoadableTheme for $ty {
            fn load(&self, raw: &std::collections::HashMap<&str, &str>, current: &druid::Env) -> Result<druid::Env, $crate::ThemeLoadError> {
                use std::any::TypeId;
                use druid::{Color, Value};
                use $crate::{ThemeLoadError, LoadableTheme, ValueKind};

                fn get_kind<T: 'static>(_k: &druid::Key<T>) -> Result<ValueKind, ThemeLoadError> {
                    let this_type = TypeId::of::<T>();
                    if this_type  == TypeId::of::<Color>() {
                        Ok(ValueKind::Color)
                    } else if this_type == TypeId::of::<f64>() {
                        Ok(ValueKind::Float)
                    } else {
                        Err(ThemeLoadError::UnknownType(std::any::type_name::<T>()))
                    }
                }

                let mut new_env = current.clone();
                let mut expected_keys = std::collections::HashSet::with_capacity(raw.len());
                // TODO: it would be nice to also verify that all keys have unique identifiers?
                // this requires https://github.com/linebender/druid/pull/1527

                $(
                let kind = get_kind(&$key)?;
                let key_ident = stringify!($key);
                let val = raw.get(key_ident).ok_or_else(|| ThemeLoadError::MissingKey(key_ident))?;
                let val: Value = match kind {
                    ValueKind::Color => {
                        Color::from_hex_str(val).map(Into::into).map_err(ThemeLoadError::ParseColorError)
                    }
                    ValueKind::Float => {
                        val.parse::<f64>().map(Into::into).map_err(ThemeLoadError::ParseFloatError)
                    }
                }?;
                new_env.try_set_raw($key, val).map_err(ThemeLoadError::ValueTypeError)?;
                expected_keys.insert(key_ident);
                )+

                let unexpected_keys = raw.keys().filter(|k| !expected_keys.contains(*k))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                if unexpected_keys.is_empty() {
                    Ok(new_env)
                } else {
                    Err(ThemeLoadError::UnexpectedKeys(unexpected_keys))
                }
            }
        }
    };
    // also work with a trailing comma
    ($vis:vis $ty:ident { $($key:ident),+, } ) => {
        $crate::loadable_theme!($vis $ty { $( $key  ),+ });
    };
}

/// A trait for a theme that can be loaded from disk.
///
/// This is essentially a schema.
///
/// You should not implement this trait directly, but rather should use the
/// [`loadable_theme`] macro to generate it.
pub trait LoadableTheme {
    fn load(&self, raw: &RawTheme, current: &Env) -> Result<Env, ThemeLoadError>;
}

/// A type for errors that occur when loading a theme from file.
#[derive(Debug)]
pub enum ThemeLoadError {
    IoError(std::io::Error),
    UnknownType(&'static str),
    MissingKey(&'static str),
    ParseColorError(druid::piet::ColorParseError),
    ParseFloatError(std::num::ParseFloatError),
    ValueTypeError(druid::ValueTypeError),
    UnexpectedKeys(Vec<String>),
    ParseThemeLineError(String),
}

impl std::fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "io error loading theme: '{}'", err),
            Self::UnknownType(t) => write!(f, "Unsupported theme key type '{}'", t),
            Self::MissingKey(k) => write!(f, "Theme is missing expected key '{}'", k),
            Self::ParseColorError(e) => write!(f, "Theme failed to parse color: '{}'", e),
            Self::ParseFloatError(e) => write!(f, "Theme failed to parse float: '{}'", e),
            Self::ValueTypeError(e) => write!(f, "Theme value type mismatch: '{}'", e),
            Self::UnexpectedKeys(keys) => {
                write!(f, "Theme file contained undeclared keys: {:?}", keys)
            }
            Self::ParseThemeLineError(s) => {
                write!(f, "Theme contained malformed line: '{}'", s.escape_debug())
            }
        }
    }
}

impl std::error::Error for ThemeLoadError {}

impl From<std::io::Error> for ThemeLoadError {
    fn from(src: std::io::Error) -> ThemeLoadError {
        ThemeLoadError::IoError(src)
    }
}
/// An enum representing the kinds of the types in [`Value`].
///
/// This is used for things like theme loading, where you want a homogenous
/// type that represents all of the keys you want to load, but don't know
/// the actual values yet, pending validation.
///
/// [`Value`]: druid::Value
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueKind {
    Color,
    Float,
    //TODO: add more types as needed
    //Point,
    //Size,
    //Rect,
    //Insets,
    //Bool,
    //UnsignedInt,
    //String,
    //Font,
}
