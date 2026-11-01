//! Native ship motion and collision callbacks ported from ship.c.

use crate::ships::native::abi::*;

fn velocity_squared(dx: i16, dy: i16) -> u32 {
    (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32
}

const INPUT_FLAGS: STATUS_FLAGS = (LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as STATUS_FLAGS;

fn input_flags(old: STATUS_FLAGS, input: BYTE, appearing: bool) -> STATUS_FLAGS {
    (old & !INPUT_FLAGS)
        | if appearing {
            0
        } else {
            STATUS_FLAGS::from(input) & INPUT_FLAGS
        }
}

fn spawn_crew(activity: BYTE, crew: COUNT, current: COUNT, maximum: COUNT) -> COUNT {
    if activity == IN_ENCOUNTER as BYTE || activity == IN_LAST_BATTLE as BYTE {
        (if crew != 0 { crew } else { current }).min(maximum)
    } else {
        current
    }
}

fn spawn_coordinate(random: u32, extent: i32) -> COORD {
    let aligned = (i32::from(random as COUNT) % extent) as COORD & !((1 << ONE_SHIFT) - 1);
    let value = i32::from(aligned);
    (if value < 0 {
        value + extent
    } else if value >= extent {
        value - extent
    } else {
        value
    }) as COUNT as COORD
}

#[derive(Debug, PartialEq)]
enum Acceleration {
    Normal,
    Beyond,
    Aligned,
    Redirect,
}

fn acceleration(
    desired: u32,
    maximum: u32,
    current: u32,
    gravity: bool,
    aligned: bool,
) -> Acceleration {
    if desired <= maximum {
        Acceleration::Normal
    } else if (gravity && desired <= (18u32 << (ONE_SHIFT + VELOCITY_SHIFT)).pow(2))
        || desired < current
    {
        Acceleration::Beyond
    } else if aligned {
        Acceleration::Aligned
    } else {
        Acceleration::Redirect
    }
}

pub(crate) fn thrust(
    velocity: &mut crate::battle::velocity::VelocityDesc,
    facing: u16,
    status: u16,
    increment: u16,
    maximum: u16,
) -> u16 {
    const MAX_SPEED: u16 = 1 << 7;
    const BEYOND_SPEED: u16 = 1 << 6;
    const GRAVITY: u16 = 1 << 8;
    let current_angle = (i32::from(facing) << 2) as u16 as i32;
    let travel_angle = velocity.travel_angle;
    let mut thrust = increment;
    let maximum_thrust = maximum;
    if thrust == maximum_thrust {
        velocity.set_vector(i32::from(maximum_thrust as i16), facing);
        return MAX_SPEED;
    }
    if travel_angle == current_angle as u16
        && status & (MAX_SPEED | BEYOND_SPEED) != 0
        && status & GRAVITY == 0
    {
        return status & (MAX_SPEED | BEYOND_SPEED);
    }
    thrust = (i32::from(thrust) << 5) as u16;
    let (dx, dy) = velocity.get_current_components();
    let (mut dx, mut dy) = (dx as i16, dy as i16);
    let current_speed = velocity_squared(dx, dy);
    let next_dx = dx.wrapping_add(crate::math::cosine(current_angle, i32::from(thrust)));
    let next_dy = dy.wrapping_add(crate::math::sine(current_angle, i32::from(thrust)));
    let desired = velocity_squared(next_dx, next_dy);
    let max_speed = velocity_squared((i32::from(maximum_thrust) << 5) as i16, 0);
    match acceleration(
        desired,
        max_speed,
        current_speed,
        status & GRAVITY != 0,
        travel_angle == current_angle as u16,
    ) {
        Acceleration::Normal => {
            velocity.set_components(i32::from(next_dx), i32::from(next_dy));
        }
        Acceleration::Beyond => {
            velocity.set_components(i32::from(next_dx), i32::from(next_dy));
            return MAX_SPEED | BEYOND_SPEED;
        }
        Acceleration::Aligned => {
            if current_speed <= max_speed {
                velocity.set_vector(i32::from(maximum_thrust as i16), facing);
            }
            return MAX_SPEED;
        }
        Acceleration::Redirect => {
            let mut next_velocity = *velocity;
            next_velocity.delta_components(
                i32::from(
                    crate::math::cosine(current_angle, i32::from(thrust >> 1)).wrapping_sub(
                        crate::math::cosine(i32::from(travel_angle), i32::from(thrust)),
                    ),
                ),
                i32::from(
                    crate::math::sine(current_angle, i32::from(thrust >> 1)).wrapping_sub(
                        crate::math::sine(i32::from(travel_angle), i32::from(thrust)),
                    ),
                ),
            );
            let components = next_velocity.get_current_components();
            (dx, dy) = (components.0 as i16, components.1 as i16);
            let desired = velocity_squared(dx, dy);
            if desired > max_speed {
                if desired < current_speed {
                    *velocity = next_velocity;
                }
                return MAX_SPEED | BEYOND_SPEED;
            }
            *velocity = next_velocity;
        }
    }
    0
}

#[cfg(not(test))]
fn install_ship_hooks(
    cx: &mut crate::engine::battle::Cx<'_>,
    element: crate::engine::battle::ElementId,
    ship: crate::engine::battle::ShipId,
) {
    cx.element_mut(element).pre = Some(ElementHook::Native(engine::ship_preprocess));
    cx.element_mut(element).post = Some(ElementHook::Native(engine::ship_postprocess));
    cx.element_mut(element).death = Some(ElementHook::Native(ship_death));
    cx.element_mut(element).collide = Some(CollisionHook::Native(engine::collision));
    cx.element_mut(element).velocity.zero();
    cx.set_parent(element, Some(ship));
    crate::battle::process_native::dispatch::record_spawn(
        cx.ship_pointer(ship) as usize,
        cx.ship_state(ship).player,
        cx.ship(ship).SpeciesID,
    );
    cx.set_target(element, None);
}

#[cfg(not(test))]
const GRAVITY_MASS_NATIVE: u16 = 100;

#[cfg(not(test))]
pub(crate) mod engine {
    use super::*;
    use crate::battle::status_native::PostProcessStatus;
    use crate::collections::observation::image;
    use crate::engine::battle::{Cx, ElementId, ShipId};
    use crate::ships::native::normalize_facing;
    use std::{mem, ptr};

    const TARGET_DAMAGED_FOR_1_PT_NATIVE: i16 = 2;
    const TARGET_DAMAGED_FOR_6_PLUS_PT_NATIVE: i16 = 5;

    pub fn animation_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let wait = cx.element(e).turn_wait();
        if wait > 0 {
            cx.element_mut(e).set_turn_wait(wait - 1);
        } else {
            let frame = cx.advance_frame(cx.element(e).current.image.frame);
            cx.set_frame(e, true, frame);
            cx.element_mut(e).flags |= CHANGING as u16;
            let wait = cx.element(e).thrust_wait();
            cx.element_mut(e).set_turn_wait(wait);
        }
    }

    pub fn inertial_thrust(cx: &mut Cx<'_>, e: ElementId) -> STATUS_FLAGS {
        let ship = cx.owner(e);
        let state = cx.ship_state(ship);
        let stats = cx.race_stats(ship).characteristics;
        let mut velocity = cx.element(e).velocity;
        let result = thrust(
            &mut velocity,
            state.facing,
            state.status,
            stats.thrust_increment,
            stats.max_thrust,
        );
        cx.element_mut(e).velocity = velocity;
        result
    }

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
            let activity = cx.world.activity() as BYTE;
            if cx.element(e).player == NPC_PLAYER_NUM as SIZE && activity == IN_LAST_BATTLE as BYTE
            {
                cx.draw_final_battle_captain(ship);
            } else if activity <= IN_ENCOUNTER as BYTE {
                cx.init_ship_status(ship);
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
                    (((w - STATUS_WIDTH as i32) << (ONE_SHIFT + MAX_REDUCTION)) >> 1) as COORD;
                cx.element_mut(e).current.location.y =
                    ((h << (ONE_SHIFT + MAX_REDUCTION)) >> 1) as COORD;
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
            let delta = cx.desc(ship).characteristics.energy_regeneration as i8 as SIZE;
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
                i32::from(cx.ship(ship).ShipFacing) + if flags & LEFT as u16 != 0 { -1 } else { 1 },
            );
            cx.ship_state_mut(ship).facing = facing;
            let frame = cx.absolute_frame(cx.element(e).next.image.frame, facing);
            cx.set_frame(e, true, frame);
            cx.element_mut(e).flags |= CHANGING as u16;
            let wait = cx.desc(ship).characteristics.turn_wait;
            cx.element_mut(e).set_turn_wait(wait);
        }
        if cx.element(e).thrust_wait() != 0 {
            let wait = cx.element(e).thrust_wait() - 1;
            cx.element_mut(e).set_thrust_wait(wait);
        } else if flags & THRUST as u16 != 0 {
            let facing = cx.ship(ship).ShipFacing;
            let status = cx.ship(ship).cur_status_flags;
            let characteristics = cx.desc(ship).characteristics;
            let result = thrust(
                &mut cx.element_mut(e).velocity,
                facing,
                status,
                characteristics.thrust_increment,
                characteristics.max_thrust,
            );
            cx.ship_state_mut(ship).status &=
                !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16);
            cx.ship_state_mut(ship).status |= result;
            cx.element_mut(e)
                .set_thrust_wait(characteristics.thrust_wait);
            if !cx.object_cloaked(e) && cx.world.activity() as BYTE <= IN_ENCOUNTER as BYTE {
                cx.spawn_ion_trail(e);
            }
        }
        if cx.world.activity() as BYTE <= IN_ENCOUNTER as BYTE {
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
                -SIZE::from(cx.desc(ship).characteristics.weapon_energy_cost),
            )
        {
            let weapons = crate::engine::battle::Cx::spawn_weapons(cx, ship, cx.weapon_origin(e));
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
        if cx.world.activity() as BYTE <= IN_ENCOUNTER as BYTE {
            PostProcessStatus(cx, e);
        }
    }

    pub unsafe fn spawn_ship(cx: &mut Cx<'_>, ship: ShipId) -> BOOLEAN {
        let Some(descriptor) = load_ship(cx.ship(ship).SpeciesID, 1) else {
            return 0;
        };
        cx.attach_descriptor(ship, descriptor);
        cx.ship_state_mut(ship).input = 0;
        cx.ship_state_mut(ship).status = 0;
        cx.ship_state_mut(ship).old_status = 0;
        let info = cx.race_stats(ship).ship_info;
        cx.race_stats_mut(ship).ship_info.crew_level = spawn_crew(
            cx.world.activity() as BYTE,
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
        cx.element_mut(id).life = NORMAL_LIFE as COUNT;
        cx.element_mut(id).color_cycle = 0;
        cx.primitive_style_mut(id).kind = gfx_object_STAMP_PRIM as u8;
        cx.set_frames(id, false, cx.ship_frame_set(ship, 0));
        let (width, height) = cx.screen_dimensions();
        let width = (width - STATUS_WIDTH as i32) << (ONE_SHIFT + MAX_REDUCTION);
        let height = height << (ONE_SHIFT + MAX_REDUCTION);
        if cx.element(id).player == NPC_PLAYER_NUM as SIZE
            && cx.world.activity() as BYTE == IN_LAST_BATTLE as BYTE
        {
            cx.ship_state_mut(ship).facing = 0;
            let frame = cx.absolute_frame(cx.ship_frame(ship, 0), cx.ship_state(ship).facing);
            cx.set_frame(id, false, frame);
            cx.element_mut(id).current.location = POINT {
                x: (width >> 1) as COORD,
                y: (height >> 1) as COORD,
            };
            cx.element_mut(id).life += 1;
        } else {
            cx.ship_state_mut(ship).facing = normalize_facing(cx.random() as i32);
            if inHQSpace() != 0 {
                let facing = GlobData.Game_state.ShipFacing;
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

    pub unsafe fn next_star_ship(
        cx: &mut Cx<'_>,
        mut last: Option<ShipId>,
        side: COUNT,
    ) -> BOOLEAN {
        let selected = crate::app::bridge::publish(cx, || {
            crate::battle::context_access::reader_legacy::get_encounter_star_ship(last, side)
        });
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
        selected.is_some() as BOOLEAN
    }

    /// # Safety
    /// Player queues and encounter/melee selection state must be initialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetInitialStarShips() -> BOOLEAN {
        if GlobData.Game_state.CurrentActivity as BYTE == SUPER_MELEE as BYTE {
            let mut ships = [ptr::null_mut(); NUM_PLAYERS as usize];
            if GetInitialMeleeStarShips(ships.as_mut_ptr()) == 0 {
                return 0;
            }
            for i in 0..NUM_PLAYERS as COUNT {
                let player = GetPlayerOrder(i);
                if crate::app::bridge::enter(|cx| {
                    spawn_ship(cx, cx.ship_id(ships[player as usize] as usize))
                }) == 0
                {
                    return 0;
                }
            }
        } else {
            for player in (0..NUM_PLAYERS as COUNT).rev() {
                if crate::app::bridge::enter(|cx| next_star_ship(cx, None, player)) == 0 {
                    return 0;
                }
            }
        }
        1
    }

    pub fn collision(
        cx: &mut Cx<'_>,
        e0: ElementId,
        _p0: &mut POINT,
        e1: ElementId,
        _p1: &mut POINT,
    ) {
        if cx.element(e1).flags & FINITE_LIFE as u16 == 0 {
            cx.element_mut(e0).flags |= COLLISION as u16;
            if u16::from(cx.element(e1).mass_points()) > GRAVITY_MASS_NATIVE {
                let damage = (cx.element(e0).hit_points() >> 2).max(1);
                cx.damage(e0, damage as i16);
                let sound = (TARGET_DAMAGED_FOR_1_PT_NATIVE + (damage >> 1) as i16)
                    .min(TARGET_DAMAGED_FOR_6_PLUS_PT_NATIVE);
                cx.game_sound(sound as u16, e0);
            }
        }
    }
}

#[cfg(not(test))]
pub use engine::{
    animation_preprocess, collision, inertial_thrust, next_star_ship, ship_postprocess,
    ship_preprocess, spawn_ship, GetInitialStarShips,
};

#[cfg(test)]
mod tests {
    #[test]
    fn inertial_thrust_preserves_speed_flags_and_complete_velocity_state() {
        use crate::battle::velocity::VelocityDesc;
        for facing in 0..16 {
            let mut instant = VelocityDesc::new();
            let mut expected = instant;
            expected.set_vector(24, facing);
            assert_eq!(
                super::thrust(&mut instant, facing, 0, 24, 24),
                super::SHIP_AT_MAX_SPEED as u16
            );
            assert_eq!(instant, expected);
            for flags in [
                super::SHIP_AT_MAX_SPEED as u16,
                super::SHIP_BEYOND_MAX_SPEED as u16,
                (super::SHIP_AT_MAX_SPEED | super::SHIP_BEYOND_MAX_SPEED) as u16,
            ] {
                let before = instant;
                assert_eq!(super::thrust(&mut instant, facing, flags, 2, 24), flags);
                assert_eq!(instant, before);
            }
            let mut accelerating = VelocityDesc::new();
            assert_eq!(super::thrust(&mut accelerating, facing, 0, 2, 24), 0);
            let angle = i32::from(facing) << 2;
            let mut expected = VelocityDesc::new();
            expected.set_components(
                i32::from(crate::math::cosine(angle, 64)),
                i32::from(crate::math::sine(angle, 64)),
            );
            assert_eq!(accelerating, expected);
        }
    }

    use super::*;

    #[test]
    fn appearing_ship_does_not_process_controls_and_preserves_non_input_status() {
        let state = (LEFT | LOW_ON_ENERGY | SHIP_AT_MAX_SPEED) as STATUS_FLAGS;
        assert_eq!(
            input_flags(state, (RIGHT | WEAPON) as BYTE, true),
            (LOW_ON_ENERGY | SHIP_AT_MAX_SPEED) as STATUS_FLAGS
        );
        assert_eq!(
            input_flags(state, (RIGHT | WEAPON) as BYTE, false),
            (LOW_ON_ENERGY | SHIP_AT_MAX_SPEED | RIGHT | WEAPON) as STATUS_FLAGS
        );
    }

    #[test]
    fn encounter_crew_and_sis_zero_sentinel_match_spawn_policy() {
        assert_eq!(spawn_crew(IN_ENCOUNTER as BYTE, 0, 90, 42), 42);
        assert_eq!(spawn_crew(IN_LAST_BATTLE as BYTE, 5, 90, 42), 5);
        assert_eq!(spawn_crew(SUPER_MELEE as BYTE, 5, 90, 42), 90);
    }

    #[test]
    fn inertial_thrust_gravity_redirect_and_native_widths_keep_complete_state() {
        use crate::battle::velocity::VelocityDesc;
        for (initial, facing, status, increment, maximum, components, flags) in [
            ((0, -768), 0, 1 << 8, 2, 24, (0, -832), 192),
            ((0, -768), 4, 0, 2, 24, (32, -704), 0),
            ((0, -2000), 4, 0, 2, 24, (32, -1936), 192),
            (
                (i16::MIN as i32, i16::MAX as i32),
                4,
                0,
                u16::MAX - 1,
                u16::MAX,
                (32704, 32767),
                192,
            ),
        ] {
            let mut actual = VelocityDesc::default();
            actual.set_components(initial.0, initial.1);
            let mut expected = VelocityDesc::default();
            expected.set_components(components.0, components.1);
            assert_eq!(
                thrust(&mut actual, facing, status, increment, maximum),
                flags
            );
            assert_eq!(actual, expected, "initial={initial:?}, facing={facing}");
        }
    }

    #[test]
    fn spawn_position_narrows_random_before_align_and_wrap() {
        assert_eq!(spawn_coordinate(0x1234_0007, 8192), 4);
        assert_eq!(spawn_coordinate(8191, 8192), 8188);
        assert_eq!(spawn_coordinate(8192, 8192), 0);
        assert_eq!(spawn_coordinate(0x9000, 38912), 10240);
    }

    #[test]
    fn thrust_branches_prioritize_normal_then_gravity_and_deceleration() {
        assert_eq!(
            acceleration(100, 100, 90, false, true),
            Acceleration::Normal
        );
        assert_eq!(acceleration(101, 100, 90, true, true), Acceleration::Beyond);
        assert_eq!(
            acceleration(101, 100, 102, false, true),
            Acceleration::Beyond
        );
        assert_eq!(
            acceleration(101, 100, 90, false, true),
            Acceleration::Aligned
        );
        assert_eq!(
            acceleration(101, 100, 90, false, false),
            Acceleration::Redirect
        );
        let limit = (18u32 << (ONE_SHIFT + VELOCITY_SHIFT)).pow(2);
        assert_eq!(
            acceleration(limit, 100, 90, true, false),
            Acceleration::Beyond
        );
        assert_eq!(
            acceleration(limit + 1, 100, 90, true, false),
            Acceleration::Redirect
        );
    }

    #[test]
    fn velocity_squared_matches_widened_c_long_arithmetic() {
        assert_eq!(velocity_squared(3, 4), 25);
        assert_eq!(velocity_squared(i16::MIN, 0), 1_073_741_824);
        assert_eq!(velocity_squared(i16::MIN, i16::MIN), 2_147_483_648);
    }
}
