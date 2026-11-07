//! Lease values shared by the owned engine records and legacy entry boundaries.

use std::{marker::PhantomData, rc::Rc};

#[derive(Clone, Copy)]
pub enum Owner {
    World,
    Kernel,
}

/// A raw legacy projection whose borrow ends on drop. It cannot cross threads.
/// No owner or field reference or pointer may outlive this lease.
pub struct Projection<T> {
    pub pointer: *mut T,
    pub owner: Owner,
    pub not_send: PhantomData<Rc<()>>,
}
