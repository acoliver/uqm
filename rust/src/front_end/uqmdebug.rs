//! Debug cheats, universe inspection and graphics-context diagnostics.
#![cfg(feature = "debug-process")]

use crate::engine::{
    battle::{Cx, ElementId, ShipId},
    observe::Io,
    world::generation::Body,
    Kernel, StoryKey, World,
};
use crate::graphics::context_abi::ContextLease;
use crate::{navigation::abi::*, planets::solar_system::SolarSystem};

mod catalog;
pub use catalog::uqm_debug_creature_bio_value;
mod colors;
mod equipment;
mod labels;
pub use labels::*;
mod traversal;
pub use traversal::*;
mod output;
pub use output::*;
mod contexts;
pub use contexts::debug_contexts;

#[cfg(test)]
mod supplied_tests;

pub fn debug_key_pressed_synchronous(
    cx: &mut Cx<'_>,
    crew: impl FnOnce(&mut Cx<'_>, ElementId, i16) -> bool,
    energy: impl FnOnce(&mut Cx<'_>, ElementId, i16) -> bool,
) {
    equip_ship(cx);
    give_devices(cx.world);
    clear_escorts(cx);
    for race in [
        ARILOU_SHIP,
        PKUNK_SHIP,
        VUX_SHIP,
        YEHAT_SHIP,
        MELNORME_SHIP,
        DRUUGE_SHIP,
        ILWRATH_SHIP,
        MYCON_SHIP,
        SLYLANDRO_SHIP,
        UMGAH_SHIP,
        URQUAN_SHIP,
        BLACK_URQUAN_SHIP,
    ] {
        let added = crate::engine::world::fleet::add_escort_ships(
            cx.world,
            race as u16,
            1,
            crate::engine::world::fleet::captain_name,
        );
        if added.is_some() {
            crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 32767, 32767, 32767);
        }
    }
    reset_crew_battle(cx, crew);
    reset_energy_battle(cx, energy);
    cx.kernel.instant_move = !cx.kernel.instant_move;
    show_spheres(cx.world);
    activate_all_ships(cx.world);
}

pub fn forward_to_next_event(cx: &mut Cx<'_>, skip_hee: bool) {
    if crate::time::clock_bridge::rust_clock_is_running(cx.world) == 0 {
        return;
    }
    let mut done = !skip_hee;
    loop {
        let (date, selector) = {
            let events = cx.world.clock.events_ref();
            let Some(id) = events.head() else {
                return;
            };
            let event = events.get(id);
            (
                (event.year_index, event.month_index, event.day_index),
                event.func_index,
            )
        };
        done |= u32::from(selector) != HYPERSPACE_ENCOUNTER_EVENT;
        while {
            let clock = &cx.world.data.game.game_clock;
            (clock.year_index, clock.month_index, clock.day_index) < date
        } {
            crate::time::clock_abi::advance_days(cx, 1);
        }
        if done {
            break;
        }
    }
}

pub fn equip_ship(cx: &mut Cx<'_>) {
    if !equipment::equip_flagship(cx.world) {
        return;
    }
    if let Some(effects) = cx.solar_flight {
        let mut flight = crate::engine::world::generation::SolarFlight::default();
        equipment::update_flight(&cx.world.data.flagship, &mut flight);
        effects.set(Some(flight));
    }
    if crate::navigation::globals::in_hqspace(cx.world) != 0
        || cx.world.activity() as u8 as u32 == IN_INTERPLANETARY
    {
        crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 32767, 32767, 32767);
    }
}

pub fn give_devices(world: &mut World) {
    for key in [
        StoryKey::ROSY_SPHERE_ON_SHIP,
        StoryKey::ARTIFACT_2_ON_SHIP,
        StoryKey::ARTIFACT_3_ON_SHIP,
        StoryKey::SUN_DEVICE_ON_SHIP,
        StoryKey::UTWIG_BOMB_ON_SHIP,
        StoryKey::ULTRON_CONDITION,
        StoryKey::MAIDENS_ON_SHIP,
        StoryKey::TALKING_PET_ON_SHIP,
        StoryKey::AQUA_HELIX_ON_SHIP,
        StoryKey::CLEAR_SPINDLE_ON_SHIP,
        StoryKey::UMGAH_BROADCASTERS_ON_SHIP,
        StoryKey::TAALO_PROTECTOR_ON_SHIP,
        StoryKey::EGG_CASE0_ON_SHIP,
        StoryKey::EGG_CASE1_ON_SHIP,
        StoryKey::EGG_CASE2_ON_SHIP,
        StoryKey::SYREEN_SHUTTLE_ON_SHIP,
        StoryKey::VUX_BEAST_ON_SHIP,
        StoryKey::PORTAL_SPAWNER_ON_SHIP,
        StoryKey::PORTAL_KEY_ON_SHIP,
        StoryKey::BURV_BROADCASTERS_ON_SHIP,
        StoryKey::MOONBASE_ON_SHIP,
        StoryKey::DESTRUCT_CODE_ON_SHIP,
    ] {
        world.set_story_state(key, 1);
    }
}

pub fn clear_escorts(cx: &mut Cx<'_>) {
    equipment::clear_escorts(cx.world);
    crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 32767, 32767, 32767);
}
pub fn show_spheres(world: &mut World) {
    equipment::reveal_spheres(world);
}
pub fn activate_all_ships(world: &mut World) {
    equipment::activate_fleets(world);
}

