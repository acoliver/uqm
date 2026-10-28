//! Graphics records shared with the battle engine during the storage transition.
pub use super::geometry_ffi::BoxIntersect;
use super::native_abi::{Color, GRAPHICS_PRIM, POINT, PRIM_DESC, PRIM_LINKS, STAMP};

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PRIMITIVE {
    #[allow(non_snake_case)]
    pub Links: PRIM_LINKS,
    #[allow(non_snake_case)]
    pub Type: GRAPHICS_PRIM,
    pub color: Color,
    #[allow(non_snake_case)]
    pub Object: PRIM_DESC,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IntersectControl {
    pub last_time_val: u16,
    #[allow(non_snake_case)]
    pub EndPoint: POINT,
    #[allow(non_snake_case)]
    pub IntersectStamp: STAMP,
}

pub use IntersectControl as INTERSECT_CONTROL;
