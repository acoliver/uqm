//! Typed debug equipment changes, independent of rendering and live publication.

use super::*;
use crate::engine::{world::SisState, StoryKey, World};

pub(super) fn equip_flagship(world: &mut World) -> bool {
    if world.activity() as u8 as u32 == SUPER_MELEE {
        return false;
    }
    world.data.flagship.drive_slots.fill(FUSION_THRUSTER as u8);
    world.data.flagship.jet_slots.fill(TURNING_JETS as u8);
    world.set_story_state(
        StoryKey::LANDER_SHIELDS,
        (1 << EARTHQUAKE_DISASTER)
            | (1 << BIOLOGICAL_DISASTER)
            | (1 << LIGHTNING_DISASTER)
            | (1 << LAVASPOT_DISASTER),
    );
    for key in [
        StoryKey::IMPROVED_LANDER_SPEED,
        StoryKey::IMPROVED_LANDER_CARGO,
        StoryKey::IMPROVED_LANDER_SHOT,
    ] {
        world.set_story_state(key, 1);
    }
    let bomb = world.story_state(StoryKey::CHMMR_BOMB_STATE) >= 2;
    let sis = &mut world.data.flagship;
    if bomb {
        sis.module_slots[NUM_BOMB_MODULES as usize..].copy_from_slice(&[
            HIGHEFF_FUELSYS as u8,
            CREW_POD as u8,
            SHIVA_FURNACE as u8,
            SHIVA_FURNACE as u8,
            CANNON_WEAPON as u8,
            SHIVA_FURNACE as u8,
        ]);
    } else {
        sis.module_slots = [
            HIGHEFF_FUELSYS as u8,
            HIGHEFF_FUELSYS as u8,
            CREW_POD as u8,
            CREW_POD as u8,
            CREW_POD as u8,
            CREW_POD as u8,
            CREW_POD as u8,
            STORAGE_BAY as u8,
            SHIVA_FURNACE as u8,
            SHIVA_FURNACE as u8,
            DYNAMO_UNIT as u8,
            TRACKING_SYSTEM as u8,
            TRACKING_SYSTEM as u8,
            SHIVA_FURNACE as u8,
            CANNON_WEAPON as u8,
            CANNON_WEAPON as u8,
        ];
        sis.num_landers = MAX_LANDERS as u8;
    }
    sis.fuel_on_board = FUEL_RESERVE;
    sis.crew_enlisted = 0;
    for &module in &sis.module_slots {
        match u32::from(module) {
            CREW_POD => {
                sis.crew_enlisted = sis.crew_enlisted.wrapping_add(CREW_POD_CAPACITY as u16)
            }
            FUEL_TANK => sis.fuel_on_board = sis.fuel_on_board.wrapping_add(FUEL_TANK_CAPACITY),
            HIGHEFF_FUELSYS => {
                sis.fuel_on_board = sis.fuel_on_board.wrapping_add(HEFUEL_TANK_CAPACITY)
            }
            _ => {}
        }
    }
    true
}

pub(super) fn update_flight(sis: &SisState, system: &mut SOLARSYS_STATE) {
    system.max_ship_speed = (5 * IP_SHIP_THRUST_INCREMENT) as u8;
    for i in 0..NUM_DRIVE_SLOTS as usize {
        // Preserve the original C boolean subscript, including its unusual slot choice.
        if sis.drive_slots[usize::from(i == FUSION_THRUSTER as usize)] != 0 {
            system.max_ship_speed = system
                .max_ship_speed
                .wrapping_add(IP_SHIP_THRUST_INCREMENT as u8);
        }
    }
    system.turn_wait = IP_SHIP_TURN_WAIT as u8;
    for &jet in &sis.jet_slots {
        if u32::from(jet) == TURNING_JETS {
            system.turn_wait = system.turn_wait.wrapping_sub(IP_SHIP_TURN_DECREMENT as u8);
        }
    }
}

pub(super) fn clear_escorts(world: &mut crate::engine::World) {
    let mut ships = world.built_ships();
    let mut next = ships.head();
    while let Some(id) = next {
        next = ships.succ(id);
        ships.remove(id);
        ships.free(id);
    }
}

#[cfg(not(test))]
pub(super) fn reveal_spheres(world: &mut World) {
    let mut fleets = world.avail_races();
    let mut next = fleets.head();
    while let Some(id) = next {
        next = fleets.succ(id);
        let fleet = fleets.get_mut(id);
        if fleet.actual_strength != COUNT::MAX && fleet.known_strength != fleet.actual_strength {
            fleet.known_strength = fleet.actual_strength;
            fleet.known_loc = fleet.loc;
        }
    }
}

