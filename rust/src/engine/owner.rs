//! Lease values shared by the owned engine records and legacy entry boundaries.

use std::{marker::PhantomData, rc::Rc};

#[derive(Clone, Copy)]
pub(crate) enum Owner {
    World,
    Kernel,
}

/// A raw legacy projection whose borrow ends on drop. It cannot cross threads.
/// No owner or field reference or pointer may outlive this lease.
pub struct Projection<T> {
    pub(crate) pointer: *mut T,
    pub(crate) owner: Owner,
    pub(crate) not_send: PhantomData<Rc<()>>,
}
