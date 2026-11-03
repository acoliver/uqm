//! Descriptor construction uses the species behavior table directly.
#[cfg(not(test))]
#[cfg(not(test))]
/// # Safety
/// Construct on the game thread while the lifecycle resources are initialized.
pub unsafe fn descriptor(species: u32) -> crate::engine::battle::RaceDesc {
    use crate::engine::World as GameWorld;
    let race =
        crate::ships::races::race(species).unwrap_or_else(|| panic!("unknown species {species}"));
    (race.descriptor)(&mut *GameWorld::current())
}
