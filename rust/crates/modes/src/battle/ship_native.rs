//! Native ship motion and collision callbacks ported from ship.c.

use crate::battle::context_access::ContextOps;
use crate::battle::tactrans_native::engine::{ship_death, ship_transition};
use crate::starbase::ids::STATUS_WIDTH;
use crate::supermelee::native::battle_picker::live::get_initial_melee_star_ships;
use uqm_content::ships::DescriptorLoader;
use uqm_engine::battle::ai_constants::{IGNORE_SIMILAR, LEFT, RIGHT, SPECIAL, THRUST, WEAPON};
use uqm_engine::battle::flags::{
    APPEARING, CHANGING, IN_LAST_BATTLE, NORMAL_LIFE, NPC_PLAYER_NUM, PLAYER_SHIP,
};
use uqm_engine::battle::STAMP_PRIMITIVE;
use uqm_engine::battle::{CollisionHook, ElementHook};
use uqm_engine::graphics::geometry::Point;
use uqm_engine::math::battle_types::{MAX_REDUCTION, ONE_SHIFT};
use uqm_engine::world::activity::activity_kinds::{IN_ENCOUNTER, SUPER_MELEE};

const INPUT_FLAGS: u16 = (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as u16;

fn input_flags(old: u16, input: u8, appearing: bool) -> u16 {
    (old & !INPUT_FLAGS)
        | if appearing {
            0
        } else {
            u16::from(input) & INPUT_FLAGS
        }
}

fn spawn_crew(activity: u8, crew: u16, current: u16, maximum: u16) -> u16 {
    if activity == IN_ENCOUNTER as u8 || activity == IN_LAST_BATTLE as u8 {
        (if crew != 0 { crew } else { current }).min(maximum)
    } else {
        current
    }
}

fn spawn_coordinate(random: u32, extent: i32) -> i16 {
    let aligned = (i32::from(random as u16) % extent) as i16 & !((1 << ONE_SHIFT) - 1);
    let value = i32::from(aligned);
    (if value < 0 {
        value + extent
    } else if value >= extent {
        value - extent
    } else {
        value
    }) as u16 as i16
}

fn install_ship_hooks(
    cx: &mut uqm_engine::battle::Cx<'_>,
    element: uqm_engine::battle::ElementId,
    ship: uqm_engine::battle::ShipId,
) {
    cx.element_mut(element).pre = Some(ElementHook::Native(engine::ship_preprocess));
    cx.element_mut(element).post = Some(ElementHook::Native(engine::ship_postprocess));
    cx.element_mut(element).death = Some(ElementHook::Native(ship_death));
    cx.element_mut(element).collide =
        Some(CollisionHook::Native(uqm_engine::battle::motion::collision));
    cx.element_mut(element).velocity.zero();
    cx.set_parent(element, Some(ship));
    crate::battle::process_native::dispatch::record_spawn(
        cx.ship_pointer(ship) as usize,
        cx.ship_state(ship).player,
        cx.ship(ship).species_id,
    );
    cx.set_target(element, None);
}

pub(crate) mod engine {
    use super::*;
    use crate::battle::status_native::post_process_status;
    use uqm_content::ships::native::normalize_facing;
    use uqm_engine::battle::input::NUM_PLAYERS;
    use uqm_engine::battle::{Cx, ElementId, ShipId};

    pub use uqm_engine::battle::hooks::animation_preprocess;

    pub fn ship_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ship = cx.owner(e);
        let mut flags = input_flags(
            cx.ship(ship).cur_status_flags,
            cx.ship(ship).ship_input_state,
            cx.element(e).flags & APPEARING as u16 != 0,
        );
        if cx.element(e).flags & APPEARING as u16 != 0 {
            let crew = cx.desc(ship).ship_info.crew_level;
            cx.element_mut(e).set_hit_points(crew);
            let activity = cx.world.activity() as u8;
            let player = cx.element(e).player;
            if player == NPC_PLAYER_NUM as i16 && activity == IN_LAST_BATTLE as u8 {
                cx.draw_final_battle_captain(ship);
            } else if activity <= IN_ENCOUNTER as u8 {
                cx.init_ship_status(ship, None);
                cx.invoke_ship_process(ship, false);
                if cx.target(e).is_none() {
                    ship_transition(cx, e);
                } else {
                    cx.set_target(e, None);
                    cx.start_battle_song_if_ready(ship);
                }
                return;
            } else {
                let (w, h) = cx.screen_dimensions();
                cx.element_mut(e).current.location.x =
                    (((w - STATUS_WIDTH) << (ONE_SHIFT + MAX_REDUCTION)) >> 1) as i16;
                cx.element_mut(e).current.location.y =
                    ((h << (ONE_SHIFT + MAX_REDUCTION)) >> 1) as i16;
                cx.element_mut(e).next.location = cx.element(e).current.location;
                cx.init_intersect(e);
                if cx.hyper_transition(e) {
                    return;
                }
            }
        }
        cx.ship_state_mut(ship).status = flags;
        if cx.ship(ship).energy_counter != 0 {
            cx.ship_state_mut(ship).energy_counter -= 1;
        } else if cx.desc(ship).ship_info.energy_level < cx.desc(ship).ship_info.max_energy
            || (cx.desc(ship).characteristics.energy_regeneration as i8) < 0
        {
            let delta = cx.desc(ship).characteristics.energy_regeneration as i8 as i16;
            cx.delta_energy(e, delta);
        }
        if cx.invoke_ship_process(ship, false) {
            flags = cx.ship(ship).cur_status_flags;
        }
        if cx.element(e).turn_wait() != 0 {
            let wait = cx.element(e).turn_wait() - 1;
            cx.element_mut(e).set_turn_wait(wait);
        } else if flags & (LEFT | RIGHT) as u16 != 0 {
            let facing = normalize_facing(
                i32::from(cx.ship(ship).ship_facing)
                    + if flags & LEFT as u16 != 0 { -1 } else { 1 },
            );
            cx.ship_state_mut(ship).facing = facing;
            let frame = cx.absolute_frame(cx.element(e).next.image.frame.view(), facing);
            cx.set_frame(e, true, frame);
            cx.element_mut(e).flags |= CHANGING as u16;
            let wait = cx.desc(ship).characteristics.turn_wait;
            cx.element_mut(e).set_turn_wait(wait);
        }
        if cx.element(e).thrust_wait() != 0 {
            let wait = cx.element(e).thrust_wait() - 1;
            cx.element_mut(e).set_thrust_wait(wait);
        } else if flags & THRUST as u16 != 0 {
            let facing = cx.ship(ship).ship_facing;
            let status = cx.ship(ship).cur_status_flags;
            let characteristics = cx.desc(ship).characteristics;
            let result = uqm_engine::battle::motion::thrust(
                &mut cx.element_mut(e).velocity,
                facing,
                status,
                characteristics.thrust_increment,
                characteristics.max_thrust,
            );
            cx.ship_state_mut(ship).status &= !uqm_engine::battle::flags::SHIP_SPEED_FLAGS;
            cx.ship_state_mut(ship).status |= result;
            cx.element_mut(e)
                .set_thrust_wait(characteristics.thrust_wait);
            if !cx.object_cloaked(e) && cx.world.activity() as u8 <= IN_ENCOUNTER as u8 {
                cx.spawn_ion_trail(e);
            }
        }
        if cx.world.activity() as u8 <= IN_ENCOUNTER as u8 {
            cx.preprocess_status(e);
        }
    }

    pub fn ship_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).hit_points() == 0 {
            return;
        }
        let ship = cx.owner(e);
        if cx.ship(ship).weapon_counter != 0 {
            cx.ship_state_mut(ship).weapon_counter -= 1;
        } else if cx.ship(ship).cur_status_flags & WEAPON as u16 != 0
            && cx.delta_energy(
                e,
                -i16::from(cx.desc(ship).characteristics.weapon_energy_cost),
            )
        {
            let weapons = uqm_engine::battle::Cx::spawn_weapons(cx, ship, cx.weapon_origin(e));
            if !weapons.is_empty() {
                let mut played = false;
                for weapon in weapons.into_iter().flatten() {
                    cx.set_parent(weapon, Some(ship));
                    if !played {
                        cx.process_ship_sound(ship, 0, weapon);
                        played = true;
                    }
                    cx.put(weapon);
                }
                if !played {
                    cx.process_ship_sound(ship, 0, e);
                }
            }
            cx.ship_state_mut(ship).weapon_counter = cx.desc(ship).characteristics.weapon_wait;
        }
        if cx.ship(ship).special_counter != 0 {
            cx.ship_state_mut(ship).special_counter -= 1;
        }
        cx.invoke_ship_process(ship, true);
        if cx.world.activity() as u8 <= IN_ENCOUNTER as u8 {
            post_process_status(cx, e);
        }
    }

    pub fn spawn_ship(cx: &mut Cx<'_>, ship: ShipId) -> u32 {
        let Some(descriptor) = uqm_engine::battle::RaceDesc::load(
            &cx.kernel.content(),
            cx.world,
            cx.ship(ship).species_id,
            true,
        ) else {
            return 0;
        };
        cx.attach_descriptor(ship, descriptor);
        cx.ship_state_mut(ship).input = 0;
        cx.ship_state_mut(ship).status = 0;
        cx.ship_state_mut(ship).old_status = 0;
        let info = cx.race_stats(ship).ship_info;
        cx.race_stats_mut(ship).ship_info.crew_level = spawn_crew(
            cx.world.activity() as u8,
            cx.ship_state(ship).crew,
            info.crew_level,
            info.max_crew,
        );
        cx.ship_state_mut(ship).energy_counter = 0;
        cx.ship_state_mut(ship).weapon_counter = 0;
        cx.ship_state_mut(ship).special_counter = 0;
        let mut element = cx.optional_ship_element(ship);
        if element.is_none() {
            element = cx.alloc_element();
            if let Some(id) = element {
                cx.insert_before(id, cx.elements().head());
            }
        }
        cx.set_ship_element(ship, element);
        let Some(id) = element else {
            return 0;
        };
        cx.element_mut(id).player = cx.ship_state(ship).player;
        cx.element_mut(id).hits = 0;
        cx.element_mut(id).mass = cx.race_stats(ship).characteristics.ship_mass;
        cx.element_mut(id).flags = (APPEARING | PLAYER_SHIP | IGNORE_SIMILAR) as u16;
        cx.element_mut(id).turn = 0;
        cx.element_mut(id).thrust = 0;
        cx.element_mut(id).life = NORMAL_LIFE as u16;
        cx.element_mut(id).color_cycle = 0;
        cx.primitive_style_mut(id).kind = STAMP_PRIMITIVE;
        cx.set_frames(id, false, cx.ship_frame_set(ship, 0));
        let (width, height) = cx.screen_dimensions();
        let width = (width - STATUS_WIDTH) << (ONE_SHIFT + MAX_REDUCTION);
        let height = height << (ONE_SHIFT + MAX_REDUCTION);
        if cx.element(id).player == NPC_PLAYER_NUM as i16
            && cx.world.activity() as u8 == IN_LAST_BATTLE as u8
        {
            cx.ship_state_mut(ship).facing = 0;
            let frame = cx.absolute_frame(cx.ship_frame(ship, 0), cx.ship_state(ship).facing);
            cx.set_frame(id, false, frame);
            cx.element_mut(id).current.location = Point {
                x: (width >> 1) as i16,
                y: (height >> 1) as i16,
            };
            cx.element_mut(id).life += 1;
        } else {
            cx.ship_state_mut(ship).facing = normalize_facing(cx.random() as i32);
            if cx.world.in_hqspace() {
                let facing = cx.world.data.game.ship_facing;
                cx.ship_state_mut(ship).facing = if facing > 0 { facing - 1 } else { facing };
            }
            let frame = cx.absolute_frame(cx.ship_frame(ship, 0), cx.ship_state(ship).facing);
            cx.set_frame(id, false, frame);
            loop {
                cx.element_mut(id).current.location.x = spawn_coordinate(cx.random(), width);
                cx.element_mut(id).current.location.y = spawn_coordinate(cx.random(), height);
                if !cx.gravity(id) && !cx.matter_conflict(id) {
                    break;
                }
            }
        }
        install_ship_hooks(cx, id, ship);
        1
    }

    pub fn next_star_ship(cx: &mut Cx<'_>, mut last: Option<ShipId>, side: u16) -> u32 {
        let activity = cx.world.activity_control();
        let runtime = cx.kernel.input_runtime();
        let selected = crate::battle::context_access::reader_legacy::get_encounter_star_ship(
            cx,
            runtime.input(),
            activity,
            last,
            side,
        );
        if let Some(ship) = selected {
            if let Some(previous) = last {
                if ship == previous {
                    last = None;
                } else {
                    cx.set_ship_element(ship, cx.optional_ship_element(previous));
                }
            }
            if spawn_ship(cx, ship) == 0 {
                return 0;
            }
        }
        if let Some(last) = last {
            cx.set_ship_element(last, None);
        }
        selected.is_some() as u32
    }

    pub fn initial_star_ships(cx: &mut Cx<'_>) -> u32 {
        if cx.world.activity() as u8 == SUPER_MELEE as u8 {
            let runtime = cx.kernel.input_runtime();
            let Some(ships) = get_initial_melee_star_ships(cx, runtime.input()) else {
                return 0;
            };
            for rank in 0..NUM_PLAYERS as u16 {
                let player = cx.kernel.player_order(rank);
                let ship = ships[player as usize]
                    .unwrap_or_else(|| panic!("successful picker returned no initial ship"));
                if spawn_ship(cx, ship) == 0 {
                    return 0;
                }
            }
        } else {
            for player in (0..NUM_PLAYERS as u16).rev() {
                if next_star_ship(cx, None, player) == 0 {
                    return 0;
                }
            }
        }
        1
    }
}

