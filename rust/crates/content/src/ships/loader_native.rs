//! Construct descriptors from the closed race factory table.
pub trait DescriptorLoader: Sized {
    fn load(
        content: &uqm_engine::resource::Content,
        world: &mut uqm_engine::World,
        species: u32,
        battle: bool,
    ) -> Option<Self>;
}

impl DescriptorLoader for uqm_engine::battle::RaceDesc {
    fn load(
        content: &uqm_engine::resource::Content,
        world: &mut uqm_engine::World,
        species: u32,
        battle: bool,
    ) -> Option<Self> {
        let race = crate::ships::races::race(species)?;
        let mut descriptor = (race.descriptor)(world);
        if !descriptor.initialize_assets(content, battle) {
            descriptor.unload(world, true);
            return None;
        }
        Some(descriptor)
    }
}
