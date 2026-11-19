//! Typed debug equipment changes, independent of rendering and live publication.

use super::*;
use crate::engine::{world::SisState, StoryKey, World};
use crate::planet_side::hazards::HazardKind;
use crate::starbase::ids::{
    IP_SHIP_THRUST_INCREMENT, IP_SHIP_TURN_DECREMENT, IP_SHIP_TURN_WAIT, MAX_LANDERS,
    NUM_BOMB_MODULES, NUM_DRIVE_SLOTS,
};
use crate::starbase::shipyard::CREW_POD_CAPACITY;
use crate::starbase::sis_modules::{
    ModuleType, FUEL_RESERVE, FUEL_TANK_CAPACITY, HEFUEL_TANK_CAPACITY,
};

const FUSION_THRUSTER: u8 = ModuleType::FusionThruster as u8;
const TURNING_JETS: u8 = ModuleType::TurningJets as u8;
const CREW_POD: u8 = ModuleType::CrewPod as u8;
const STORAGE_BAY: u8 = ModuleType::StorageBay as u8;
const FUEL_TANK: u8 = ModuleType::FuelTank as u8;
const HIGHEFF_FUELSYS: u8 = ModuleType::HighEffFuelSys as u8;
const DYNAMO_UNIT: u8 = ModuleType::DynamoUnit as u8;
const SHIVA_FURNACE: u8 = ModuleType::ShivaFurnace as u8;
const CANNON_WEAPON: u8 = ModuleType::CannonWeapon as u8;
const TRACKING_SYSTEM: u8 = ModuleType::TrackingSystem as u8;

pub(super) fn equip_flagship(world: &mut World) -> bool {
    if world.activity() as u8 == ActivityKind::SuperMelee as u8 {
        return false;
    }
    world.data.flagship.drive_slots.fill(FUSION_THRUSTER);
    world.data.flagship.jet_slots.fill(TURNING_JETS);
    world.set_story_state(
        StoryKey::LANDER_SHIELDS,
        u32::from(
            HazardKind::Earthquake.shield_bit()
                | HazardKind::Biological.shield_bit()
                | HazardKind::Lightning.shield_bit()
                | HazardKind::Lava.shield_bit(),
        ),
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
        sis.module_slots[usize::from(NUM_BOMB_MODULES)..].copy_from_slice(&[
            HIGHEFF_FUELSYS,
            CREW_POD,
            SHIVA_FURNACE,
            SHIVA_FURNACE,
            CANNON_WEAPON,
            SHIVA_FURNACE,
        ]);
    } else {
        sis.module_slots = [
            HIGHEFF_FUELSYS,
            HIGHEFF_FUELSYS,
            CREW_POD,
            CREW_POD,
            CREW_POD,
            CREW_POD,
            CREW_POD,
            STORAGE_BAY,
            SHIVA_FURNACE,
            SHIVA_FURNACE,
            DYNAMO_UNIT,
            TRACKING_SYSTEM,
            TRACKING_SYSTEM,
            SHIVA_FURNACE,
            CANNON_WEAPON,
            CANNON_WEAPON,
        ];
        sis.num_landers = MAX_LANDERS;
    }
    sis.fuel_on_board = FUEL_RESERVE;
    sis.crew_enlisted = 0;
    for &module in &sis.module_slots {
        match module {
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

pub(super) fn update_flight(
    sis: &SisState,
    system: &mut crate::engine::world::generation::SolarFlight,
) {
    system.max_ship_speed = 5 * IP_SHIP_THRUST_INCREMENT;
    for i in 0..NUM_DRIVE_SLOTS {
        // Preserve the original C boolean subscript, including its unusual slot choice.
        if sis.drive_slots[usize::from(i == usize::from(FUSION_THRUSTER))] != 0 {
            system.max_ship_speed = system.max_ship_speed.wrapping_add(IP_SHIP_THRUST_INCREMENT);
        }
    }
    system.turn_wait = IP_SHIP_TURN_WAIT;
    for &jet in &sis.jet_slots {
        if jet == TURNING_JETS {
            system.turn_wait = system.turn_wait.wrapping_sub(IP_SHIP_TURN_DECREMENT);
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

pub(super) fn reveal_spheres(world: &mut World) {
    let mut fleets = world.avail_races();
    let mut next = fleets.head();
    while let Some(id) = next {
        next = fleets.succ(id);
        let fleet = fleets.get_mut(id);
        if fleet.actual_strength != u16::MAX && fleet.known_strength != fleet.actual_strength {
            fleet.known_strength = fleet.actual_strength;
            fleet.known_loc = fleet.loc;
        }
    }
}

pub(super) fn activate_fleets(world: &mut World) {
    let mut fleets = world.avail_races();
    let mut next = fleets.head();
    while let Some(id) = next {
        next = fleets.succ(id);
        let fleet = fleets.get_mut(id);
        if !fleet.icons.is_null() {
            fleet.allied_state = crate::comm::dispatch::GOOD_GUY;
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
        world.set_activity(ActivityKind::InInterplanetary as u16);
        world
    }
    #[test]
    fn equips_normal_flagship_and_lander_upgrades() {
        let mut world = world();
        assert!(equip_flagship(&mut world));
        let sis = &world.data.flagship;
        assert_eq!(sis.drive_slots, [FUSION_THRUSTER; 11]);
        assert_eq!(sis.jet_slots, [TURNING_JETS; 8]);
        assert_eq!(sis.crew_enlisted, 5 * CREW_POD_CAPACITY as u16);
        assert_eq!(sis.fuel_on_board, FUEL_RESERVE + 2 * HEFUEL_TANK_CAPACITY);
        assert_eq!(sis.num_landers, MAX_LANDERS);
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
            &sis.module_slots[..usize::from(NUM_BOMB_MODULES)],
            &[42; NUM_BOMB_MODULES as usize]
        );
        assert_eq!(sis.num_landers, 2);
        assert_eq!(sis.crew_enlisted, CREW_POD_CAPACITY as u16);
        assert_eq!(sis.fuel_on_board, FUEL_RESERVE + HEFUEL_TANK_CAPACITY);
    }
    #[test]
    fn flight_updates_preserve_the_original_boolean_drive_subscript() {
        let mut world = world();
        let mut system = crate::engine::world::generation::SolarFlight::default();
        world.data.flagship.drive_slots.fill(0);
        world.data.flagship.drive_slots[2] = 255;
        world.data.flagship.jet_slots.fill(255);
        update_flight(&world.data.flagship, &mut system);
        assert_eq!(system.max_ship_speed, 5 * IP_SHIP_THRUST_INCREMENT);
        assert_eq!(system.turn_wait, IP_SHIP_TURN_WAIT);
        world.data.flagship.drive_slots[0] = 1;
        world.data.flagship.jet_slots.fill(TURNING_JETS);
        update_flight(&world.data.flagship, &mut system);
        assert_eq!(
            system.max_ship_speed,
            (5 + NUM_DRIVE_SLOTS as u8 - 1) * IP_SHIP_THRUST_INCREMENT
        );
        assert_eq!(
            system.turn_wait,
            IP_SHIP_TURN_WAIT - crate::starbase::ids::NUM_JET_SLOTS as u8 * IP_SHIP_TURN_DECREMENT
        );
    }

    #[test]
    fn leaves_melee_untouched() {
        let mut world = world();
        world.set_activity(ActivityKind::SuperMelee as u16);
        assert!(!equip_flagship(&mut world));
        assert_eq!(world.data.flagship.drive_slots, [0; 11]);
        assert_eq!(world.story_state(StoryKey::LANDER_SHIELDS), 0);
    }
}
