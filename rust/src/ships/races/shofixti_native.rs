//! Shofixti live callbacks translated from `sc2/src/uqm/ships/shofixti/shofixti.c`.
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::collections::observation::image;
    use crate::ships::native::{abi::*, element, frame_index, put_element, starship};
    use std::{mem, ptr};

    const MAX_CREW: u16 = 6;
    const MAX_ENERGY: u8 = 4;
    const MAX_THRUST: u16 = 35;
    const THRUST_INCREMENT: u16 = 5;
    const DESTRUCT_RANGE: i32 = 180;
    const MAX_DESTRUCTION: i32 = DESTRUCT_RANGE / 10;
    // element.h: NUM_EXPLOSION_FRAMES is twelve, including for the ship flash.
    const DESTRUCT_SWITCH: u16 = 12 * 3 - 3;
    const PLAY_VICTORY_DITTY: u16 = 1 << 9;

    use crate::battle::sounds::engine::{CalcSoundPosition, PlaySound};

    use crate::navigation::globals::getGameState;

    use crate::battle::tactrans_native::engine::StopAllBattleMusic;

    use crate::battle::tactrans_native::engine::StartShipExplosion;

    use crate::battle::tactrans_native::engine::GetWinnerStarShip;

    use crate::battle::tactrans_native::engine::FindAliveStarShip;

    use crate::battle::tactrans_native::engine::SetWinnerStarShip;

    use crate::battle::tactrans_native::engine::RecordShipDeath;

    use crate::ships::races::orz_native::engine::intruder_preprocess;

    use crate::ships::races::orz_native::engine::marine_collision;

    // BUILD_COLOR ignores the palette index; MAKE_RGB15 expands each channel.
    const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    fn same_color(a: Color, b: Color) -> bool {
        a.r == b.r && a.g == b.g && a.b == b.b && a.a == b.a
    }

    unsafe fn prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
        GlobData
            .Game_state
            .DisplayArray
            .add((*e).PrimIndex as usize)
    }

    const fn descriptor() -> RACE_DESC {
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = FIRES_FORE as u16;
        d.ship_info.ship_cost = 5;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.shofixti.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.shofixti.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.shofixti.meleeicons".as_ptr();
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: 1,
            weapon_energy_cost: 1,
            special_energy_cost: 0,
            energy_wait: 9,
            turn_wait: 1,
            thrust_wait: 0,
            weapon_wait: 3,
            special_wait: 0,
            ship_mass: 1,
        };
        d.ship_data.ship_rsc = [
            c"ship.shofixti.graphics.scout.large".as_ptr(),
            c"ship.shofixti.graphics.scout.medium".as_ptr(),
            c"ship.shofixti.graphics.scout.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.shofixti.graphics.missile.large".as_ptr(),
            c"ship.shofixti.graphics.missile.medium".as_ptr(),
            c"ship.shofixti.graphics.missile.small".as_ptr(),
        ];
        d.ship_data.special_rsc = [
            c"ship.shofixti.graphics.destruct.large".as_ptr(),
            c"ship.shofixti.graphics.destruct.medium".as_ptr(),
            c"ship.shofixti.graphics.destruct.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.shofixti.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.shofixti.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.shofixti.sounds".as_ptr();
        d.cyborg_control.WeaponRange = ((24 << ONE_SHIFT) * 10) as COUNT;
        d
    }
    static mut SHOFIXTI_DESC: RACE_DESC = descriptor();
    static mut NEW_SHOFIXTI_DESC: RACE_DESC = descriptor();

    #[no_mangle]
    pub extern "C" fn init_shofixti() -> *mut RACE_DESC {
        unsafe {
            SHOFIXTI_DESC.postprocess_func = Some(shofixti_postprocess);
            SHOFIXTI_DESC.init_weapon_func = Some(initialize_standard_missile);
            SHOFIXTI_DESC.cyborg_control.intelligence_func = Some(shofixti_intelligence);
            let mut d = SHOFIXTI_DESC;
            if GlobData.Game_state.CurrentActivity as u8 == IN_ENCOUNTER as u8
                && getGameState(SHOFIXTI_RECRUITED as i32, END_SHOFIXTI_RECRUITED as i32) == 0
            {
                d.ship_data.ship_rsc = [
                    c"ship.shofixti.graphics.oldscout.large".as_ptr(),
                    c"ship.shofixti.graphics.oldscout.medium".as_ptr(),
                    c"ship.shofixti.graphics.oldscout.small".as_ptr(),
                ];
                d.ship_data.special_rsc = [ptr::null(); 3];
                d.ship_data.captain_control.captain_rsc =
                    c"ship.shofixti.graphics.oldcaptain".as_ptr();
                d.characteristics.weapon_wait = 10;
                for _ in 0..3 {
                    let c = &mut d.characteristics;
                    c.turn_wait = c.turn_wait.wrapping_add(1);
                    if c.turn_wait == 0 {
                        c.turn_wait = c.turn_wait.wrapping_sub(1);
                    }
                    c.thrust_wait = c.thrust_wait.wrapping_add(1);
                    if c.thrust_wait == 0 {
                        c.thrust_wait = c.thrust_wait.wrapping_sub(1);
                    }
                    if c.thrust_increment <= 1 << ONE_SHIFT {
                        c.max_thrust = c.thrust_increment << 1;
                    } else {
                        let num_thrusts = c.max_thrust / c.thrust_increment;
                        c.thrust_increment -= 1;
                        c.max_thrust = c.thrust_increment * num_thrusts;
                    }
                }
            }
            NEW_SHOFIXTI_DESC = d;
        }
        &raw mut NEW_SHOFIXTI_DESC
    }

    pub(crate) unsafe extern "C" fn initialize_standard_missile(
        ship: *mut ELEMENT,
        out: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: (*ss).ShipFacing,
            index: (*ss).ShipFacing,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 15,
            speed: (24 << ONE_SHIFT) as SIZE,
            hit_points: 1,
            damage: 1,
            life: 10,
            preprocess_func: None,
            blast_offs: 1,
        };
        *out = initialize_missile(&mut block);
        1
    }

    pub(crate) unsafe extern "C" fn destruct_preprocess(e: *mut ELEMENT) {
        let primitive = prim(e);
        (*e).state_flags |= CHANGING as u16;
        let life = (*e).__bindgen_anon_1.life_span;
        if life > DESTRUCT_SWITCH {
            (*primitive).Type = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
            (*primitive).color = if life == DESTRUCT_SWITCH + 2 {
                rgb15(31, 31, 10)
            } else {
                rgb15(31, 31, 31)
            };
        } else if life < DESTRUCT_SWITCH {
            let color = (*primitive).color;
            image!(frame; (*e).next.image.frame, IncFrameIndex((*e).current.image.frame));
            for (from, to) in [
                (rgb15(31, 31, 31), rgb15(31, 31, 10)),
                (rgb15(31, 31, 10), rgb15(31, 10, 10)),
                (rgb15(31, 10, 10), rgb15(20, 10, 0)),
                (rgb15(20, 10, 0), rgb15(20, 0, 0)),
            ] {
                if same_color(color, from) {
                    (*primitive).color = to;
                    break;
                }
            }
        } else {
            (*primitive).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
            (*e).preprocess_func = None;
            let h = AllocElement();
            if !h.is_null() {
                let ss = starship(e);
                put_element(h);
                let d = element(h);
                (*d).pParent = ss.cast();
                (*d).__bindgen_anon_2.hit_points = 0;
                (*d).__bindgen_anon_3.mass_points = 0;
                (*d).playerNr = NEUTRAL_PLAYER_NUM as SIZE;
                (*d).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
                (*prim(d)).Type = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
                (*prim(d)).color = rgb15(31, 31, 31);
                let frame = (*(*ss).RaceDescPtr).ship_data.special[0];
                image!(set; (*d).current.image.farray, (&raw mut (*(*ss).RaceDescPtr).ship_data.special).cast());
                image!(frame; (*d).current.image.frame, frame);
                (*d).__bindgen_anon_1.life_span = GetFrameCount(frame.cast());
                (*d).current.location = (*e).current.location;
                (*d).preprocess_func = Some(ElementHook::Legacy(destruct_preprocess));
                (*d).postprocess_func = None;
                (*d).death_func = None;
                (*d).velocity = mem::zeroed();
            }
        }
    }

    pub(crate) unsafe extern "C" fn self_destruct_kill_objects(e: *mut ELEMENT) {
        let mut h = disp_q.head;
        while !h.is_null() {
            let obj = element(h);
            let next = (*obj).succ;
            let flags = (*obj).state_flags;
            let marine = (*obj).preprocess_func.map(|f| f.address())
                == Some(intruder_preprocess as *const () as usize)
                && (*obj).collision_func.map(|f| f.address())
                    == Some(marine_collision as *const () as usize);
            if flags & (NONSOLID | DISAPPEARING) as u16 == 0 || marine {
                // C stores each subtraction and negation in SIZE before shifting.
                let mut dx =
                    (i32::from((*obj).next.location.x) - i32::from((*e).next.location.x)) as SIZE;
                let mut dy =
                    (i32::from((*obj).next.location.y) - i32::from((*e).next.location.y)) as SIZE;
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                let dist = (i32::from(dx) * i32::from(dx) + i32::from(dy) * i32::from(dy)) as DWORD;
                if i32::from(dx) <= DESTRUCT_RANGE
                    && i32::from(dy) <= DESTRUCT_RANGE
                    && dist <= (DESTRUCT_RANGE * DESTRUCT_RANGE) as DWORD
                {
                    let destruction = 1 + MAX_DESTRUCTION
                        * (DESTRUCT_RANGE - i32::from(square_root(dist)))
                        / DESTRUCT_RANGE;
                    if flags & PLAYER_SHIP as u16 != 0 {
                        if DeltaCrew(obj, -destruction as SIZE) == 0 {
                            (*obj).__bindgen_anon_1.life_span = 0;
                        }
                    } else if u32::from((*obj).__bindgen_anon_3.mass_points) <= MAX_SHIP_MASS * 10 {
                        let hp = (*obj).__bindgen_anon_2.hit_points;
                        if destruction < i32::from(hp) {
                            (*obj).__bindgen_anon_2.hit_points =
                                hp.wrapping_sub(destruction as COUNT);
                        } else {
                            (*obj).__bindgen_anon_2.hit_points = 0;
                            (*obj).__bindgen_anon_1.life_span = 0;
                        }
                    }
                }
            }
            h = next;
        }
    }

    pub(crate) unsafe extern "C" fn self_destruct(e: *mut ELEMENT) {
        let h = AllocElement();
        if !h.is_null() {
            let d = element(h);
            (*d).playerNr = (*e).playerNr;
            (*d).state_flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
            (*d).next.location = (*e).next.location;
            (*d).__bindgen_anon_1.life_span = 0;
            (*d).pParent = (*e).pParent;
            (*d).hTarget = ptr::null_mut();
            (*d).death_func = Some(ElementHook::Legacy(self_destruct_kill_objects));
            put_element(h);
        }
        DeltaCrew(e, -(i32::from((*e).__bindgen_anon_2.crew_level)) as SIZE);
        (*e).state_flags |= NONSOLID as u16;
        (*e).__bindgen_anon_1.life_span = 0;
        (*e).death_func = Some(ElementHook::Legacy(shofixti_destruct_death));
    }

    pub(crate) unsafe extern "C" fn shofixti_destruct_death(e: *mut ELEMENT) {
        let ss = starship(e);
        StopAllBattleMusic();
        StartShipExplosion(e, false);
        (*e).preprocess_func = Some(ElementHook::Legacy(destruct_preprocess));
        PlaySound(
            SetAbsStringTableIndex((*(*ss).RaceDescPtr).ship_data.ship_sounds, 1).cast(),
            CalcSoundPosition(e),
            e.cast(),
            3,
        );
        let mut winner = GetWinnerStarShip();
        if winner.is_null() {
            winner = FindAliveStarShip(e);
            if winner.is_null() {
                winner = ss;
            }
            SetWinnerStarShip(winner);
        } else if winner == ss {
            (*ss).cur_status_flags &= !PLAY_VICTORY_DITTY;
        }
        RecordShipDeath(e);
    }

    pub(crate) unsafe extern "C" fn shofixti_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        ship_intelligence(ship, concerns, count);
        let ss = starship(ship);
        if (*ss).special_counter != 0 {
            return;
        }
        if (*ss).ship_input_state & SPECIAL as u8 != 0 {
            (*ss).ship_input_state &= !(SPECIAL as u8);
        } else {
            let weapon = concerns.add(ENEMY_WEAPON_INDEX as usize);
            let enemy = concerns.add(ENEMY_SHIP_INDEX as usize);
            let captain = (*(*ss).RaceDescPtr).ship_data.captain_control.special;
            if !(*(*ss).RaceDescPtr).ship_data.special[0].is_null()
                && (i32::from(GetFrameCount(captain)) - i32::from(frame_index(captain)) > 5
                    || (!(*enemy).ObjectPtr.is_null() && (*enemy).which_turn <= 4)
                    || (!(*weapon).ObjectPtr.is_null()
                        && (((*(*weapon).ObjectPtr).state_flags & PLAYER_SHIP as u16 != 0
                            && (*ship).__bindgen_anon_2.crew_level == 1)
                            || (PlotIntercept((*weapon).ObjectPtr, ship, 2, 0) != 0
                                && u16::from(
                                    (*(*weapon).ObjectPtr).__bindgen_anon_3.mass_points,
                                ) >= (*ship).__bindgen_anon_2.crew_level
                                && TFB_Random() & 1 != 0))))
            {
                (*ss).ship_input_state |= SPECIAL as u8;
            }
        }
    }

    pub(crate) unsafe extern "C" fn shofixti_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        if ((*ss).cur_status_flags ^ (*ss).old_status_flags) & SPECIAL as u16 != 0 {
            let captain = &raw mut (*(*ss).RaceDescPtr).ship_data.captain_control.special;
            *captain = IncFrameIndex(*captain);
            if i32::from(GetFrameCount((*captain).cast())) - i32::from(frame_index(*captain)) == 3 {
                self_destruct(e);
            }
        }
    }
}