pub fn find_player_ship(cx: &Cx<'_>, player: i16) -> Option<ShipId> {
    let mut next = cx.elements().head();
    while let Some(element) = next {
        let state = cx.element(element);
        if state.flags & PLAYER_SHIP as u16 != 0 && state.player == player {
            return Some(cx.owner(element));
        }
        next = cx.elements().succ(element);
    }
    None
}

fn refill(cx: &mut Cx<'_>, energy: bool, apply: impl FnOnce(&mut Cx<'_>, ElementId, i16) -> bool) {
    if cx.world.activity() & IN_BATTLE as u16 == 0
        || crate::navigation::globals::in_hqspace(cx.world) != 0
    {
        return;
    }
    let Some(ship) = find_player_ship(cx, RPG_PLAYER_NUM as i16) else {
        return;
    };
    if !cx.has_descriptor(ship) {
        return;
    }
    let info = cx.race_stats(ship).ship_info;
    let delta = if energy {
        i32::from(info.max_energy) - i32::from(info.energy_level)
    } else {
        i32::from(info.max_crew) - i32::from(info.crew_level)
    } as i16;
    let context = ContextLease::require(cx.kernel.status_context);
    context.with_selected(|| apply(cx, cx.ship_element(ship), delta));
}

pub fn reset_crew_battle(cx: &mut Cx<'_>, apply: impl FnOnce(&mut Cx<'_>, ElementId, i16) -> bool) {
    refill(cx, false, apply);
}
pub fn reset_energy_battle(
    cx: &mut Cx<'_>,
    apply: impl FnOnce(&mut Cx<'_>, ElementId, i16) -> bool,
) {
    refill(cx, true, apply);
}

/// Apply the selected debug destination using only the supplied campaign.
pub fn move_flagship(world: &mut World, dimensions: (i32, i32)) {
    let log_x = (((dimensions.0 - 64) << 5) >> 4) * 195;
    let log_y = ((dimensions.1 << 5) >> 4) * 25;
    let game = &mut world.data.game;
    let sis = &mut world.data.flagship;
    if game.autopilot.x == -1 || game.autopilot.y == -1 {
        game.autopilot.x = ((sis.log_x.wrapping_mul(6250) + (log_x >> 1)) / log_x) as i16;
        game.autopilot.y = (9999 - ((sis.log_y.wrapping_mul(625) + (log_y >> 1)) / log_y)) as i16;
    } else {
        sis.log_x = (i32::from(game.autopilot.x) * log_x + (6250 >> 1)) / 6250;
        sis.log_y = ((9999 - i32::from(game.autopilot.y)) * log_y + (625 >> 1)) / 625;
    }
    if world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1 {
        world.current_star = crate::planets::starmap::find_star_id(
            world,
            world.catalog(),
            None,
            world.data.game.autopilot,
            0,
            0,
        );
        if world.current_star.is_some() {
            world.set_story_state(StoryKey::USED_BROADCASTER, 0);
            world.set_activity(world.activity() & !(IN_BATTLE as u16));
            world.data.game.ship_facing = 0;
            world.data.game.ip_planet = 0;
            world.data.game.in_orbit = 0;
            world.set_activity(world.activity() | START_INTERPLANETARY as u16);
        }
    }
    world.data.game.autopilot = POINT { x: -1, y: -1 };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clearing_escort_queue_returns_every_fragment_slot_to_pool() {
        let mut world = World::new();
        let mut original = Vec::new();
        for _ in 0..12 {
            let mut ships = world.built_ships();
            let id = ships.alloc().unwrap();
            ships.put(id);
            original.push(id);
        }
        assert!(world.built_ships().alloc().is_none());
        equipment::clear_escorts(&mut world);
        assert!(world.built_ships_ref().head().is_none());
        assert!(world.built_ships_ref().tail().is_none());
        for id in original {
            assert!(!world.built_ships_ref().contains(id));
        }
        for _ in 0..12 {
            assert!(world.built_ships().alloc().is_some());
        }
        assert!(world.built_ships().alloc().is_none());
    }
    #[test]
    fn debug_names_use_header_indices_and_unknown_fallbacks() {
        assert_eq!(
            event_name(HYPERSPACE_ENCOUNTER_EVENT as u8),
            "HYPERSPACE_ENCOUNTER_EVENT"
        );
        assert_eq!(
            star_presence_string(MELNORME8_DEFINED as u8),
            "Melnorme trader #8"
        );
        assert_eq!(star_presence_string(0), "");
        assert_eq!(body_color_string(CYAN_BODY as u8), "cyan");
        assert_eq!(world_gen_algo_string(CRATERED_ALGO as u8), "CRATERED_ALGO");
        assert_eq!(event_name(u8::MAX), "???");
        assert_eq!(atmosphere_string(u8::MAX), "super thick");
    }
    #[test]
    fn hsv_sectors_preserve_c_rounding_and_alpha() {
        for (hue, (r, g, b)) in [
            (0.0, (255, 0, 0)),
            (60.0, (255, 255, 0)),
            (120.0, (0, 255, 0)),
            (180.0, (0, 255, 255)),
            (240.0, (0, 0, 255)),
            (300.0, (255, 0, 255)),
        ] {
            let color = colors::hsva_to_rgba(hue, 1.0, 1.0, 90);
            assert_eq!((color.r, color.g, color.b, color.a), (r, g, b, 90));
        }
        let color = colors::hsva_to_rgba(30.0, 0.5, 1.0, 100);
        assert_eq!((color.r, color.g, color.b, color.a), (255, 64, 128, 100));
    }
}

#[cfg(test)]
mod traversal_tests;
