//! LIFO handoff between legacy entry points and the one owned battle context.

use crate::engine::battle::{record::Kernel, Cx};
use std::ptr::{self, NonNull};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    published: Option<NonNull<Cx<'static>>>,
    kernel: Option<NonNull<crate::engine::battle::record::Kernel>>,
}
impl Slot {
    fn published(cx: &mut Cx<'_>) -> Self {
        let kernel = NonNull::new(cx.kernel());
        Self {
            published: Some(NonNull::from(cx).cast()),
            kernel,
        }
    }
    fn entered(self) -> Self {
        Self {
            published: None,
            ..self
        }
    }
    fn context(self) -> NonNull<Cx<'static>> {
        self.published
            .unwrap_or_else(|| panic!("bridge entry without a published context"))
    }
    fn owner(self) -> *mut crate::engine::battle::record::Kernel {
        self.kernel
            .unwrap_or_else(|| panic!("kernel access without a lifecycle context"))
            .as_ptr()
    }
}

static mut CONTEXT: Slot = Slot {
    published: None,
    kernel: None,
};

struct Restore {
    before: Slot,
    during: Slot,
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
        let before = ptr::read(&raw const CONTEXT);
        assert!(
            before.published.is_none(),
            "bridge already has a published context"
        );
        let published = Slot::published(cx);
        let restore = Restore {
            before,
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
        let before = ptr::read(&raw const CONTEXT);
        let mut context = before.context();
        let during = before.entered();
        ptr::write(&raw mut CONTEXT, during);
        let restore = Restore { before, during };
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
    #[cfg(not(test))]
    let shared_frames = [
        (&raw mut crate::game_init::space_resources::asteroid).cast(),
        (&raw mut crate::game_init::space_resources::explosion).cast(),
    ];
    #[cfg(test)]
    let shared_frames = [ptr::null_mut(); 2];
    #[cfg(not(test))]
    let tracking =
        Some(crate::battle::weapon_native::track_ship as unsafe extern "C" fn(_, _) -> _);
    #[cfg(test)]
    let tracking = None;
    // SAFETY: all pointers derive directly from the authority's raw address.
    unsafe {
        let mut battle = Battle::new(&raw mut disp_q);
        let mut world = World::new(
            &raw mut GlobData.Game_state.CurrentActivity,
            &raw mut GlobData.Game_state.ip_group_q,
            shared_frames,
            tracking,
        );
        let mut cx = Cx::new(&mut battle, &mut world, crate::math::random);
        publish(&mut cx, call)
    }
}

impl Kernel {
    /// # Safety
    /// Game-thread lifecycle only. No kernel reference may survive a callback or handoff.
    pub(crate) unsafe fn current<'a>() -> &'a mut Self {
        // SAFETY: the lifecycle retains this owner projection while Cx is entered.
        unsafe { &mut *ptr::read(&raw const CONTEXT).owner() }
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
        let mut world = unsafe {
            World::new(
                &raw mut activity,
                &raw mut groups,
                [ptr::null_mut(); 2],
                None,
            )
        };
        let mut cx = Cx::new(&mut battle, &mut world, crate::math::random);
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            publish(&mut cx, || {
                enter(|cx| {
                    let owner = ptr::from_mut(Kernel::current());
                    assert_eq!(owner, cx.kernel());
                    (*owner).instant_victory = 1;
                    publish(cx, || {
                        enter(|cx| {
                            assert_eq!(ptr::from_mut(Kernel::current()), cx.kernel());
                            assert_eq!(Kernel::current().instant_victory, 1);
                        })
                    });
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
