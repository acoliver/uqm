//! Graphics records shared with the battle engine during the storage transition.
pub use super::geometry_ffi::BoxIntersect;
use super::native_abi::{Color, GRAPHICS_PRIM, POINT, PRIM_DESC, PRIM_LINKS, STAMP};

#[repr(C)]
#[allow(non_snake_case)]
pub struct PRIMITIVE {
    pub Links: PRIM_LINKS,
    pub Type: GRAPHICS_PRIM,
    pub color: Color,
    pub Object: PRIM_DESC,
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct IntersectControl {
    pub last_time_val: u16,
    pub EndPoint: POINT,
    pub IntersectStamp: STAMP,
}

pub use IntersectControl as INTERSECT_CONTROL;

impl Copy for PRIMITIVE {}
impl Clone for PRIMITIVE {
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for IntersectControl {}
impl Clone for IntersectControl {
    fn clone(&self) -> Self {
        *self
    }
}
impl std::fmt::Debug for IntersectControl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("INTERSECT_CONTROL")
            .field("last_time_val", &self.last_time_val)
            .field("EndPoint", &self.EndPoint)
            .field("IntersectStamp", &self.IntersectStamp)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cloned<T: Clone>(value: &T) -> T {
        value.clone()
    }

    #[test]
    fn record_clones_keep_fields_and_debug_names() {
        // SAFETY: these ABI records contain only integers, unions, and nullable raw pointers.
        let (mut primitive, mut control): (PRIMITIVE, IntersectControl) =
            unsafe { std::mem::zeroed() };
        primitive.Type = 3;
        primitive.Links = 17;
        control.last_time_val = 21;
        control.EndPoint.x = 9;
        let copy = cloned(&primitive);
        assert_eq!((copy.Type, copy.Links), (3, 17));
        let copy = cloned(&control);
        assert_eq!((copy.last_time_val, copy.EndPoint.x), (21, 9));
        let text = format!("{copy:?}");
        assert!(text.starts_with("INTERSECT_CONTROL { last_time_val: 21, EndPoint:"));
        assert!(text.contains("IntersectStamp:"));
    }
}
