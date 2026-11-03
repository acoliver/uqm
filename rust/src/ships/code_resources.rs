//! Descriptor construction uses the species behavior table directly.
#[cfg(not(test))]
pub fn descriptor(
    world: &mut crate::engine::World,
    species: u32,
) -> crate::engine::battle::RaceDesc {
    let race =
        crate::ships::races::race(species).unwrap_or_else(|| panic!("unknown species {species}"));
    (race.descriptor)(world)
}
