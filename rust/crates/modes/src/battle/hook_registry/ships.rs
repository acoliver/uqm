//! Ship and battle callback registry.
mod core;
pub(super) fn hooks() -> Vec<(usize, &'static str)> {
    core::hooks()
}
