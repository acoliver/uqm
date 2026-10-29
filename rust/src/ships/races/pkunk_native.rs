//! Pkunk callbacks and descriptor translated from `sc2/src/uqm/ships/pkunk/pkunk.c`.
//!
//! The C file leans on function-like macros from `element.h`, `units.h`,
//! `gfxlib.h`, `prim.h`, `collide.h` and `velocity.h`. They are expanded here
//! with the same integer promotions, truncations and (importantly) the same
//! number of evaluations of their arguments, because `TFB_Random` is one of
//! those arguments.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::battle::gravity::{CalculateGravity, TimeSpaceMatterConflict};
    use crate::collections::observation::image;
    use crate::memory::{rust_hfree, rust_hmalloc};
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, element, facing_to_angle, normalize_facing, put_element,
        sine, starship,
    };
    use crate::strings::string_table::GetStringTableCount;
    use std::{mem, ptr, sync::atomic::Ordering};

    // Core characteristics
    const MAX_CREW: COUNT = 8;
    const MAX_ENERGY: BYTE = 12;
    const ENERGY_REGENERATION: BYTE = 0;
    const ENERGY_WAIT: BYTE = 0;
    const MAX_THRUST: COUNT = 64;
    const THRUST_INCREMENT: COUNT = 16;
    const THRUST_WAIT: BYTE = 0;
    const TURN_WAIT: BYTE = 0;
    const SHIP_MASS: BYTE = 1;

    // Triple Miniguns
    const WEAPON_ENERGY_COST: BYTE = 1;
    const WEAPON_WAIT: BYTE = 0;
    const PKUNK_OFFSET: SIZE = 15;
    const MISSILE_OFFSET: SIZE = 1;
    const MISSILE_SPEED: SIZE = 24 << ONE_SHIFT;
    const MISSILE_LIFE: COUNT = 5;
    const MISSILE_HITS: SIZE = 1;
    const MISSILE_DAMAGE: SIZE = 1;

    // Taunt
    const SPECIAL_ENERGY_COST: BYTE = 2;
    const SPECIAL_WAIT: BYTE = 16;

    // Respawn
    const PHOENIX_LIFE: COUNT = 12;
    const TRANSITION_LIFE: COUNT = 1;
    const TRANSITION_SPEED: i32 = 20 << ONE_SHIFT;
    const START_PHOENIX_COLOR: Color = rgb15(0x1F, 0x15, 0x00);

    // `intel.h`: DISPLAY_TO_WORLD (50), and the control rating this ship tests.
    const CLOSE_RANGE_WEAPON: i32 = 50 << ONE_SHIFT;
    const STANDARD_RATING: BYTE = 1 << 4;
    // `races.h`
    const PLAY_VICTORY_DITTY: u16 = 1 << 9;
    // `units.h`
    const STATUS_WIDTH: i32 = 64;
    const MAX_REDUCTION: u32 = 3;

    /// `CC5TO8`, promoted to `int` and truncated to `BYTE` on assignment.
    const fn cc5to8(c: u32) -> BYTE {
        ((c << 3) | (c >> 2)) as BYTE
    }

    /// `MAKE_RGB15` / `MAKE_RGB15_INIT`. `BUILD_COLOR (col, c256)` is `col`:
    /// the palette index is ignored, so the table below drops it.
    const fn rgb15(r: u32, g: u32, b: u32) -> Color {
        Color {
            r: cc5to8(r),
            g: cc5to8(g),
            b: cc5to8(b),
            a: 0xff,
        }
    }

    const PHOENIX_TRAIL_COLORS: [Color; 12] = [
        rgb15(0x1F, 0x15, 0x00),
        rgb15(0x1F, 0x11, 0x00),
        rgb15(0x1F, 0x0E, 0x00),
        rgb15(0x1F, 0x0A, 0x00),
        rgb15(0x1F, 0x07, 0x00),
        rgb15(0x1F, 0x03, 0x00),
        rgb15(0x1F, 0x00, 0x00),
        rgb15(0x1B, 0x00, 0x00),
        rgb15(0x17, 0x00, 0x00),
        rgb15(0x13, 0x00, 0x00),
        rgb15(0x0F, 0x00, 0x00),
        rgb15(0x0B, 0x00, 0x00),
    ];

    // Engine entry points the shared race ABI does not export. The generated
    // ABI types are used so these resolve to the same C symbols.

    use crate::collections::queue::InsertQueue;

    use crate::battle::tactrans_native::engine::StopAllBattleMusic;

    use crate::battle::tactrans_native::engine::StartShipExplosion;

    // Private per-instance ship data
    #[repr(C)]
    #[derive(Clone, Copy)]
    #[allow(non_snake_case, reason = "field names follow PKUNK_DATA in pkunk.c")]
    pub(crate) struct PkunkData {
        pub(crate) hPhoenix: HELEMENT,
        pub(crate) saved_preprocess_func: ElementProcessFunc,
        pub(crate) saved_postprocess_func: ElementProcessFunc,
        pub(crate) saved_death_func: ElementProcessFunc,
    }

    impl PkunkData {
        /// The `memset (&empty_data, 0, ...)` block `init_pkunk` installs.
        const EMPTY: Self = Self {
            hPhoenix: ptr::null_mut(),
            saved_preprocess_func: None,
            saved_postprocess_func: None,
            saved_death_func: None,
        };
    }

    const fn descriptor() -> RACE_DESC {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut d = RACE_DESC::EMPTY;
        d.ship_info.ship_flags = (FIRES_FORE | FIRES_LEFT | FIRES_RIGHT) as UWORD;
        d.ship_info.ship_cost = 20;
        d.ship_info.crew_level = MAX_CREW;
        d.ship_info.max_crew = MAX_CREW;
        d.ship_info.energy_level = MAX_ENERGY;
        d.ship_info.max_energy = MAX_ENERGY;
        d.ship_info.race_strings_rsc = c"ship.pkunk.text".as_ptr();
        d.ship_info.icons_rsc = c"ship.pkunk.icons".as_ptr();
        d.ship_info.melee_icon_rsc = c"ship.pkunk.meleeicons".as_ptr();
        d.fleet.strength = (666 / SPHERE_RADIUS_INCREMENT * 2) as COUNT;
        d.fleet.known_loc = POINT { x: 502, y: 401 };
        d.characteristics = CHARACTERISTIC_STUFF {
            max_thrust: MAX_THRUST,
            thrust_increment: THRUST_INCREMENT,
            energy_regeneration: ENERGY_REGENERATION,
            weapon_energy_cost: WEAPON_ENERGY_COST,
            special_energy_cost: SPECIAL_ENERGY_COST,
            energy_wait: ENERGY_WAIT,
            turn_wait: TURN_WAIT,
            thrust_wait: THRUST_WAIT,
            weapon_wait: WEAPON_WAIT,
            special_wait: 0,
            ship_mass: SHIP_MASS,
        };
        // PKUNK_*_MASK_PMAP_ANIM name the "fury" graphics, BUG_* the missiles.
        d.ship_data.ship_rsc = [
            c"ship.pkunk.graphics.fury.large".as_ptr(),
            c"ship.pkunk.graphics.fury.medium".as_ptr(),
            c"ship.pkunk.graphics.fury.small".as_ptr(),
        ];
        d.ship_data.weapon_rsc = [
            c"ship.pkunk.graphics.bug.large".as_ptr(),
            c"ship.pkunk.graphics.bug.medium".as_ptr(),
            c"ship.pkunk.graphics.bug.small".as_ptr(),
        ];
        d.ship_data.captain_control.captain_rsc = c"ship.pkunk.graphics.captain".as_ptr();
        d.ship_data.victory_ditty_rsc = c"ship.pkunk.ditty".as_ptr();
        d.ship_data.ship_sounds_rsc = c"ship.pkunk.sounds".as_ptr();
        d.cyborg_control.WeaponRange = (CLOSE_RANGE_WEAPON + 1) as COUNT;
        d
    }

    /// `pkunk_desc`: the template whose callbacks `init_pkunk` fills in.
    static mut PKUNK_DESC: RACE_DESC = descriptor();
    /// `new_pkunk_desc`: the copy `init_pkunk` hands out, which owns the data.
    static mut NEW_PKUNK_DESC: RACE_DESC = RACE_DESC::EMPTY;
    use crate::engine::battle::kernel::LAST_PKUNK_SOUND as LAST_SOUND;

    #[no_mangle]
    pub extern "C" fn init_pkunk() -> *mut RACE_DESC {
        unsafe {
            PKUNK_DESC.uninit_func = Some(uninit_pkunk);
            PKUNK_DESC.preprocess_func = Some(pkunk_preprocess);
            PKUNK_DESC.postprocess_func = Some(pkunk_postprocess);
            PKUNK_DESC.init_weapon_func = Some(initialize_bug_missile);
            PKUNK_DESC.cyborg_control.intelligence_func = Some(pkunk_intelligence);

            // copy initial ship settings to the new descriptor
            NEW_PKUNK_DESC = PKUNK_DESC;
            let empty_data = PkunkData::EMPTY;
            set_custom_ship_data(&raw mut NEW_PKUNK_DESC, &empty_data);

            // Reinitialised at least each battle, so that NetPlay stays
            // synchronised after another game was played first.
            LAST_SOUND.store(0, Ordering::Relaxed);

            &raw mut NEW_PKUNK_DESC
        }
    }

    /// `GetCustomShipData`
    unsafe fn get_custom_ship_data(desc: *mut RACE_DESC) -> *mut PkunkData {
        (*desc).data.cast()
    }

    /// `SetCustomShipData`: (re)allocates the descriptor's own storage.
    unsafe fn set_custom_ship_data(desc: *mut RACE_DESC, data: *const PkunkData) {
        if (*desc).data.cast_const() == data.cast() {
            return; // no-op
        }

        if !(*desc).data.is_null() {
            // Out with the old
            rust_hfree((*desc).data);
            (*desc).data = ptr::null_mut();
        }

        if !data.is_null() {
            // In with the new
            let new_data = rust_hmalloc(mem::size_of::<PkunkData>()).cast::<PkunkData>();
            new_data.write(*data);
            (*desc).data = new_data.cast();
        }
    }

    /// `SetPrimType (&(GLOBAL (DisplayArray))[PrimIndex], type)`
    unsafe fn display_prim(e: *mut ELEMENT) -> *mut PRIMITIVE {
        GlobData
            .Game_state
            .DisplayArray
            .add(usize::from((*e).PrimIndex))
    }

    /// `LOG_SPACE_WIDTH`: `DISPLAY_TO_WORLD (SPACE_WIDTH) << MAX_REDUCTION`.
    unsafe fn log_space_width() -> i32 {
        ((ScreenWidth - STATUS_WIDTH) << ONE_SHIFT) << MAX_REDUCTION
    }

    /// `LOG_SPACE_HEIGHT`
    unsafe fn log_space_height() -> i32 {
        (ScreenHeight << ONE_SHIFT) << MAX_REDUCTION
    }

    /// `DISPLAY_ALIGN_X/Y` once the argument is known: reduce the value as a
    /// COUNT, narrow to COORD, then snap to a display pixel.
    fn display_align(random: u32, log_extent: i32) -> i32 {
        let reduced = i32::from(random as COUNT) % log_extent;
        i32::from(reduced as COORD) & !((1 << ONE_SHIFT) - 1)
    }

    /// `WRAP_VAL` for an argument without side effects; the result is a COUNT.
    fn wrap_val(v: i32, w: i32) -> COUNT {
        (if v < 0 {
            v + w
        } else if v >= w {
            v - w
        } else {
            v
        }) as COUNT
    }

    /// `WRAP_X (DISPLAY_ALIGN_X (TFB_Random ()))` and its Y twin. `WRAP_VAL`
    /// writes its value operand up to three times, so every textual use is a
    /// fresh draw: two for a negative value, three otherwise.
    unsafe fn random_space_coord(log_extent: i32) -> COORD {
        let draw = || display_align(TFB_Random(), log_extent);
        let wrapped = if draw() < 0 {
            draw() + log_extent
        } else if draw() >= log_extent {
            draw() - log_extent
        } else {
            draw()
        };
        wrapped as COUNT as COORD
    }

    /// `InitIntersectStartPoint`; `WORLD_TO_DISPLAY` is `>> ONE_SHIFT`.
    unsafe fn init_intersect_start_point(e: *mut ELEMENT) {
        let origin = &raw mut (*e).IntersectControl.IntersectStamp.origin;
        (*origin).x = (i32::from((*e).current.location.x) >> ONE_SHIFT) as COORD;
        (*origin).y = (i32::from((*e).current.location.y) >> ONE_SHIFT) as COORD;
    }

    /// `InitIntersectEndPoint`
    unsafe fn init_intersect_end_point(e: *mut ELEMENT) {
        let end_point = &raw mut (*e).IntersectControl.EndPoint;
        (*end_point).x = (i32::from((*e).next.location.x) >> ONE_SHIFT) as COORD;
        (*end_point).y = (i32::from((*e).next.location.y) >> ONE_SHIFT) as COORD;
    }

    /// `InitIntersectFrame`
    unsafe fn init_intersect_frame(e: *mut ELEMENT) {
        image!(frame; (*e).IntersectControl.IntersectStamp.frame, SetEquFrameIndex(*(*e).next.image.farray, (*e).next.image.frame));
    }

    /// `ZeroVelocityComponents`: a memset of the whole VELOCITY_DESC.
    unsafe fn zero_velocity_components(velocity: *mut VELOCITY_DESC) {
        ptr::write_bytes(velocity, 0, 1);
    }

    /// `GRAVITY_MASS`
    fn gravity_mass(mass: i32) -> bool {
        mass > MAX_SHIP_MASS as i32 * 10
    }

    pub(crate) unsafe extern "C" fn animate(e: *mut ELEMENT) {
        if (*e).__bindgen_anon_4.turn_wait > 0 {
            (*e).__bindgen_anon_4.turn_wait -= 1;
        } else {
            image!(frame; (*e).next.image.frame, IncFrameIndex((*e).current.image.frame));
            (*e).state_flags |= CHANGING as ELEMENT_FLAGS;

            (*e).__bindgen_anon_4.turn_wait = (*e).__bindgen_anon_5.next_turn;
        }
    }

    pub(crate) unsafe extern "C" fn initialize_bug_missile(
        ship: *mut ELEMENT,
        missiles: *mut HELEMENT,
    ) -> COUNT {
        let ss = starship(ship);
        let mut block = MISSILE_BLOCK {
            cx: (*ship).next.location.x,
            cy: (*ship).next.location.y,
            farray: (&raw mut (*(*ss).RaceDescPtr).ship_data.weapon).cast(),
            face: 0,
            index: 0,
            sender: (*ship).playerNr,
            flags: IGNORE_SIMILAR as ELEMENT_FLAGS,
            pixoffs: PKUNK_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess_func: None,
            blast_offs: MISSILE_OFFSET,
        };

        for i in 0..3usize {
            let mut face =
                i32::from((*ss).ShipFacing) + angle_to_facing(QUADRANT as i32) * i as i32;
            if i == 2 {
                face += angle_to_facing(QUADRANT as i32);
            }
            block.face = normalize_facing(face);

            let missile = initialize_missile(&mut block);
            *missiles.add(i) = missile;
            if !missile.is_null() {
                let missile = element(missile);
                let mut dx: SIZE = 0;
                let mut dy: SIZE = 0;
                GetCurrentVelocityComponents(&raw mut (*ship).velocity, &mut dx, &mut dy);
                DeltaVelocityComponents(&raw mut (*missile).velocity, dx, dy);
                (*missile).current.location.x = (i32::from((*missile).current.location.x)
                    - (i32::from(dx) >> VELOCITY_SHIFT))
                    as COORD;
                (*missile).current.location.y = (i32::from((*missile).current.location.y)
                    - (i32::from(dy) >> VELOCITY_SHIFT))
                    as COORD;

                (*missile).preprocess_func = Some(ElementHook::Legacy(animate));
            }
        }

        3
    }

    pub(crate) unsafe extern "C" fn pkunk_intelligence(
        ship: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: COUNT,
    ) {
        let ss = starship(ship);
        let data = get_custom_ship_data((*ss).RaceDescPtr);
        if !(*data).hPhoenix.is_null() && (*ss).control & STANDARD_RATING != 0 {
            RemoveElement((*data).hPhoenix);
            FreeElement((*data).hPhoenix);
            (*data).hPhoenix = ptr::null_mut();
        }

        let ship_info = &raw const (*(*ss).RaceDescPtr).ship_info;
        if (*ship_info).energy_level < (*ship_info).max_energy
            && ((*ss).special_counter == 0 || (TFB_Random() as BYTE) < 20)
        {
            (*ss).ship_input_state |= SPECIAL as BYTE;
        } else {
            (*ss).ship_input_state &= !(SPECIAL as BYTE);
        }
        ship_intelligence(ship, concerns, count);
    }

    pub(crate) unsafe extern "C" fn new_pkunk(e: *mut ELEMENT) {
        let ss = starship(e);
        let desc = (*ss).RaceDescPtr;
        let data = get_custom_ship_data(desc);

        (*e).state_flags = (APPEARING | PLAYER_SHIP | IGNORE_SIMILAR) as ELEMENT_FLAGS;
        (*e).__bindgen_anon_3.mass_points = SHIP_MASS;
        // Restore the element processing callbacks after the explosion.
        // The callbacks were changed for the explosion sequence
        (*e).preprocess_func = (*data).saved_preprocess_func;
        (*e).postprocess_func = (*data).saved_postprocess_func;
        (*e).death_func = (*data).saved_death_func;
        // preprocess_func() is called during the phoenix transition and
        // then cleared, so we need to restore it
        (*desc).preprocess_func = Some(pkunk_preprocess);
        (*desc).ship_info.crew_level = MAX_CREW;
        (*desc).ship_info.energy_level = MAX_ENERGY;
        // fix vux impairment
        (*desc).characteristics.max_thrust = MAX_THRUST;
        (*desc).characteristics.thrust_increment = THRUST_INCREMENT;
        (*desc).characteristics.turn_wait = TURN_WAIT;
        (*desc).characteristics.thrust_wait = THRUST_WAIT;
        (*desc).characteristics.special_wait = 0;

        (*ss).ship_input_state = 0;
        // Pkunk wins in a simultaneous destruction if it reincarnates
        (*ss).cur_status_flags &= PLAY_VICTORY_DITTY;
        (*ss).old_status_flags = 0;
        (*ss).energy_counter = 0;
        (*ss).weapon_counter = 0;
        (*ss).special_counter = 0;
        (*e).__bindgen_anon_2.crew_level = 0;
        (*e).__bindgen_anon_4.turn_wait = 0;
        (*e).__bindgen_anon_5.thrust_wait = 0;
        (*e).__bindgen_anon_1.life_span = NORMAL_LIFE as COUNT;

        (*ss).ShipFacing = normalize_facing(TFB_Random() as i32);
        image!(set; (*e).current.image.farray, (&raw mut (*desc).ship_data.ship).cast());
        image!(frame; (*e).current.image.frame, SetAbsFrameIndex((*desc).ship_data.ship[0], (*ss).ShipFacing));
        (*display_prim(e)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;

        loop {
            (*e).current.location.x = random_space_coord(log_space_width());
            (*e).current.location.y = random_space_coord(log_space_height());
            if CalculateGravity(e.cast()) == 0 && TimeSpaceMatterConflict(e.cast()) == 0 {
                break;
            }
        }

        // XXX: Hack: Set hTarget!=0 so that ship_preprocess() does not
        //   call ship_transition() for us.
        (*e).hTarget = (*ss).hShip;
    }

    // This function is called when the ship dies but reincarnates.
    // The generic ship_death() function is not called for the ship in this case.
    pub(crate) unsafe extern "C" fn pkunk_reincarnation_death(ship: *mut ELEMENT) {
        // Simulate ship death
        StopAllBattleMusic();
        StartShipExplosion(ship, true);
        // Once the explosion ends, we will get a brand new ship
        (*ship).death_func = Some(ElementHook::Legacy(new_pkunk));
    }

    pub(crate) unsafe extern "C" fn intercept_pkunk_death(e: *mut ELEMENT) {
        let ss = starship(e);
        let desc = (*ss).RaceDescPtr;
        let data = get_custom_ship_data(desc);

        if (*desc).ship_info.crew_level != 0 {
            // Ship not dead yet.
            // Keep the Phoenix element alive.
            (*e).state_flags &= !(DISAPPEARING as ELEMENT_FLAGS);
            (*e).__bindgen_anon_1.life_span = 1;
            return;
        }

        let ship = element((*ss).hShip);
        // GRAVITY_MASS() indicates a warp-out here. If Pkunk dies while warping
        // out, there is no reincarnation.
        if !gravity_mass(i32::from((*ship).__bindgen_anon_3.mass_points) + 1) {
            // XXX: Hack: Set mass_points to indicate a reincarnation to
            //   FindAliveStarShip()
            (*ship).__bindgen_anon_3.mass_points = (MAX_SHIP_MASS + 1) as BYTE;
            // Save the various element processing callbacks before the
            // explosion happens, because we were not the ones who set
            // these callbacks and they are about to be changed.
            (*data).saved_preprocess_func = (*ship).preprocess_func;
            (*data).saved_postprocess_func = (*ship).postprocess_func;
            (*data).saved_death_func = (*ship).death_func;

            (*ship).death_func = Some(ElementHook::Legacy(pkunk_reincarnation_death));
        }
    }

    pub(crate) unsafe extern "C" fn spawn_phoenix_trail(e: *mut ELEMENT) {
        (*e).colorCycleIndex = (*e).colorCycleIndex.wrapping_add(1);
        if usize::from((*e).colorCycleIndex) != PHOENIX_TRAIL_COLORS.len() {
            (*e).__bindgen_anon_1.life_span = TRANSITION_LIFE;

            (*display_prim(e)).color = PHOENIX_TRAIL_COLORS[usize::from((*e).colorCycleIndex)];

            (*e).state_flags &= !(DISAPPEARING as ELEMENT_FLAGS);
            (*e).state_flags |= CHANGING as ELEMENT_FLAGS;
        } // else, the element disappears.
    }

    pub(crate) unsafe extern "C" fn phoenix_transition(e: *mut ELEMENT) {
        let ss = starship(e);
        let ship = element((*ss).hShip);

        if (*ship).state_flags & NONSOLID as ELEMENT_FLAGS == 0 {
            (*e).preprocess_func = None;
        } else {
            let h_ship_image = AllocElement();
            if !h_ship_image.is_null() {
                put_element(h_ship_image);

                let image = element(h_ship_image);
                (*image).playerNr = NEUTRAL_PLAYER_NUM as SIZE;
                (*image).state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as ELEMENT_FLAGS;
                (*image).__bindgen_anon_1.life_span = TRANSITION_LIFE;
                (*display_prim(image)).Type = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
                (*display_prim(image)).color = START_PHOENIX_COLOR;
                (*image).colorCycleIndex = 0;
                image!(copy; (*image).current.image, (*e).current.image);
                (*image).current.location = (*e).current.location;

                let angle: COUNT;
                if (*e).state_flags & PLAYER_SHIP as ELEMENT_FLAGS == 0 {
                    angle = COUNT::from((*e).__bindgen_anon_3.mass_points);

                    (*image).current.location.x = (i32::from((*image).current.location.x)
                        + i32::from(cosine(i32::from(angle), TRANSITION_SPEED)))
                        as COORD;
                    (*image).current.location.y = (i32::from((*image).current.location.y)
                        + i32::from(sine(i32::from(angle), TRANSITION_SPEED)))
                        as COORD;
                    (*e).preprocess_func = None;
                } else {
                    angle = facing_to_angle(i32::from((*ss).ShipFacing)) as COUNT;

                    let trail = i32::from((*e).__bindgen_anon_1.life_span) - 1;
                    (*image).current.location.x = (i32::from((*image).current.location.x)
                        - i32::from(cosine(i32::from(angle), TRANSITION_SPEED)) * trail)
                        as COORD;
                    (*image).current.location.y = (i32::from((*image).current.location.y)
                        - i32::from(sine(i32::from(angle), TRANSITION_SPEED)) * trail)
                        as COORD;

                    (*image).current.location.x =
                        wrap_val(i32::from((*image).current.location.x), log_space_width())
                            as COORD;
                    (*image).current.location.y =
                        wrap_val(i32::from((*image).current.location.y), log_space_height())
                            as COORD;
                }

                (*image).__bindgen_anon_3.mass_points = angle as BYTE;
                (*image).preprocess_func = Some(ElementHook::Legacy(phoenix_transition));
                (*image).death_func = Some(ElementHook::Legacy(spawn_phoenix_trail));
                (*image).pParent = ss.cast();
            }
        }
    }

    pub(crate) unsafe extern "C" fn pkunk_preprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let desc = (*ss).RaceDescPtr;
        let data = get_custom_ship_data(desc);

        if (*e).state_flags & APPEARING as ELEMENT_FLAGS != 0 {
            let mut h_phoenix: HELEMENT = ptr::null_mut();

            if TFB_Random() & 1 != 0 {
                h_phoenix = AllocElement();
            }

            // The hPhoenix element is created and placed at the head of the
            // queue so that it is preprocessed before any of the ships' elements
            // are, and so before death_func() is called for the dead Pkunk.
            // hPhoenix detects when the Pkunk ship dies and tweaks the ship,
            // starting the death + reincarnation sequence.
            if !h_phoenix.is_null() {
                let phoenix = element(h_phoenix);
                (*phoenix).playerNr = (*e).playerNr;
                (*phoenix).state_flags = (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as ELEMENT_FLAGS;
                (*phoenix).__bindgen_anon_1.life_span = 1;

                (*phoenix).death_func = Some(ElementHook::Legacy(intercept_pkunk_death));

                (*phoenix).pParent = ss.cast();

                InsertQueue((&raw mut disp_q).cast(), h_phoenix, disp_q.head);
            }
            (*data).hPhoenix = h_phoenix;

            // XXX: Hack: new_pkunk() sets hTarget!=0 which indicates a
            //   reincarnation to us.
            if (*e).hTarget.is_null() {
                // A brand new ship is preprocessed only once
                (*desc).preprocess_func = None;
            } else {
                // Start the reincarnation sequence
                ProcessSound(SetAbsStringTableIndex((*desc).ship_data.ship_sounds, 1), e);

                (*e).__bindgen_anon_1.life_span = PHOENIX_LIFE;
                (*display_prim(e)).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                (*e).state_flags |= (NONSOLID | FINITE_LIFE | CHANGING) as ELEMENT_FLAGS;

                let facing = (*ss).ShipFacing;
                let mut angle = OCTANT as COUNT;
                while angle < FULL_CIRCLE as COUNT {
                    (*ss).ShipFacing =
                        normalize_facing(i32::from(facing) + angle_to_facing(i32::from(angle)));
                    phoenix_transition(e);
                    angle += QUADRANT as COUNT;
                }
                (*ss).ShipFacing = facing;
            }
        }

        if (*desc).preprocess_func.is_some() {
            (*ss).cur_status_flags &= !((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as STATUS_FLAGS);

            if (*e).__bindgen_anon_1.life_span == NORMAL_LIFE as COUNT {
                image!(frame; (*e).current.image.frame, SetEquFrameIndex(*(*e).current.image.farray, (*e).current.image.frame));
                image!(copy; (*e).next.image.frame, (*e).current.image.frame);
                (*display_prim(e)).Type = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                init_intersect_start_point(e);
                init_intersect_end_point(e);
                init_intersect_frame(e);
                zero_velocity_components(&raw mut (*e).velocity);
                (*e).state_flags &= !((NONSOLID | FINITE_LIFE) as ELEMENT_FLAGS);
                (*e).state_flags |= CHANGING as ELEMENT_FLAGS;

                (*desc).preprocess_func = None;
            }
        }
    }

    pub(crate) unsafe extern "C" fn pkunk_postprocess(e: *mut ELEMENT) {
        let ss = starship(e);
        let desc = (*ss).RaceDescPtr;

        if (*desc).characteristics.special_wait != 0 {
            (*desc).characteristics.special_wait -= 1;
        } else if (*ss).cur_status_flags & SPECIAL as STATUS_FLAGS != 0
            && (*desc).ship_info.energy_level < (*desc).ship_info.max_energy
        {
            let mut cur_sound: COUNT;

            loop {
                // (COUNT)TFB_Random () % (GetSoundCount (...) - 2) is int math.
                let sound_count =
                    i32::from(GetStringTableCount((*desc).ship_data.ship_sounds.cast()));
                cur_sound = (2 + i32::from(TFB_Random() as COUNT) % (sound_count - 2)) as COUNT;
                if cur_sound != LAST_SOUND.load(Ordering::Relaxed) {
                    break;
                }
            }
            ProcessSound(
                SetAbsStringTableIndex((*desc).ship_data.ship_sounds, cur_sound),
                e,
            );
            LAST_SOUND.store(cur_sound, Ordering::Relaxed);

            DeltaEnergy(e, SPECIAL_ENERGY_COST as SIZE);

            (*desc).characteristics.special_wait = SPECIAL_WAIT;
        }
    }

    pub(crate) unsafe extern "C" fn uninit_pkunk(desc: *mut RACE_DESC) {
        set_custom_ship_data(desc, ptr::null());
    }
}
