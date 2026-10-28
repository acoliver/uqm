//! LIFO handoff between legacy entry points and the one owned battle context.

use crate::engine::battle::Cx;
use std::ptr::{self, NonNull};

static mut CONTEXT: Option<NonNull<Cx<'static>>> = None;

struct Restore {
    before: Option<NonNull<Cx<'static>>>,
    during: Option<NonNull<Cx<'static>>>,
}

impl Drop for Restore {
    fn drop(&mut self) {
        // SAFETY: every bridge operation runs synchronously on the game thread.
        unsafe {
            let current = ptr::replace(&raw mut CONTEXT, self.before);
            assert_eq!(current, self.during, "bridge handoff returned out of order");
        }
    }
}

/// # Safety
/// Game thread only. The caller must suspend all access to cx and its records
/// until the legacy call returns. The published pointer cannot escape the call.
pub unsafe fn publish<R>(cx: &mut Cx<'_>, call: impl FnOnce() -> R) -> R {
    // SAFETY: publication transfers exclusive access for this synchronous call.
    unsafe {
        assert!(
            ptr::read(&raw const CONTEXT).is_none(),
            "bridge already has a published context"
        );
        let published = Some(NonNull::from(cx).cast());
        let restore = Restore {
            before: None,
            during: published,
        };
        ptr::write(&raw mut CONTEXT, published);
        let result = call();
        drop(restore);
        result
    }
}

/// # Safety
/// Game thread only, inside a published legacy call. No record reference may
/// survive this call or any nested publication from its body.
pub unsafe fn enter<R>(call: impl FnOnce(&mut Cx<'_>) -> R) -> R {
    // SAFETY: taking the slot suspends publication while the unique borrow is live.
    unsafe {
        let mut context = ptr::replace(&raw mut CONTEXT, None)
            .unwrap_or_else(|| panic!("bridge entry without a published context"));
        let restore = Restore {
            before: Some(context),
            during: None,
        };
        let result = call(context.as_mut());
        drop(restore);
        result
    }
}

/// # Safety
/// Must be called at a game-thread legacy boundary with a published context.
pub unsafe fn battle<R>(call: impl FnOnce(&mut crate::engine::battle::Battle) -> R) -> R {
    // SAFETY: this accessor retains no battle or record borrow across entry.
    unsafe { enter(|cx| call(cx.b)) }
}

/// Own the battle context across the existing synchronous game lifecycle.
///
/// # Safety
/// Run only on the game thread. The legacy queue and GlobData authorities must
/// outlive the call; initialization and cleanup occur inside this boundary.
pub unsafe fn with_battle<R>(call: impl FnOnce() -> R) -> R {
    use crate::engine::battle::legacy::disp_q;
    use crate::engine::battle::{Battle, Cx, World};
    use crate::ships::native::abi::GlobData;
    // SAFETY: all pointers derive directly from the authority's raw address.
    unsafe {
        let mut battle = Battle::new(&raw mut disp_q);
        let mut world = World::new(
            &raw mut GlobData.Game_state.CurrentActivity,
            &raw mut GlobData.Game_state.ip_group_q,
        );
        let mut cx = Cx {
            b: &mut battle,
            world: &mut world,
            rng: crate::math::random,
        };
        publish(&mut cx, call)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::battle::record::QUEUE;
    use crate::engine::battle::{Battle, World};

    #[test]
    #[serial_test::serial]
    fn empty_entry_panics_and_nested_handoffs_restore_lifo() {
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        assert!(std::panic::catch_unwind(|| unsafe { enter(|_| ()) }).is_err());
        let mut queue = QUEUE::default();
        let mut groups = QUEUE::default();
        let mut activity = 7;
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let mut battle = unsafe { Battle::new(&raw mut queue) };
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let mut world = unsafe { World::new(&raw mut activity, &raw mut groups) };
        let mut cx = Cx {
            b: &mut battle,
            world: &mut world,
            rng: crate::math::random,
        };
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            publish(&mut cx, || {
                enter(|cx| {
                    cx.b.counters = [3, 4];
                    publish(cx, || enter(|cx| cx.b.counters[0] -= 1));
                    assert_eq!(cx.b.counters, [2, 4]);
                    assert_eq!(cx.world.activity(), 7);
                })
            });
        }
        assert_eq!(cx.b.counters, [2, 4]);
        cx.b.ship_lost(0);
        assert_eq!(cx.b.counters, [1, 4]);
        cx.b.ship_lost(0);
        cx.b.ship_lost(0);
        assert_eq!(cx.b.counters, [u8::MAX, 4]);
        cx.b.counters = [2, 4];
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
            publish(&mut cx, || {
                enter(|cx| publish(cx, || enter::<()>(|_| panic!("hook failed"))))
            })
        }));
        assert!(failed.is_err());
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            publish(&mut cx, || enter(|cx| assert_eq!(cx.b.counters, [2, 4])));
        }
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        assert!(std::panic::catch_unwind(|| unsafe { enter(|_| ()) }).is_err());
    }
}
