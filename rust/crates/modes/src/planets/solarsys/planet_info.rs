//! Per-body scan masks for the selected solar-system location.

use crate::engine::world::planet_info::{PlanetInfoManager, ScanRetrieveMask};
use crate::engine::World;
use crate::planets::solar_system::SolarSystem;

pub fn get_planet_info(world: &mut World, solar: &mut SolarSystem) {
    let location = solar.selected_location();
    let moons = solar
        .sys
        .planets
        .each_ref()
        .map(|planet| planet.num_planets);
    let mut manager = PlanetInfoManager::new(&mut world.files);
    // Preserve the empty-mask policy for absent or unreadable external planet data.
    solar.sys.info.planet_info.scan_retrieve_mask = manager
        .get_planet_info(
            usize::from(location.star),
            usize::from(location.planet),
            usize::from(location.moon),
            &moons[..usize::from(solar.sys.sun.num_planets)],
        )
        .unwrap_or_default()
        .to_array();
}

pub fn put_planet_info(world: &mut World, solar: &SolarSystem) {
    let location = solar.selected_location();
    let moons = solar
        .sys
        .planets
        .each_ref()
        .map(|planet| planet.num_planets);
    let mut manager = PlanetInfoManager::new(&mut world.files);
    let mask = ScanRetrieveMask::from_array(&solar.sys.info.planet_info.scan_retrieve_mask);
    let _ = manager.put_planet_info(
        usize::from(location.star),
        usize::from(location.planet),
        usize::from(location.moon),
        &mask,
        &moons[..usize::from(solar.sys.sun.num_planets)],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{
        world::{generation::Body, StarId},
        World,
    };

    #[test]
    fn selected_body_masks_and_key_ignore_the_campaigns_later_star_selection() {
        let mut world = World::new();
        world.set_catalog(std::borrow::Cow::Borrowed(
            &crate::planets::tables::STARMAP_ARRAY,
        ));
        let stars = world.catalog().len();
        PlanetInfoManager::new(&mut world.files)
            .init_planet_info(stars)
            .unwrap();
        let mut solar = SolarSystem::new(&world, StarId(82));
        solar.sys.sun.num_planets = 3;
        solar.sys.planets[2].num_planets = 2;
        solar.enter_inner(2);
        solar.enter_orbit(Body::Moon { planet: 2, moon: 1 });
        world.current_star = Some(StarId(99));
        assert_eq!(solar.selected_location().node_key(31), (82, 2, 2, 31));
        let masks = [u32::MAX, 1 << 31, 0x1234_5678];
        solar.sys.info.planet_info.scan_retrieve_mask = masks;
        put_planet_info(&mut world, &solar);
        solar.sys.info.planet_info.scan_retrieve_mask = [0; 3];
        get_planet_info(&mut world, &mut solar);
        assert_eq!(solar.sys.info.planet_info.scan_retrieve_mask, masks);
        solar.enter_orbit(Body::Planet(2));
        get_planet_info(&mut world, &mut solar);
        assert_eq!(solar.sys.info.planet_info.scan_retrieve_mask, [0; 3]);
        solar.enter_orbit(Body::Moon { planet: 2, moon: 1 });
        get_planet_info(&mut world, &mut solar);
        assert_eq!(solar.sys.info.planet_info.scan_retrieve_mask, masks);
    }
}
