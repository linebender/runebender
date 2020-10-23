//! Shared helpers.

use druid::kurbo::{Size, Vec2};

/// could be a size or a vec2 :shrug:
pub(crate) fn compute_scale(pre: Size, post: Size) -> Vec2 {
    let ensure_finite = |f: f64| if f.is_finite() { f } else { 1.0 };
    let x = ensure_finite(post.width / pre.width);
    let y = ensure_finite(post.height / pre.height);
    Vec2::new(x, y)
}
