//! Copyable legacy hook values; native arms arrive with their first consumers.

use super::record::ELEMENT;
use crate::graphics::native_abi::POINT;

pub type LegacyElementFn = unsafe extern "C" fn(*mut ELEMENT);
pub type LegacyCollisionFn =
    unsafe extern "C" fn(*mut ELEMENT, *mut POINT, *mut ELEMENT, *mut POINT);

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub enum ElementHook<F = LegacyElementFn> {
    Legacy(F),
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub enum CollisionHook<F = LegacyCollisionFn> {
    Legacy(F),
}

impl<E> ElementHook<unsafe extern "C" fn(*mut E)> {
    pub fn address(self) -> usize {
        let Self::Legacy(f) = self;
        f as usize
    }
}
impl<E, P> CollisionHook<unsafe extern "C" fn(*mut E, *mut P, *mut E, *mut P)> {
    pub fn address(self) -> usize {
        let Self::Legacy(f) = self;
        f as usize
    }
}
impl<E, P> CollisionHook<unsafe extern "C" fn(*mut E, *const P, *mut E, *const P)> {
    pub fn address(self) -> usize {
        let Self::Legacy(f) = self;
        f as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_nullable_hook_keeps_the_record_layout_and_zero_value() {
        assert_eq!(
            size_of::<Option<ElementHook>>(),
            size_of::<Option<LegacyElementFn>>()
        );
        assert_eq!(
            size_of::<Option<CollisionHook>>(),
            size_of::<Option<LegacyCollisionFn>>()
        );
        assert_eq!(
            align_of::<Option<CollisionHook>>(),
            align_of::<Option<LegacyCollisionFn>>()
        );
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let empty: Option<ElementHook> = unsafe { std::mem::zeroed() };
        assert!(empty.is_none());
    }
}
