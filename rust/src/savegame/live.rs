//! The save menu persistence entry points, called on the game thread.
mod load;
mod save;
pub(super) use load::finish_load;
