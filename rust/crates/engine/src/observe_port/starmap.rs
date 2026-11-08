//! Star-map coordinate values copied at an input boundary.
use crate::graphics::native_abi::POINT;
use std::ffi::c_long;

pub fn signed_div_with_error(mut value: c_long, divisor: c_long) -> c_long {
    let invert = value < 0;
    if invert {
        value = -value;
    }
    value = (value + (divisor >> 1)) / divisor;
    if invert {
        -value
    } else {
        value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Projection {
    pub origin: POINT,
    pub zoom: i32,
    pub width: i32,
    pub height: i32,
}
impl Projection {
    pub fn x(self, ux: c_long) -> i16 {
        let fit = 10000 / self.width + 1;
        (signed_div_with_error(
            ((ux - c_long::from(self.origin.x)) << self.zoom) * c_long::from(self.width),
            c_long::from(9999 + fit),
        ) + c_long::from((self.width - 1) >> 1)) as i16
    }
    pub fn y(self, uy: c_long) -> i16 {
        (signed_div_with_error(
            ((c_long::from(self.origin.y) - uy) << self.zoom) * c_long::from(self.height),
            10001,
        ) + c_long::from((self.height - 1) >> 1)) as i16
    }
    pub fn inverse_x(self, dx: i16) -> i16 {
        let fit = 10000 / self.width + 1;
        ((((c_long::from(dx) - c_long::from((self.width - 1) >> 1)) * c_long::from(9999 + fit))
            >> self.zoom)
            / c_long::from(self.width)
            + c_long::from(self.origin.x)) as i16
    }
    pub fn inverse_y(self, dy: i16) -> i16 {
        ((((c_long::from((self.height - 1) >> 1) - c_long::from(dy)) * 10001) >> self.zoom)
            / c_long::from(self.height)
            + c_long::from(self.origin.y)) as i16
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StarMapView {
    pub cursor: POINT,
    pub projection: Projection,
}
