//! Shared helpers.

use std::convert::TryFrom;

use druid::kurbo::{Size, Vec2};

/// Unwrap an optional, printing a message and returning if it is missing.
///
/// This should generate less code than unwrap? Honestly it's a total
/// experiment.
macro_rules! bail {
    ($opt:expr $(,)?) => {
        match $opt {
            Some(val) => val,
            None => {
                eprintln!("[{}:{}] bailed", file!(), line!());
                return
            }
        }
    };
     ($opt:expr, $($arg:tt)+) => {
        match $opt {
            Some(val) => val,
            None => {
                eprintln!("[{}:{}] bailed: ", file!(), line!());
                eprintln!($($arg)+);
                return
            }
        }
    };
}

/// could be a size or a vec2 :shrug:
pub(crate) fn compute_scale(pre: Size, post: Size) -> Vec2 {
    let ensure_finite = |f: f64| if f.is_finite() { f } else { 1.0 };
    let x = ensure_finite(post.width / pre.width);
    let y = ensure_finite(post.height / pre.height);
    Vec2::new(x, y)
}

/// temporary; creates a new blank  font with some placeholder glyphs.
pub fn create_blank_font() -> norad::Ufo {
    let mut ufo = norad::Ufo::new();
    ufo.font_info = norad::FontInfo {
        family_name: Some("Untitled".into()),
        style_name: Some("Regular".into()),
        units_per_em: Some(TryFrom::try_from(1000.0f64).unwrap()),
        descender: Some(From::from(-200.0)),
        ascender: Some(800.0.into()),
        cap_height: Some(700.0.into()),
        x_height: Some(500.0.into()),
        ..Default::default()
    }
    .into();

    let layer = ufo.get_default_layer_mut().unwrap();
    ('a'..='z')
        .into_iter()
        .chain('A'..='Z')
        .map(|chr| {
            let mut glyph = norad::Glyph::new_named(chr.to_string());
            glyph.codepoints = Some(vec![chr]);
            glyph
        })
        .for_each(|glyph| layer.insert_glyph(glyph));
    ufo
}
