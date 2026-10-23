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

#[cfg(any(not(test), feature = "linked_c_archive"))]
#[allow(non_snake_case)]
mod engine {
    use super::*;
    use crate::battle::sounds::{GameSounds, ProcessSound};
    use crate::battle::status_native::{
        DeltaEnergy, DrawCaptainsWindow, PostProcessStatus, PreProcessStatus,
    };
    use crate::battle::world_elements::do_damage;
    use crate::ships::native::{
        cosine, element, facing_to_angle, init_intersect_points, normalize_facing, object_cloaked,
        put_element, sine, starship,
    };
    use std::{mem, ptr};

    const GRAVITY_MASS_NATIVE: u16 = 100;
    const TARGET_DAMAGED_FOR_1_PT_NATIVE: i16 = 2;
    const TARGET_DAMAGED_FOR_6_PLUS_PT_NATIVE: i16 = 5;

    /// # Safety
    /// Element and attached STARSHIP/RACE_DESC records must be live.
    #[no_mangle]
    pub unsafe extern "C" fn animation_preprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            (*e).next.image.frame = IncFrameIndex((*e).current.image.frame);
            (*e).state_flags |= CHANGING as u16;
            (*e).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_5.next_turn;
        }
    }

    /// # Safety
    /// Element and attached STARSHIP/RACE_DESC records must be live.
    #[no_mangle]
    pub unsafe extern "C" fn inertial_thrust(e: *mut ELEMENT) -> STATUS_FLAGS {
        let ship = starship(e);
        let desc = (*ship).RaceDescPtr;
        let facing = (*ship).ShipFacing;
        let current_angle = facing_to_angle(i32::from(facing)) as COUNT as i32;
        let travel_angle = (*e).velocity.TravelAngle;
        let mut thrust = (*desc).characteristics.thrust_increment;
        let maximum_thrust = (*desc).characteristics.max_thrust;
        if thrust == maximum_thrust {
            SetVelocityVector(&raw mut (*e).velocity, maximum_thrust as i16, facing);
            return SHIP_AT_MAX_SPEED as STATUS_FLAGS;
        }
        if travel_angle == current_angle as u16
            && (*ship).cur_status_flags
                & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as STATUS_FLAGS
                != 0
            && (*ship).cur_status_flags & SHIP_IN_GRAVITY_WELL as STATUS_FLAGS == 0
        {
            return (*ship).cur_status_flags
                & (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as STATUS_FLAGS;
        }
        thrust = (i32::from(thrust) << VELOCITY_SHIFT) as u16;
        let mut dx = 0;
        let mut dy = 0;
        GetCurrentVelocityComponents(&raw mut (*e).velocity, &mut dx, &mut dy);
        let current_speed = velocity_squared(dx, dy);
        let next_dx = dx.wrapping_add(cosine(current_angle, i32::from(thrust)));
        let next_dy = dy.wrapping_add(sine(current_angle, i32::from(thrust)));
        let desired = velocity_squared(next_dx, next_dy);
        let max_speed = velocity_squared((i32::from(maximum_thrust) << VELOCITY_SHIFT) as i16, 0);
        match acceleration(
            desired,
            max_speed,
            current_speed,
            (*ship).cur_status_flags & SHIP_IN_GRAVITY_WELL as STATUS_FLAGS != 0,
            travel_angle == current_angle as COUNT,
        ) {
            Acceleration::Normal => {
                SetVelocityComponents(&raw mut (*e).velocity, next_dx, next_dy);
            }
            Acceleration::Beyond => {
                SetVelocityComponents(&raw mut (*e).velocity, next_dx, next_dy);
                return (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as STATUS_FLAGS;
            }
            Acceleration::Aligned => {
                if current_speed <= max_speed {
                    SetVelocityVector(&raw mut (*e).velocity, maximum_thrust as i16, facing);
                }
                return SHIP_AT_MAX_SPEED as STATUS_FLAGS;
            }
            Acceleration::Redirect => {
                let mut velocity = (*e).velocity;
                DeltaVelocityComponents(
                    &mut velocity,
                    cosine(current_angle, i32::from(thrust >> 1))
                        .wrapping_sub(cosine(i32::from(travel_angle), i32::from(thrust))),
                    sine(current_angle, i32::from(thrust >> 1))
                        .wrapping_sub(sine(i32::from(travel_angle), i32::from(thrust))),
                );
                GetCurrentVelocityComponents(&raw mut velocity, &mut dx, &mut dy);
                let desired = velocity_squared(dx, dy);
                if desired > max_speed {
                    if desired < current_speed {
                        (*e).velocity = velocity;
                    }
                    return (SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as STATUS_FLAGS;
                }
                (*e).velocity = velocity;
            }
        }
        0
    }

    /// # Safety
    /// Element, owner, loaded race callbacks, and battle/UI state must be live.
    #[no_mangle]
    pub unsafe extern "C" fn ship_preprocess(e: *mut ELEMENT) {
        let ship = starship(e);
        let rd = (*ship).RaceDescPtr;
        let mut flags = input_flags(
            (*ship).cur_status_flags,
            (*ship).ship_input_state,
            (*e).state_flags & APPEARING as u16 != 0,
        );
        if (*e).state_flags & APPEARING as u16 != 0 {
            (*e).__bindgen_anon_2.crew_level = (*rd).ship_info.crew_level;
            let activity = GlobData.Game_state.CurrentActivity as BYTE;
            if (*e).playerNr == NPC_PLAYER_NUM as SIZE && activity == IN_LAST_BATTLE as BYTE {
                let old = SetContext(StatusContext);
                let mut stamp = STAMP {
                    origin: POINT { x: 0, y: 0 },
                    frame: (*rd).ship_data.captain_control.background,
                };
                DrawStamp(&mut stamp);
                DestroyDrawable(ReleaseDrawable(stamp.frame));
                (*rd).ship_data.captain_control.background = ptr::null_mut();
                SetContext(old);
            } else if activity <= IN_ENCOUNTER as BYTE {
                InitShipStatus(&raw mut (*rd).ship_info, ship, ptr::null_mut());
                let old = SetContext(StatusContext);
                DrawCaptainsWindow(ship);
                SetContext(old);
                if let Some(preprocess) = (*rd).preprocess_func {
                    preprocess(e);
                }
                if (*e).hTarget.is_null() {
                    ship_transition(e);
                } else {
                    (*e).hTarget = ptr::null_mut();
                    if PLRPlaying(usize::MAX as MUSIC_REF) == 0 && OpponentAlive(ship) != 0 {
                        BattleSong(1);
                    }
                }
                return;
            } else {
                (*e).current.location.x = (((ScreenWidth - STATUS_WIDTH as i32)
                    << (ONE_SHIFT + MAX_REDUCTION))
                    >> 1) as COORD;
                (*e).current.location.y =
                    ((ScreenHeight << (ONE_SHIFT + MAX_REDUCTION)) >> 1) as COORD;
                (*e).next.location = (*e).current.location;
                init_intersect_points(e);
                if hyper_transition(e) != 0 {
                    return;
                }
            }
        }
        (*ship).cur_status_flags = flags;
        if (*ship).energy_counter != 0 {
            (*ship).energy_counter -= 1;
        } else if (*rd).ship_info.energy_level < (*rd).ship_info.max_energy
            || ((*rd).characteristics.energy_regeneration as i8) < 0
        {
            DeltaEnergy(e, (*rd).characteristics.energy_regeneration as i8 as SIZE);
        }
        if let Some(preprocess) = (*rd).preprocess_func {
            preprocess(e);
            flags = (*ship).cur_status_flags;
        }
        if (*e).__bindgen_anon_4.turn_wait != 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else if flags & (LEFT | RIGHT) as u16 != 0 {
            (*ship).ShipFacing = normalize_facing(
                i32::from((*ship).ShipFacing) + if flags & LEFT as u16 != 0 { -1 } else { 1 },
            );
            (*e).next.image.frame = SetAbsFrameIndex((*e).next.image.frame, (*ship).ShipFacing);
            (*e).state_flags |= CHANGING as u16;
            (*e).__bindgen_anon_4.turn_wait = (*rd).characteristics.turn_wait;
        }
        if (*e).__bindgen_anon_5.thrust_wait != 0 {
            (*e).__bindgen_anon_5.thrust_wait -= 1;
        } else if flags & THRUST as u16 != 0 {
            let thrust = inertial_thrust(e);
            (*ship).cur_status_flags &=
                !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED | SHIP_IN_GRAVITY_WELL) as u16);
            (*ship).cur_status_flags |= thrust;
            (*e).__bindgen_anon_5.thrust_wait = (*rd).characteristics.thrust_wait;
            if !object_cloaked(e)
                && GlobData.Game_state.CurrentActivity as BYTE <= IN_ENCOUNTER as BYTE
            {
                spawn_ion_trail(e);
            }
        }
        if GlobData.Game_state.CurrentActivity as BYTE <= IN_ENCOUNTER as BYTE {
            PreProcessStatus(e);
        }
    }

    /// # Safety
    /// Element, ship weapon callbacks, element pool, and sound/UI state must be live.
    #[no_mangle]
    pub unsafe extern "C" fn ship_postprocess(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_2.crew_level == 0 {
            return;
        }
        let ship = starship(e);
        let rd = (*ship).RaceDescPtr;
        if (*ship).weapon_counter != 0 {
            (*ship).weapon_counter -= 1;
        } else if (*ship).cur_status_flags & WEAPON as u16 != 0
            && DeltaEnergy(e, -SIZE::from((*rd).characteristics.weapon_energy_cost)) != 0
        {
            let mut weapons = [ptr::null_mut(); 6];
            let count =
                ((*rd).init_weapon_func.expect("ship weapon initializer"))(e, weapons.as_mut_ptr());
            if count != 0 {
                let owner = starship(e);
                let mut played = false;
                for &weapon in &weapons[..count as usize] {
                    if !weapon.is_null() {
                        let w = element(weapon);
                        (*w).pParent = owner.cast();
                        if !played {
                            ProcessSound((*rd).ship_data.ship_sounds, w);
                            played = true;
                        }
                        put_element(weapon);
                    }
                }
                if !played {
                    ProcessSound((*rd).ship_data.ship_sounds, e);
                }
            }
            (*ship).weapon_counter = (*rd).characteristics.weapon_wait;
        }
        if (*ship).special_counter != 0 {
            (*ship).special_counter -= 1;
        }
        if let Some(postprocess) = (*rd).postprocess_func {
            postprocess(e);
        }
        if GlobData.Game_state.CurrentActivity as BYTE <= IN_ENCOUNTER as BYTE {
            PostProcessStatus(e);
        }
    }

    /// # Safety
    /// Ship, loader, battle queues, display primitives, and element pool must be live.
    pub unsafe fn spawn_ship(ship: *mut STARSHIP) -> BOOLEAN {
        let rd = load_ship((*ship).SpeciesID, 1);
        if rd.is_null() {
            return 0;
        }
        (*ship).RaceDescPtr = rd;
        (*ship).ship_input_state = 0;
        (*ship).cur_status_flags = 0;
        (*ship).old_status_flags = 0;
        (*rd).ship_info.crew_level = spawn_crew(
            GlobData.Game_state.CurrentActivity as BYTE,
            (*ship).crew_level,
            (*rd).ship_info.crew_level,
            (*rd).ship_info.max_crew,
        );
        (*ship).energy_counter = 0;
        (*ship).weapon_counter = 0;
        (*ship).special_counter = 0;
        let mut handle = (*ship).hShip;
        if handle.is_null() {
            handle = AllocElement();
            if !handle.is_null() {
                InsertQueue(&raw mut disp_q, handle, disp_q.head);
            }
        }
        (*ship).hShip = handle;
        if !handle.is_null() {
            let e = element(handle);
            (*e).playerNr = (*ship).playerNr;
            (*e).__bindgen_anon_2.crew_level = 0;
            (*e).__bindgen_anon_3.mass_points = (*rd).characteristics.ship_mass;
            (*e).state_flags = (APPEARING | PLAYER_SHIP | IGNORE_SIMILAR) as u16;
            (*e).__bindgen_anon_4.turn_wait = 0;
            (*e).__bindgen_anon_5.thrust_wait = 0;
            (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;
            (*e).colorCycleIndex = 0;
            (*(&raw mut DisplayArray)
                .cast::<PRIMITIVE>()
                .add((*e).PrimIndex as usize))
            .Type = gfx_object_STAMP_PRIM as u8;
            (*e).current.image.farray = (&raw mut (*rd).ship_data.ship).cast();
            let width = (ScreenWidth - STATUS_WIDTH as i32) << (ONE_SHIFT + MAX_REDUCTION);
            let height = ScreenHeight << (ONE_SHIFT + MAX_REDUCTION);
            if (*e).playerNr == NPC_PLAYER_NUM as SIZE
                && GlobData.Game_state.CurrentActivity as BYTE == IN_LAST_BATTLE as BYTE
            {
                (*ship).ShipFacing = 0;
                (*e).current.image.frame =
                    SetAbsFrameIndex((*rd).ship_data.ship[0], (*ship).ShipFacing);
                (*e).current.location = POINT {
                    x: (width >> 1) as COORD,
                    y: (height >> 1) as COORD,
                };
                (*e).__bindgen_anon_1.life_span += 1;
            } else {
                (*ship).ShipFacing = normalize_facing(TFB_Random() as i32);
                if inHQSpace() != 0 {
                    let facing = GlobData.Game_state.ShipFacing;
                    (*ship).ShipFacing = if facing > 0 { facing - 1 } else { facing };
                }
                (*e).current.image.frame =
                    SetAbsFrameIndex((*rd).ship_data.ship[0], (*ship).ShipFacing);
                loop {
                    (*e).current.location.x = spawn_coordinate(TFB_Random(), width);
                    (*e).current.location.y = spawn_coordinate(TFB_Random(), height);
                    if CalculateGravity(e) == 0 && TimeSpaceMatterConflict(e) == 0 {
                        break;
                    }
                }
            }
            (*e).preprocess_func = Some(ship_preprocess);
            (*e).postprocess_func = Some(ship_postprocess);
            (*e).death_func = Some(ship_death);
            (*e).collision_func = Some(collision);
            (*e).velocity = mem::zeroed();
            (*e).pParent = ship.cast();
            (*e).hTarget = ptr::null_mut();
        }
        (!handle.is_null()) as BOOLEAN
    }

    /// # Safety
    /// Encounter selection state, battle queues and optional last ship must be live.
    #[no_mangle]
    pub unsafe extern "C" fn GetNextStarShip(mut last: *mut STARSHIP, side: COUNT) -> BOOLEAN {
        let handle = GetEncounterStarShip(last, side);
        if !handle.is_null() {
            assert_eq!(
                (*(&raw const race_q).cast::<QUEUE>().add(side as usize)).object_size as usize,
                mem::size_of::<STARSHIP>()
            );
            let ship = handle.cast::<STARSHIP>();
            if !last.is_null() {
                if ship == last {
                    last = ptr::null_mut();
                } else {
                    (*ship).hShip = (*last).hShip;
                }
            }
            if spawn_ship(ship) == 0 {
                return 0;
            }
        }
        if !last.is_null() {
            (*last).hShip = ptr::null_mut();
        }
        (!handle.is_null()) as BOOLEAN
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
                assert_eq!(
                    (*(&raw const race_q).cast::<QUEUE>().add(player as usize)).object_size
                        as usize,
                    mem::size_of::<STARSHIP>()
                );
                if spawn_ship(ships[player as usize].cast()) == 0 {
                    return 0;
                }
            }
        } else {
            for player in (0..NUM_PLAYERS as COUNT).rev() {
                if GetNextStarShip(ptr::null_mut(), player) == 0 {
                    return 0;
                }
            }
        }
        1
    }

    /// # Safety
    /// Both elements and collision point pointers must be live.
    #[no_mangle]
    pub unsafe extern "C" fn collision(
        e0: *mut ELEMENT,
        _p0: *mut POINT,
        e1: *mut ELEMENT,
        _p1: *mut POINT,
    ) {
        if (*e1).state_flags & FINITE_LIFE as u16 == 0 {
            (*e0).state_flags |= COLLISION as u16;
            if u16::from((*e1).__bindgen_anon_3.mass_points) > GRAVITY_MASS_NATIVE {
                let damage = ((*e0).__bindgen_anon_2.hit_points >> 2).max(1);
                do_damage(e0, damage as i16);
                let sound = (TARGET_DAMAGED_FOR_1_PT_NATIVE + (damage >> 1) as i16)
                    .min(TARGET_DAMAGED_FOR_6_PLUS_PT_NATIVE);
                ProcessSound(SetAbsStringTableIndex(GameSounds, sound as u16), e0);
            }
        }
    }
}

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub use engine::{
    animation_preprocess, collision, inertial_thrust, ship_postprocess, ship_preprocess,
    spawn_ship, GetInitialStarShips, GetNextStarShip,
};

#[cfg(test)]
mod tests {
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
