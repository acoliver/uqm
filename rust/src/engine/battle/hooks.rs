//! Copyable callbacks shared by legacy records and id-based consumers.

use super::{record::ElementRecord, Cx, ElementId};
use crate::graphics::native_abi::POINT;

pub type ElementFn = fn(&mut Cx<'_>, ElementId);
pub type CollisionFn = fn(&mut Cx<'_>, ElementId, &mut POINT, ElementId, &mut POINT);

#[derive(Clone, Copy, Debug)]
pub enum ElementHook {
    Native(ElementFn),
}

#[derive(Clone, Copy, Debug)]
pub enum CollisionHook {
    Native(CollisionFn),
}

impl ElementHook {
    pub fn address(self) -> usize {
        match self {
            Self::Native(f) => f as usize,
        }
    }
}
impl CollisionHook {
    pub fn address(self) -> usize {
        match self {
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
    pub fn install(self, e: &mut ElementRecord) {
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
        // SAFETY: Option uses the null function-pointer niche for an absent native hook.
        let empty: (Option<ElementHook>, Option<CollisionHook>) = unsafe { std::mem::zeroed() };
        assert!(empty.0.is_none());
        assert!(empty.1.is_none());
    }
}
