//! The save menu persistence entry points, called on the game thread.
pub(crate) mod load;
pub(crate) mod save;
pub(super) use load::finish_load;
