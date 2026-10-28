//! C ABI for the task library (`libs/tasklib.h`).
//!
//! A task is a thread plus a mutex-protected state word used to pass
//! messages (`TASK_INUSE`, `TASK_EXIT`) between the task and its owner. The
//! 64 task slots live in a static array shared with C callers, so
//! [`TaskStruct`] mirrors `struct taskstruct` exactly.

use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};

const TASK_MAX: usize = 64;

const TASK_INUSE: u32 = 1;

const SYNC_CLASS_TOPLEVEL: u32 = 1 << 0;
const SYNC_CLASS_RESOURCE: u32 = 1 << 3;

const LOG_DEBUG: c_int = 5;

use crate::threading::legacy_abi::CreateMutex_Core;

use crate::threading::legacy_abi::DestroyMutex;

use crate::threading::legacy_abi::LockMutex;

use crate::threading::legacy_abi::UnlockMutex;

/// Mirror of `struct taskstruct`.
#[repr(C)]
pub struct TaskStruct {
    state_mutex: AtomicPtr<c_void>,
    /// Protected by `state_mutex` for writes; read lock-free by
    /// `Task_ReadState`.
    state: AtomicU32,
    name: AtomicPtr<c_char>,
    thread: AtomicPtr<c_void>,
}

impl TaskStruct {
    const fn new() -> Self {
        Self {
            state_mutex: AtomicPtr::new(ptr::null_mut()),
            state: AtomicU32::new(0),
            name: AtomicPtr::new(ptr::null_mut()),
            thread: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// Apply `update` to the state word under the task's mutex and return
    /// the previous state masked by `mask`.
    fn update_state(&self, mask: u32, update: impl FnOnce(u32) -> u32) -> u32 {
        let mutex = self.state_mutex.load(Ordering::Relaxed);
        // SAFETY: the mutex was created by InitTaskSystem.
        unsafe { LockMutex((mutex).cast()) };
        let old = self.state.load(Ordering::Relaxed);
        self.state.store(update(old), Ordering::Release);
        // SAFETY: locked above.
        unsafe { UnlockMutex((mutex).cast()) };
        old & mask
    }
}

struct TaskArray([TaskStruct; TASK_MAX]);

// SAFETY: every field is atomic.
unsafe impl Sync for TaskArray {}

static TASKS: TaskArray = TaskArray([const { TaskStruct::new() }; TASK_MAX]);

#[no_mangle]
pub extern "C" fn InitTaskSystem() {
    for task in &TASKS.0 {
        // SAFETY: CreateMutex_Core takes a static name string.
        let mutex = unsafe {
            CreateMutex_Core(
                c"task manager lock".as_ptr(),
                SYNC_CLASS_TOPLEVEL | SYNC_CLASS_RESOURCE,
            )
        };
        task.state_mutex.store((mutex).cast(), Ordering::Relaxed);
    }
}

#[no_mangle]
pub extern "C" fn CleanupTaskSystem() {
    for task in &TASKS.0 {
        let mutex = task.state_mutex.swap(ptr::null_mut(), Ordering::Relaxed);
        // SAFETY: the mutex came from InitTaskSystem.
        unsafe { DestroyMutex((mutex).cast()) };
    }
}

/// # Safety
/// `task` must point to a slot of the task array.
#[no_mangle]
pub unsafe extern "C" fn Task_SetState(task: *mut TaskStruct, state_mask: u32) -> u32 {
    (*task).update_state(state_mask, |state| state | state_mask)
}

/// # Safety
/// `task` must point to a slot of the task array.
#[no_mangle]
pub unsafe extern "C" fn Task_ClearState(task: *mut TaskStruct, state_mask: u32) -> u32 {
    (*task).update_state(state_mask, |state| state & !state_mask)
}

/// # Safety
/// `task` must be a task returned by [`AssignTask`].
#[no_mangle]
pub unsafe extern "C" fn FinishTask(task: *mut TaskStruct) {
    (*task).thread.store(ptr::null_mut(), Ordering::Release);
    if Task_ClearState(task, TASK_INUSE) == 0 {
        crate::log_abi::log_message(
            LOG_DEBUG,
            &format!(
                "Task error!  Attempted to FinishTask '{}'... but it was already done!",
                std::ffi::CStr::from_ptr((*task).name.load(Ordering::Relaxed)).to_string_lossy()
            ),
        );
    }
}
