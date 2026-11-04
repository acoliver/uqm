//! Construct descriptors from the closed race factory table.
#[cfg(not(test))]
impl crate::engine::battle::RaceDesc {
    pub fn load(world: &mut crate::engine::World, species: u32, battle: bool) -> Option<Self> {
        let race = crate::ships::races::race(species)?;
        let mut descriptor = (race.descriptor)(world);
        if !descriptor.initialize_assets(battle) {
            descriptor.unload(world, true, true);
            return None;
        }
        Some(descriptor)
    }
}
