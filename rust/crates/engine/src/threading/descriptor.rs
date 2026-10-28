/// Held while a thread creates a descriptor that must not outlive an exec, or
/// while it forks a child that would inherit every descriptor then open.
///
/// Linux creates close-on-exec descriptors atomically (`pipe2`,
/// `SOCK_CLOEXEC`), so the guard is empty there. Darwin has neither: it
/// creates the descriptor and sets `FD_CLOEXEC` in a second call, and a child
/// forked from another thread between the two calls keeps the descriptor for
/// its whole life. This lock makes the two steps and every fork exclusive.
#[cfg(unix)]
#[derive(Debug)]
pub struct DescriptorInheritanceGuard {
    #[cfg(not(target_os = "linux"))]
    _exclusive: std::sync::MutexGuard<'static, ()>,
}

/// Block until no other thread is creating a close-on-exec descriptor or
/// spawning a child, and keep it that way until the guard is dropped.
#[cfg(unix)]
pub fn guard_descriptor_inheritance() -> DescriptorInheritanceGuard {
    #[cfg(not(target_os = "linux"))]
    {
        static EXCLUSIVE: std::sync::Mutex<()> = std::sync::Mutex::new(());
        // The mutex protects no data, so a panic while it was held leaves no
        // broken invariant to propagate.
        DescriptorInheritanceGuard {
            _exclusive: EXCLUSIVE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        }
    }
    #[cfg(target_os = "linux")]
    DescriptorInheritanceGuard {}
}
