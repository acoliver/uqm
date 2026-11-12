//! Color conversion for graphics-context diagnostics.

use crate::graphics::native_abi::Color;
use std::ffi::c_int;

pub(super) fn hsva_to_rgba(hue: f64, sat: f64, val: f64, alpha: u8) -> Color {
    let hi = (hue / 60.0) as c_int as u32;
    let f = hue / 60.0 - (hue / 60.0) as c_int as f64;
    let p = val * (1.0 - sat);
    let q = val * (1.0 - f * sat);
    let t = val * (1.0 - (1.0 - f * sat));
    let pb = (p * 255.0 + 0.5) as u8;
    let qb = (q * 255.0 + 0.5) as u8;
    let tb = (t * 255.0 + 0.5) as u8;
    let vb = (val * 255.0 + 0.5) as u8;
    assert!((0.0..360.0).contains(&hue));
    assert!((0.0..=1.0).contains(&sat));
    assert!((0.0..=1.0).contains(&val));
    assert!(hi < 6);
    let (r, g, b) = match hi {
        0 => (vb, tb, pb),
        1 => (qb, vb, pb),
        2 => (pb, vb, tb),
        3 => (pb, qb, vb),
        4 => (tb, pb, vb),
        5 => (vb, pb, qb),
        _ => (0, 0, 0),
    };
    Color { r, g, b, a: alpha }
}