#[cfg(not(test))]
pub(super) fn activate_fleets(world: &mut World) {
    let mut fleets = world.avail_races();
    let mut next = fleets.head();
    while let Some(id) = next {
        next = fleets.succ(id);
        let fleet = fleets.get_mut(id);
        if !fleet.icons.is_null() {
            fleet.allied_state = GOOD_GUY as _;
        }
    }
}

#[cfg(test)]
mod equipment_tests {
    use super::*;
    use crate::engine::{StoryKey, World};
    fn world() -> World {
        let mut world = World::new();
        world.story = Some(crate::engine::world::story::StoryState::new());
        world.set_activity(IN_INTERPLANETARY as u16);
        world
    }
    #[test]
    fn equips_normal_flagship_and_lander_upgrades() {
        let mut world = world();
        assert!(equip_flagship(&mut world));
        let sis = &world.data.flagship;
        assert_eq!(sis.drive_slots, [FUSION_THRUSTER as u8; 11]);
        assert_eq!(sis.jet_slots, [TURNING_JETS as u8; 8]);
        assert_eq!(sis.crew_enlisted, 5 * CREW_POD_CAPACITY as u16);
        assert_eq!(sis.fuel_on_board, FUEL_RESERVE + 2 * HEFUEL_TANK_CAPACITY);
        assert_eq!(sis.num_landers, MAX_LANDERS as u8);
        assert_eq!(world.story_state(StoryKey::LANDER_SHIELDS), 15);
        for key in [
            StoryKey::IMPROVED_LANDER_SPEED,
            StoryKey::IMPROVED_LANDER_CARGO,
            StoryKey::IMPROVED_LANDER_SHOT,
        ] {
            assert_eq!(world.story_state(key), 1);
        }
    }
    #[test]
    fn preserves_bomb_slots_and_does_not_add_landers() {
        let mut world = world();
        world.set_story_state(StoryKey::CHMMR_BOMB_STATE, 2);
        world.data.flagship.module_slots.fill(42);
        world.data.flagship.num_landers = 2;
        assert!(equip_flagship(&mut world));
        let sis = &world.data.flagship;
        assert_eq!(
            &sis.module_slots[..NUM_BOMB_MODULES as usize],
            &[42; NUM_BOMB_MODULES as usize]
        );
        assert_eq!(sis.num_landers, 2);
        assert_eq!(sis.crew_enlisted, CREW_POD_CAPACITY as u16);
        assert_eq!(sis.fuel_on_board, FUEL_RESERVE + HEFUEL_TANK_CAPACITY);
    }
    #[test]
    fn flight_updates_preserve_the_original_boolean_drive_subscript() {
        let mut world = world();
        // SAFETY: this record contains only integers, nullable pointers and optional callbacks.
        let mut system: SOLARSYS_STATE = unsafe { std::mem::zeroed() };
        world.data.flagship.drive_slots.fill(0);
        world.data.flagship.drive_slots[2] = 255;
        world.data.flagship.jet_slots.fill(255);
        update_flight(&world.data.flagship, &mut system);
        assert_eq!(system.max_ship_speed, (5 * IP_SHIP_THRUST_INCREMENT) as u8);
        assert_eq!(system.turn_wait, IP_SHIP_TURN_WAIT as u8);
        world.data.flagship.drive_slots[0] = 1;
        world.data.flagship.jet_slots.fill(TURNING_JETS as u8);
        update_flight(&world.data.flagship, &mut system);
        assert_eq!(
            system.max_ship_speed,
            ((5 + NUM_DRIVE_SLOTS - 1) * IP_SHIP_THRUST_INCREMENT) as u8
        );
        assert_eq!(
            system.turn_wait,
            (IP_SHIP_TURN_WAIT - NUM_JET_SLOTS * IP_SHIP_TURN_DECREMENT) as u8
        );
    }

    #[test]
    fn leaves_melee_untouched() {
        let mut world = world();
        world.set_activity(SUPER_MELEE as u16);
        assert!(!equip_flagship(&mut world));
        assert_eq!(world.data.flagship.drive_slots, [0; 11]);
        assert_eq!(world.story_state(StoryKey::LANDER_SHIELDS), 0);
    }
}
