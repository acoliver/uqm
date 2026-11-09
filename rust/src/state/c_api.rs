//! Remaining state.c entry points. File storage stays in `ffi`'s sole manager.

use super::ffi::*;
#[cfg(test)]
use crate::engine::World;
use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;

/// Opaque to C. Its address identifies a file, not an independently owned buffer.
#[repr(C)]
pub struct GameStateFile {
    index: c_int,
}

static FILES: [GameStateFile; 3] = [
    GameStateFile { index: 0 },
    GameStateFile { index: 1 },
    GameStateFile { index: 2 },
];

/// # Safety
/// `mode` must be a NUL-terminated C string.
pub unsafe fn open_state_file(
    world: &mut crate::engine::World,
    index: c_int,
    mode: *const c_char,
) -> *mut GameStateFile {
    if !(0..3).contains(&index) || rust_open_state_file(world, index, mode) == 0 {
        return ptr::null_mut();
    }
    ptr::addr_of!(FILES)
        .cast_mut()
        .cast::<GameStateFile>()
        .add(index as usize)
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
pub unsafe fn close_state_file(world: &mut crate::engine::World, file: *mut GameStateFile) {
    rust_close_state_file(world, (*file).index);
}

/// # Safety
/// Called on the game thread, without concurrent state-file access.
pub fn delete_state_file(world: &mut crate::engine::World, index: c_int) {
    rust_delete_state_file(world, index);
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
pub unsafe fn length_state_file(world: &crate::engine::World, file: *mut GameStateFile) -> u32 {
    rust_length_state_file(world, (*file).index) as u32
}

/// # Safety
/// `buffer` must hold `size * count` writable bytes; `file` must be a live handle.
pub unsafe fn read_state_file(
    world: &mut crate::engine::World,
    buffer: *mut c_void,
    size: u16,
    count: u16,
    file: *mut GameStateFile,
) -> c_int {
    ({
        rust_read_state_file(
            world,
            (*file).index,
            buffer.cast(),
            size as usize,
            count as usize,
        )
    }) as c_int
}

/// # Safety
/// `buffer` must hold `size * count` readable bytes; `file` must be a live handle.
pub unsafe fn write_state_file(
    world: &mut crate::engine::World,
    buffer: *const c_void,
    size: u16,
    count: u16,
    file: *mut GameStateFile,
) -> c_int {
    ({
        rust_write_state_file(
            world,
            (*file).index,
            buffer.cast(),
            size as usize,
            count as usize,
        )
    }) as c_int
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
pub unsafe fn seek_state_file(
    world: &mut crate::engine::World,
    file: *mut GameStateFile,
    offset: c_long,
    whence: c_int,
) -> c_int {
    rust_seek_state_file(world, (*file).index, offset as _, whence)
}

pub(crate) mod planets {
    use crate::engine::world::planet_info::{PlanetInfoManager, ScanRetrieveMask};
    use crate::engine::World;
    use crate::planets::solar_system::{BodyLocation, SolarSystem};

    pub(crate) fn location(solar: &SolarSystem) -> BodyLocation {
        solar.body_location(
            solar
                .selected_body()
                .expect("planet info requires a selected body"),
        )
    }

    pub fn uninit_planet_info(world: &mut World) {
        super::rust_uninit_planet_info(world);
    }

    pub fn get_planet_info(world: &mut World, solar: &mut SolarSystem) {
        let location = location(solar);
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
        let location = location(solar);
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
            assert_eq!(location(&solar).node_key(31), (82, 2, 2, 31));
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn handles_share_the_existing_file_manager_and_preserve_io() {
        let mut game = crate::engine::Game::new();
        // SAFETY: serial tests own the published lifecycle until this call returns.
        unsafe {
            crate::app::bridge::with_game(&mut game, || {
                World::current().write(rust_init_game_state);
                assert!(World::current()
                    .write(|world| open_state_file(world, -1, c"wb".as_ptr()))
                    .is_null());
                assert!(World::current()
                    .write(|world| open_state_file(world, 3, c"wb".as_ptr()))
                    .is_null());
                let file =
                    World::current().write(|world| open_state_file(world, 1, c"wb".as_ptr()));
                assert!(!file.is_null());
                let data = [11u8, 22, 33, 44];
                assert_eq!(
                    World::current().write(|world| write_state_file(
                        world,
                        data.as_ptr().cast(),
                        2,
                        2,
                        file
                    )),
                    2
                );
                assert_eq!(
                    World::current().read(|world| length_state_file(world, file)),
                    4
                );
                assert_eq!(
                    World::current().read(|world| rust_length_state_file(world, 1)),
                    4
                );
                assert_eq!(
                    World::current().write(|world| seek_state_file(world, file, -2, 2)),
                    1
                );
                let mut output = [0u8; 2];
                assert_eq!(
                    World::current().write(|world| read_state_file(
                        world,
                        output.as_mut_ptr().cast(),
                        1,
                        2,
                        file
                    )),
                    2
                );
                assert_eq!(output, [33, 44]);
                World::current().write(|world| close_state_file(world, file));
                assert_eq!(
                    World::current().write(|world| open_state_file(world, 1, c"rb".as_ptr())),
                    file
                );
                World::current().write(|world| close_state_file(world, file));
                World::current().write(|world| delete_state_file(world, 1));
            })
        }
    }
}
