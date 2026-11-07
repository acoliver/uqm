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

#[cfg(not(test))]
pub fn animation_preprocess(cx: &mut Cx<'_>, e: ElementId) {
    let wait = cx.element(e).turn_wait();
    if wait > 0 {
        cx.element_mut(e).set_turn_wait(wait - 1);
    } else {
        let frame = cx.advance_frame(cx.element(e).current.image.frame.view());
        cx.set_frame(e, true, frame);
        cx.element_mut(e).flags |= 1 << 5;
        let wait = cx.element(e).thrust_wait();
        cx.element_mut(e).set_turn_wait(wait);
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

    #[test]
    fn installation_preserves_callback_addresses_and_clears_absent_hooks() {
        fn element(_: &mut Cx<'_>, _: ElementId) {}
        fn collision(_: &mut Cx<'_>, _: ElementId, _: &mut POINT, _: ElementId, _: &mut POINT) {}

        let mut record = ElementRecord::default();
        ElementHooks {
            pre: Some(element),
            post: Some(element),
            collide: Some(collision),
            death: Some(element),
        }
        .install(&mut record);

        assert_eq!(
            record.preprocess_func.map(ElementHook::address),
            Some(element as ElementFn as usize)
        );
        assert_eq!(
            record.postprocess_func.map(ElementHook::address),
            Some(element as ElementFn as usize)
        );
        assert_eq!(
            record.collision_func.map(CollisionHook::address),
            Some(collision as CollisionFn as usize)
        );
        assert_eq!(
            record.death_func.map(ElementHook::address),
            Some(element as ElementFn as usize)
        );

        ElementHooks::default().install(&mut record);
        assert!(record.preprocess_func.is_none());
        assert!(record.postprocess_func.is_none());
        assert!(record.collision_func.is_none());
        assert!(record.death_func.is_none());
    }
}
