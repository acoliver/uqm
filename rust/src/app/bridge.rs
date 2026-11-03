//! LIFO handoff between legacy entry points and the one owned battle context.

use crate::engine::battle::{record::Kernel, Cx};
#[cfg(test)]
use crate::engine::World as WorldOwner;
use crate::engine::{Owner, Projection};
use parking_lot::Mutex;
use std::ptr::NonNull;
use std::{marker::PhantomData, thread::ThreadId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Slot {
    published: Option<NonNull<Cx<'static>>>,
    kernel: Option<NonNull<crate::engine::battle::record::Kernel>>,
    world: Option<NonNull<crate::engine::World>>,
}
impl Slot {
    fn published(cx: &mut Cx<'_>) -> Self {
        let kernel = NonNull::new(cx.kernel());
        let world = NonNull::new(&raw mut *cx.world);
        Self {
            published: Some(NonNull::from(cx).cast()),
            kernel,
            world,
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

// SAFETY: the mutex serializes slot access, and every projection checks the
// publishing thread before its pointer can be dereferenced. Pointers never move
// their owners or permit use on a different thread.
unsafe impl Send for Slot {}

struct BridgeState {
    slot: Slot,
    thread: Option<ThreadId>,
    world_borrowed: bool,
    kernel_borrowed: bool,
    observer: std::sync::Weak<crate::automation::AutomationObserver>,
}

impl BridgeState {
    fn assert_thread(&self) {
        assert_eq!(
            self.thread,
            Some(std::thread::current().id()),
            "owner projection outside the game thread"
        );
    }
    fn assert_unborrowed(&self) {
        assert!(
            !self.world_borrowed && !self.kernel_borrowed,
            "owner projection overlaps a context handoff"
        );
    }
}

static CONTEXT: Mutex<BridgeState> = Mutex::new(BridgeState {
    slot: Slot {
        published: None,
        kernel: None,
        world: None,
    },
    thread: None,
    world_borrowed: false,
    kernel_borrowed: false,
    observer: std::sync::Weak::new(),
});

pub(crate) fn install_observer(observer: &std::sync::Arc<crate::automation::AutomationObserver>) {
    let mut state = CONTEXT.lock();
    assert!(
        state.observer.upgrade().is_none(),
        "application observer already installed"
    );
    state.observer = std::sync::Arc::downgrade(observer);
}

pub(crate) fn observer() -> Option<std::sync::Arc<crate::automation::AutomationObserver>> {
    CONTEXT.lock().observer.upgrade()
}

pub(crate) fn uninstall_observer(observer: &std::sync::Arc<crate::automation::AutomationObserver>) {
    let mut state = CONTEXT.lock();
    assert!(
        std::sync::Weak::ptr_eq(&state.observer, &std::sync::Arc::downgrade(observer)),
        "application observer changed before teardown"
    );
    state.observer = std::sync::Weak::new();
}

#[cfg(not(test))]
pub(crate) fn campaign_view() -> crate::automation::campaign_state::CampaignView {
    crate::engine::World::current().read(crate::automation::campaign_state::CampaignView::capture)
}

#[cfg(test)]
pub(crate) fn campaign_view() -> crate::automation::campaign_state::CampaignView {
    crate::automation::campaign_state::CampaignView::empty()
}

impl<T> std::ops::Deref for Projection<T> {
    type Target = *mut T;
    fn deref(&self) -> &Self::Target {
        &self.pointer
    }
}
impl<T> Drop for Projection<T> {
    fn drop(&mut self) {
        let mut state = CONTEXT.lock();
        state.assert_thread();
        let flag = match self.owner {
            Owner::World => &mut state.world_borrowed,
            Owner::Kernel => &mut state.kernel_borrowed,
        };
        assert!(*flag, "owner projection released twice");
        *flag = false;
    }
}
impl<T> Projection<T> {
    /// Copy values from this lease without holding it across the caller's next operation.
    pub fn read<R>(self, read: impl FnOnce(&T) -> R) -> R {
        // SAFETY: publication pins the owner for this game thread, the exclusive
        // flag excludes other leases, and the callback cannot return a borrowed T.
        read(unsafe { &*self.pointer })
    }

    /// Apply a scoped mutation, releasing the owner before the caller continues.
    pub fn write<R>(self, write: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY: publication pins the owner on the asserted game thread; this
        // lease excludes other projections and handoffs for the callback's borrow.
        write(unsafe { &mut *self.pointer })
    }

    /// # Safety
    /// The pointer must name an inline field of this owner; no pointer or
    /// reference from the original projection may be retained.
    #[cfg(not(test))]
    pub(crate) unsafe fn map<U>(self, project: impl FnOnce(*mut T) -> *mut U) -> Projection<U> {
        let this = std::mem::ManuallyDrop::new(self);
        Projection {
            pointer: project(this.pointer),
            owner: this.owner,
            not_send: PhantomData,
        }
    }
}

#[track_caller]
fn project<T>(owner: Owner, pointer: impl FnOnce(Slot) -> *mut T) -> Projection<T> {
    let mut state = CONTEXT.lock();
    state.assert_thread();
    let value = pointer(state.slot);
    let flag = match owner {
        Owner::World => &mut state.world_borrowed,
        Owner::Kernel => &mut state.kernel_borrowed,
    };
    assert!(!*flag, "overlapping mutable owner projection");
    *flag = true;
    Projection {
        pointer: value,
        owner,
        not_send: PhantomData,
    }
}

struct Restore {
    before: Slot,
    during: Slot,
}

impl Drop for Restore {
    fn drop(&mut self) {
        let mut state = CONTEXT.lock();
        state.assert_thread();
        state.assert_unborrowed();
        let current = std::mem::replace(&mut state.slot, self.before);
        assert_eq!(current, self.during, "bridge handoff returned out of order");
        if self.before.world.is_none() {
            state.thread = None;
        }
    }
}

/// # Safety
/// Game thread only. The caller must suspend all access to cx and its records
/// until the legacy call returns. The published pointer cannot escape the call.
pub unsafe fn publish<R>(cx: &mut Cx<'_>, call: impl FnOnce() -> R) -> R {
    let mut state = CONTEXT.lock();
    if state.thread.is_some() {
        state.assert_thread();
    }
    state.assert_unborrowed();
    let before = state.slot;
    assert!(
        before.published.is_none(),
        "bridge already has a published context"
    );
    let published = Slot::published(cx);
    let restore = Restore {
        before,
        during: published,
    };
    state.slot = published;
    state.thread = Some(std::thread::current().id());
    drop(state);
    let result = call();
    drop(restore);
    result
}

/// # Safety
/// Game thread only, inside a published legacy call. No record reference may
/// survive this call or any nested publication from its body.
pub unsafe fn enter<R>(call: impl FnOnce(&mut Cx<'_>) -> R) -> R {
    // SAFETY: taking the slot suspends publication while the unique borrow is live.
    unsafe {
        let mut state = CONTEXT.lock();
        state.assert_thread();
        state.assert_unborrowed();
        let before = state.slot;
        let mut context = before.context();
        let during = before.entered();
        state.slot = during;
        drop(state);
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

/// Run legacy mode entry points inside the app-owned game lifecycle.
///
/// # Safety
/// Game thread only; the call must not retain any owner borrow after returning.
pub unsafe fn with_game<R>(game: &mut crate::engine::Game, call: impl FnOnce() -> R) -> R {
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    );
    // SAFETY: this lifecycle owns both owners and suspends direct access until publication ends.
    unsafe { publish(&mut cx, call) }
}

impl crate::engine::World {
    /// Reserve the published owner on its game thread, panicking on overlap.
    /// Dereferencing the raw pointer remains unsafe; derived pointers and references
    /// must end before the lease is dropped or a callback is invoked.
    #[track_caller]
    pub fn current() -> Projection<Self> {
        project(Owner::World, |slot| {
            slot.world
                .unwrap_or_else(|| panic!("world access without a lifecycle context"))
                .as_ptr()
        })
    }
}

impl crate::engine::WorldData {
    /// Reserve data under the World's game-thread assertion and borrow flag.
    #[track_caller]
    pub fn current() -> Projection<Self> {
        project(Owner::World, |slot| {
            let Some(world) = slot.world else {
                panic!("world access without a lifecycle context");
            };
            // SAFETY: project asserts the publishing thread and excludes owner
            // leases; the lifecycle pins World, and data is its inline field.
            unsafe { &raw mut (*world.as_ptr()).data }
        })
    }
}

impl Kernel {
    /// Reserve the published owner on its game thread, panicking on overlap.
    /// Dereferencing the raw pointer remains unsafe; derived pointers and references
    /// must end before the lease is dropped or a callback is invoked.
    #[track_caller]
    pub fn current() -> Projection<Self> {
        project(Owner::Kernel, Slot::owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::engine::battle::{Battle, World};

    #[test]
    #[serial_test::serial]
    fn owner_projection_rejects_overlap_and_the_wrong_thread() {
        let mut game = crate::engine::Game::new();
        // SAFETY: this serialized test owns the published lifecycle and all projections.
        unsafe {
            with_game(&mut game, || {
                let first = World::current();
                assert!(std::panic::catch_unwind(|| World::current()).is_err());
                std::hint::black_box(first);
                let wrong_thread = std::thread::spawn(|| {
                    std::panic::catch_unwind(|| {
                        World::current();
                    })
                    .is_err()
                })
                .join()
                .unwrap();
                assert!(wrong_thread);
            })
        };
    }

    #[test]
    #[serial_test::serial]
    fn mapped_leases_share_world_flag_and_handoffs_require_release() {
        let mut game = crate::engine::Game::new();
        // SAFETY: this serialized test owns the lifecycle and retains no derived owner references.
        unsafe {
            with_game(&mut game, || {
                let fields = crate::engine::WorldData::current();
                assert!(std::panic::catch_unwind(WorldOwner::current).is_err());
                assert!(std::panic::catch_unwind(|| enter(|_| ())).is_err());
                drop(fields);
                let kernel = Kernel::current();
                assert!(std::panic::catch_unwind(Kernel::current).is_err());
                drop(kernel);
                let panic = std::panic::catch_unwind(|| {
                    let _lease = WorldOwner::current();
                    panic!("body failed");
                });
                assert!(panic.is_err());
                enter(|cx| {
                    cx.world.set_activity(19);
                    publish(cx, || {
                        let world = WorldOwner::current();
                        assert_eq!((**world).activity(), 19);
                    });
                });
            });
        }
        assert_eq!(game.world.activity(), 19);
        assert!(std::panic::catch_unwind(WorldOwner::current).is_err());
    }

    #[test]
    #[serial_test::serial]
    fn empty_entry_panics_and_nested_handoffs_restore_lifo() {
        use crate::engine::Kernel;
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        assert!(std::panic::catch_unwind(|| unsafe { enter(|_| ()) }).is_err());
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let mut battle = Battle::new();
        let mut kernel = crate::engine::battle::record::Kernel::new();
        kernel.reset_element_capacity(3);
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        let mut world = { World::new() };
        world.set_activity(7);
        let mut cx = Cx::new(&mut battle, &mut world, &mut kernel, crate::math::random);
        // SAFETY: this serialized test owns the records and projected storage for the scope.
        unsafe {
            publish(&mut cx, || {
                enter(|cx| {
                    let owner = Kernel::current();
                    assert_eq!(*owner, cx.kernel());
                    (**owner).instant_victory = 1;
                    drop(owner);
                    publish(cx, || {
                        enter(|cx| {
                            assert_eq!(*Kernel::current(), cx.kernel());
                            assert_eq!((**Kernel::current()).instant_victory, 1);
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