pub use engine::{
    animation_preprocess, initial_star_ships, next_star_ship, ship_postprocess, ship_preprocess,
    spawn_ship,
};

#[cfg(test)]
mod tests {
    use uqm_engine::battle::flags::{LOW_ON_ENERGY, SHIP_AT_MAX_SPEED};

    use super::*;

    #[test]
    fn appearing_ship_does_not_process_controls_and_preserves_non_input_status() {
        let state = (LEFT | LOW_ON_ENERGY | SHIP_AT_MAX_SPEED) as u16;
        assert_eq!(
            input_flags(state, (RIGHT | WEAPON) as u8, true),
            (LOW_ON_ENERGY | SHIP_AT_MAX_SPEED) as u16
        );
        assert_eq!(
            input_flags(state, (RIGHT | WEAPON) as u8, false),
            (LOW_ON_ENERGY | SHIP_AT_MAX_SPEED | RIGHT | WEAPON) as u16
        );
    }

    #[test]
    fn encounter_crew_and_sis_zero_sentinel_match_spawn_policy() {
        assert_eq!(spawn_crew(IN_ENCOUNTER as u8, 0, 90, 42), 42);
        assert_eq!(spawn_crew(IN_LAST_BATTLE as u8, 5, 90, 42), 5);
        assert_eq!(spawn_crew(SUPER_MELEE as u8, 5, 90, 42), 90);
    }

    #[test]
    fn spawn_position_narrows_random_before_align_and_wrap() {
        assert_eq!(spawn_coordinate(0x1234_0007, 8192), 4);
        assert_eq!(spawn_coordinate(8191, 8192), 8188);
        assert_eq!(spawn_coordinate(8192, 8192), 0);
        assert_eq!(spawn_coordinate(0x9000, 38912), 10240);
    }
}
