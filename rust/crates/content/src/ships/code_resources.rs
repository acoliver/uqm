//! Descriptor construction uses the species behavior table directly.
pub fn descriptor(world: &mut uqm_engine::World, species: u32) -> uqm_engine::battle::RaceDesc {
    let race =
        crate::ships::races::race(species).unwrap_or_else(|| panic!("unknown species {species}"));
    (race.descriptor)(world)
}
