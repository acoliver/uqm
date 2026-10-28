//! Graphics records shared with the battle engine during the storage transition.
pub use super::geometry_ffi::BoxIntersect;
use super::native_abi::{Color, GRAPHICS_PRIM, POINT, PRIM_DESC, PRIM_LINKS, STAMP};

#[repr(C)]
#[derive(Copy, Clone)]
#[allow(non_snake_case)]
pub struct PRIMITIVE {
    pub Links: PRIM_LINKS,
    pub Type: GRAPHICS_PRIM,
    pub color: Color,
    pub Object: PRIM_DESC,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
#[allow(non_camel_case_types, non_snake_case)]
pub struct INTERSECT_CONTROL {
    pub last_time_val: u16,
    pub EndPoint: POINT,
    pub IntersectStamp: STAMP,
}
