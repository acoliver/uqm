//! Copyable callbacks shared by legacy records and id-based consumers.

use super::{record::ELEMENT, Cx, ElementId};
use crate::graphics::native_abi::POINT;

pub type ElementFn = fn(&mut Cx<'_>, ElementId);
pub type CollisionFn = fn(&mut Cx<'_>, ElementId, &mut POINT, ElementId, &mut POINT);
pub type LegacyElementFn = unsafe extern "C" fn(*mut ELEMENT);
pub type LegacyCollisionFn =
    unsafe extern "C" fn(*mut ELEMENT, *mut POINT, *mut ELEMENT, *mut POINT);

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum ElementHook<F = LegacyElementFn> {
    Legacy(F) = 1,
    Native(ElementFn) = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum CollisionHook<F = LegacyCollisionFn> {
    Legacy(F) = 1,
    Native(CollisionFn) = 2,
}

impl<E> ElementHook<unsafe extern "C" fn(*mut E)> {
    pub fn address(self) -> usize {
        match self {
            Self::Legacy(f) => f as usize,
            Self::Native(f) => f as usize,
        }
    }
}
impl<E, P> CollisionHook<unsafe extern "C" fn(*mut E, *mut P, *mut E, *mut P)> {
    pub fn address(self) -> usize {
        match self {
            Self::Legacy(f) => f as usize,
            Self::Native(f) => f as usize,
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ElementHooks {
    pub pre: Option<ElementFn>,
    pub post: Option<ElementFn>,
    pub collide: Option<CollisionFn>,
    pub death: Option<ElementFn>,
}

impl ElementHooks {
    pub fn install(self, e: &mut ELEMENT) {
        e.preprocess_func = self.pre.map(ElementHook::Native);
        e.postprocess_func = self.post.map(ElementHook::Native);
        e.collision_func = self.collide.map(CollisionHook::Native);
        e.death_func = self.death.map(ElementHook::Native);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nullable_hooks_keep_zero_initialized_records_valid() {
        // SAFETY: zero selects the unused discriminant, which Option uses for None.
        let empty: (Option<ElementHook>, Option<CollisionHook>) = unsafe { std::mem::zeroed() };
        assert!(empty.0.is_none());
        assert!(empty.1.is_none());
    }
}
