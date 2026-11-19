//! Ship and battle callback registry.
mod core;
mod races;
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    races::hooks().into_iter().chain(core::hooks()).collect()
}
