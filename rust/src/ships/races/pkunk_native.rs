//! Pkunk callbacks and descriptor translated from `sc2/src/uqm/ships/pkunk/pkunk.c`.
//!
//! The C file leans on function-like macros from `element.h`, `units.h`,
//! `gfxlib.h`, `prim.h`, `collide.h` and `velocity.h`. They are expanded here
//! with the same integer promotions, truncations and (importantly) the same
//! number of evaluations of their arguments, because `TFB_Random` is one of
//! those arguments.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::{
        Concerns, Cx, ElementId, RaceState, Selection, ShipId, WeaponOrigin, WeaponSpawn,
    };
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, facing_to_angle, normalize_facing, sine,
    };
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

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let d = crate::engine::battle::template::RaceTemplate {
            info: crate::engine::battle::views::Info {
                ship_flags: (FIRES_FORE | FIRES_LEFT | FIRES_RIGHT) as UWORD,
                ship_cost: 20,
                crew_level: MAX_CREW,
                max_crew: MAX_CREW,
                energy_level: MAX_ENERGY,
                max_energy: MAX_ENERGY,
            },
            resources: crate::engine::battle::template::RaceResources {
                strings: Some(c"ship.pkunk.text"),
                icons: Some(c"ship.pkunk.icons"),
                melee_icon: Some(c"ship.pkunk.meleeicons"),
                ship: [
                    Some(c"ship.pkunk.graphics.fury.large"),
                    Some(c"ship.pkunk.graphics.fury.medium"),
                    Some(c"ship.pkunk.graphics.fury.small"),
                ],
                weapon: [
                    Some(c"ship.pkunk.graphics.bug.large"),
                    Some(c"ship.pkunk.graphics.bug.medium"),
                    Some(c"ship.pkunk.graphics.bug.small"),
                ],
                captain: Some(c"ship.pkunk.graphics.captain"),
                victory: Some(c"ship.pkunk.ditty"),
                sounds: Some(c"ship.pkunk.sounds"),
                ..Default::default()
            },
            fleet_strength: (666 / SPHERE_RADIUS_INCREMENT * 2) as COUNT,
            fleet_location: POINT { x: 502, y: 401 },
            characteristics: CHARACTERISTIC_STUFF {
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
            },
            intelligence: crate::engine::battle::views::Intelligence {
                weapon_range: (CLOSE_RANGE_WEAPON + 1) as COUNT,
                ..Default::default()
            },
            state: RaceState::Pkunk {
                phoenix: None,
                saved_pre: None,
                saved_post: None,
                saved_death: None,
            },
        };
        crate::engine::battle::kernel::LAST_PKUNK_SOUND
            .store(0, std::sync::atomic::Ordering::Relaxed);
        d.descriptor(&PKUNK)
    }

    pub static PKUNK: crate::engine::battle::Race = crate::engine::battle::Race {
        descriptor,
        preprocess: Some(pkunk_preprocess),
        postprocess: Some(pkunk_postprocess),
        init_weapon: Some(initialize_bug_missile),
        intelligence: Some(pkunk_intelligence),
        uninit: Some(uninit_pkunk),
    };

    fn display_align(random: u32, log_extent: i32) -> i32 {
        let reduced = i32::from(random as COUNT) % log_extent;
        i32::from(reduced as COORD) & !((1 << ONE_SHIFT) - 1)
    }

    fn wrap_val(v: i32, w: i32) -> COUNT {
        (if v < 0 {
            v + w
        } else if v >= w {
            v - w
        } else {
            v
        }) as COUNT
    }

    fn random_space_coord(cx: &mut Cx<'_>, extent: i32) -> COORD {
        let mut draw = || display_align(cx.random(), extent);
        let wrapped = if draw() < 0 {
            draw() + extent
        } else if draw() >= extent {
            draw() - extent
        } else {
            draw()
        };
        wrapped as COUNT as COORD
    }

    fn logical_dimensions(cx: &Cx<'_>) -> (i32, i32) {
        let (width, height) = cx.screen_dimensions();
        (
            ((width - STATUS_WIDTH) << ONE_SHIFT) << MAX_REDUCTION,
            (height << ONE_SHIFT) << MAX_REDUCTION,
        )
    }

    pub(crate) fn animate(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).turn > 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            let frame = cx.advance_frame(cx.element(e).current.image.frame);
            cx.set_frame(e, true, frame);
            cx.element_mut(e).flags |= CHANGING as ELEMENT_FLAGS;
            cx.element_mut(e).turn = cx.element(e).thrust;
        }
    }

    pub(crate) fn initialize_bug_missile(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = crate::engine::battle::views::Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: 0,
            index: 0,
            sender: origin.player,
            flags: IGNORE_SIMILAR as ELEMENT_FLAGS,
            pixoffs: PKUNK_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess: None,
            blast_offs: MISSILE_OFFSET,
        };
        let mut missiles = Vec::with_capacity(3);
        for i in 0..3 {
            let mut face =
                i32::from(cx.ship_state(ss).facing) + angle_to_facing(QUADRANT as i32) * i;
            if i == 2 {
                face += angle_to_facing(QUADRANT as i32);
            }
            block.face = normalize_facing(face);
            let missile = cx.missile(&mut block);
            missiles.push(missile);
            if let Some(missile) = missile {
                let (dx, dy) = origin.velocity.get_current_components();
                let (dx, dy) = (dx as SIZE, dy as SIZE);
                cx.element_mut(missile)
                    .velocity
                    .delta_components(i32::from(dx as SIZE), i32::from(dy as SIZE));
                cx.element_mut(missile).current.location.x =
                    (i32::from(cx.element(missile).current.location.x)
                        - (i32::from(dx) >> VELOCITY_SHIFT)) as COORD;
                cx.element_mut(missile).current.location.y =
                    (i32::from(cx.element(missile).current.location.y)
                        - (i32::from(dy) >> VELOCITY_SHIFT)) as COORD;
                cx.element_mut(missile).pre = Some(ElementHook::Native(animate));
            }
        }
        missiles
    }

    pub(crate) fn pkunk_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        let RaceState::Pkunk {
            phoenix,
            saved_pre,
            saved_post,
            saved_death,
        } = cx.race_stats(ss).state
        else {
            panic!("Pkunk state missing");
        };
        if let Some(phoenix) = phoenix.filter(|_| cx.ship_state(ss).control & STANDARD_RATING != 0)
        {
            cx.unlink(phoenix);
            cx.dispose(phoenix);
            cx.race_stats_mut(ss).state = RaceState::Pkunk {
                phoenix: None,
                saved_pre,
                saved_post,
                saved_death,
            };
        }
        let info = cx.race_stats(ss).ship_info;
        if info.energy_level < info.max_energy
            && (cx.ship_state(ss).special_counter == 0 || (cx.random() as BYTE) < 20)
        {
            cx.ship_state_mut(ss).input |= SPECIAL as BYTE;
        } else {
            cx.ship_state_mut(ss).input &= !(SPECIAL as BYTE);
        }
        cx.ship_intelligence(ss, concerns);
    }

    pub(crate) fn new_pkunk(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let RaceState::Pkunk {
            saved_pre,
            saved_post,
            saved_death,
            ..
        } = cx.race_stats(ss).state
        else {
            panic!("Pkunk state missing");
        };
        {
            let mut state = cx.element_mut(e);
            state.flags = (APPEARING | PLAYER_SHIP | IGNORE_SIMILAR) as ELEMENT_FLAGS;
            state.mass = SHIP_MASS;
            state.pre = saved_pre;
            state.post = saved_post;
            state.death = saved_death;
        }
        cx.set_preprocess(ss, Selection::Default);
        {
            let mut desc = cx.race_stats_mut(ss);
            desc.ship_info.crew_level = MAX_CREW;
            desc.ship_info.energy_level = MAX_ENERGY;
            desc.characteristics.max_thrust = MAX_THRUST;
            desc.characteristics.thrust_increment = THRUST_INCREMENT;
            desc.characteristics.turn_wait = TURN_WAIT;
            desc.characteristics.thrust_wait = THRUST_WAIT;
            desc.characteristics.special_wait = 0;
        }
        {
            let mut state = cx.ship_state_mut(ss);
            state.input = 0;
            state.status &= PLAY_VICTORY_DITTY;
            state.old_status = 0;
            state.energy_counter = 0;
            state.weapon_counter = 0;
            state.special_counter = 0;
        }
        {
            let mut state = cx.element_mut(e);
            state.hits = 0;
            state.turn = 0;
            state.thrust = 0;
            state.life = NORMAL_LIFE as COUNT;
        }
        cx.ship_state_mut(ss).facing = normalize_facing(cx.random() as i32);
        cx.set_frames(e, false, cx.ship_frame_set(ss, 0));
        let frame = cx.absolute_frame(cx.ship_frame(ss, 0), cx.ship_state(ss).facing);
        cx.set_frame(e, false, frame);
        cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
        let (width, height) = logical_dimensions(cx);
        loop {
            cx.element_mut(e).current.location.x = random_space_coord(cx, width);
            cx.element_mut(e).current.location.y = random_space_coord(cx, height);
            if !cx.gravity(e) && !cx.matter_conflict(e) {
                break;
            }
        }
        // ship_preprocess uses this self-target to skip the ordinary entry transition.
        cx.set_target(e, Some(cx.ship_element(ss)));
    }

    pub(crate) fn pkunk_reincarnation_death(cx: &mut Cx<'_>, ship: ElementId) {
        cx.stop_battle_music();
        cx.start_ship_explosion(ship, true);
        cx.element_mut(ship).death = Some(ElementHook::Native(new_pkunk));
    }

    pub(crate) fn intercept_pkunk_death(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        if cx.race_stats(ss).ship_info.crew_level != 0 {
            cx.element_mut(e).flags &= !(DISAPPEARING as ELEMENT_FLAGS);
            cx.element_mut(e).life = 1;
            return;
        }
        let ship = cx.ship_element(ss);
        if i32::from(cx.element(ship).mass) < MAX_SHIP_MASS as i32 * 10 {
            cx.element_mut(ship).mass = (MAX_SHIP_MASS + 1) as BYTE;
            let RaceState::Pkunk { phoenix, .. } = cx.race_stats(ss).state else {
                panic!("Pkunk state missing");
            };
            let state = cx.element(ship);
            cx.race_stats_mut(ss).state = RaceState::Pkunk {
                phoenix,
                saved_pre: state.pre,
                saved_post: state.post,
                saved_death: state.death,
            };
            cx.element_mut(ship).death = Some(ElementHook::Native(pkunk_reincarnation_death));
        }
    }

    pub(crate) fn spawn_phoenix_trail(cx: &mut Cx<'_>, e: ElementId) {
        cx.element_mut(e).color_cycle = cx.element(e).color_cycle.wrapping_add(1);
        let index = usize::from(cx.element(e).color_cycle);
        if index != PHOENIX_TRAIL_COLORS.len() {
            cx.element_mut(e).life = TRANSITION_LIFE;
            cx.primitive_style_mut(e).color = PHOENIX_TRAIL_COLORS[index];
            cx.element_mut(e).flags &= !(DISAPPEARING as ELEMENT_FLAGS);
            cx.element_mut(e).flags |= CHANGING as ELEMENT_FLAGS;
        }
    }

    pub(crate) fn phoenix_transition(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        let ship = cx.ship_element(ss);
        if cx.element(ship).flags & NONSOLID as ELEMENT_FLAGS == 0 {
            cx.element_mut(e).pre = None;
        } else if let Some(image) = cx.alloc_element() {
            cx.put(image);
            cx.element_mut(image).player = NEUTRAL_PLAYER_NUM as SIZE;
            cx.element_mut(image).flags = (APPEARING | FINITE_LIFE | NONSOLID) as ELEMENT_FLAGS;
            cx.element_mut(image).life = TRANSITION_LIFE;
            cx.primitive_style_mut(image).kind = gfx_object_STAMPFILL_PRIM as GRAPHICS_PRIM;
            cx.primitive_style_mut(image).color = START_PHOENIX_COLOR;
            cx.element_mut(image).color_cycle = 0;
            cx.copy_image_to_current(image, cx.element(e).current.image);
            cx.element_mut(image).current.location = cx.element(e).current.location;
            let angle;
            if cx.element(e).flags & PLAYER_SHIP as ELEMENT_FLAGS == 0 {
                angle = COUNT::from(cx.element(e).mass);
                cx.element_mut(image).current.location.x =
                    (i32::from(cx.element(image).current.location.x)
                        + i32::from(cosine(i32::from(angle), TRANSITION_SPEED)))
                        as COORD;
                cx.element_mut(image).current.location.y =
                    (i32::from(cx.element(image).current.location.y)
                        + i32::from(sine(i32::from(angle), TRANSITION_SPEED)))
                        as COORD;
                cx.element_mut(e).pre = None;
            } else {
                angle = facing_to_angle(i32::from(cx.ship_state(ss).facing)) as COUNT;
                let trail = i32::from(cx.element(e).life) - 1;
                cx.element_mut(image).current.location.x =
                    (i32::from(cx.element(image).current.location.x)
                        - i32::from(cosine(i32::from(angle), TRANSITION_SPEED)) * trail)
                        as COORD;
                cx.element_mut(image).current.location.y =
                    (i32::from(cx.element(image).current.location.y)
                        - i32::from(sine(i32::from(angle), TRANSITION_SPEED)) * trail)
                        as COORD;
                let (width, height) = logical_dimensions(cx);
                cx.element_mut(image).current.location.x =
                    wrap_val(i32::from(cx.element(image).current.location.x), width) as COORD;
                cx.element_mut(image).current.location.y =
                    wrap_val(i32::from(cx.element(image).current.location.y), height) as COORD;
            }
            cx.element_mut(image).mass = angle as BYTE;
            cx.element_mut(image).pre = Some(ElementHook::Native(phoenix_transition));
            cx.element_mut(image).death = Some(ElementHook::Native(spawn_phoenix_trail));
            cx.set_parent(image, Some(ss));
        }
    }

    pub(crate) fn pkunk_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.element(e).flags & APPEARING as ELEMENT_FLAGS != 0 {
            let phoenix = if cx.random() & 1 != 0 {
                cx.alloc_element()
            } else {
                None
            };
            if let Some(phoenix) = phoenix {
                cx.element_mut(phoenix).player = cx.element(e).player;
                cx.element_mut(phoenix).flags =
                    (FINITE_LIFE | NONSOLID | IGNORE_SIMILAR) as ELEMENT_FLAGS;
                cx.element_mut(phoenix).life = 1;
                cx.element_mut(phoenix).death = Some(ElementHook::Native(intercept_pkunk_death));
                cx.set_parent(phoenix, Some(ss));
                cx.insert_before(phoenix, cx.elements().head());
            }
            let RaceState::Pkunk {
                saved_pre,
                saved_post,
                saved_death,
                ..
            } = cx.race_stats(ss).state
            else {
                panic!("Pkunk state missing");
            };
            cx.race_stats_mut(ss).state = RaceState::Pkunk {
                phoenix,
                saved_pre,
                saved_post,
                saved_death,
            };
            if cx.target(e).is_none() {
                cx.set_preprocess(ss, Selection::Disabled);
            } else {
                cx.process_ship_sound(ss, 1, e);
                cx.element_mut(e).life = PHOENIX_LIFE;
                cx.primitive_style_mut(e).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                cx.element_mut(e).flags |= (NONSOLID | FINITE_LIFE | CHANGING) as ELEMENT_FLAGS;
                let facing = cx.ship_state(ss).facing;
                let mut angle = OCTANT as COUNT;
                while angle < FULL_CIRCLE as COUNT {
                    cx.ship_state_mut(ss).facing =
                        normalize_facing(i32::from(facing) + angle_to_facing(i32::from(angle)));
                    phoenix_transition(cx, e);
                    angle += QUADRANT as COUNT;
                }
                cx.ship_state_mut(ss).facing = facing;
            }
        }
        if !matches!(cx.callbacks(ss).pre, Selection::Disabled) {
            cx.ship_state_mut(ss).status &=
                !((LEFT | RIGHT | THRUST | WEAPON | SPECIAL) as STATUS_FLAGS);
            if cx.element(e).life == NORMAL_LIFE as COUNT {
                let frame = cx.equivalent_frame(
                    cx.element_frame_set_first(e, false),
                    cx.element(e).current.image.frame,
                );
                cx.set_frame(e, false, frame);
                cx.set_frame(e, true, frame);
                cx.primitive_style_mut(e).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                cx.init_intersect(e);
                let frame = cx.equivalent_frame(
                    cx.element_frame_set_first(e, true),
                    cx.element(e).next.image.frame,
                );
                cx.set_intersect_frame(e, frame);
                cx.element_mut(e).velocity = VELOCITY_DESC::default();
                cx.element_mut(e).flags &= !((NONSOLID | FINITE_LIFE) as ELEMENT_FLAGS);
                cx.element_mut(e).flags |= CHANGING as ELEMENT_FLAGS;
                cx.set_preprocess(ss, Selection::Disabled);
            }
        }
    }

    pub(crate) fn pkunk_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);
        if cx.race_stats(ss).characteristics.special_wait != 0 {
            cx.race_stats_mut(ss).characteristics.special_wait -= 1;
        } else if cx.ship_state(ss).status & SPECIAL as STATUS_FLAGS != 0
            && cx.race_stats(ss).ship_info.energy_level < cx.race_stats(ss).ship_info.max_energy
        {
            let mut sound;
            loop {
                let count = i32::from(cx.ship_sound_count(ss));
                sound = (2 + i32::from(cx.random() as COUNT) % (count - 2)) as COUNT;
                if sound != cx.last_pkunk_sound() {
                    break;
                }
            }
            cx.process_ship_sound(ss, sound, e);
            cx.set_last_pkunk_sound(sound);
            cx.delta_energy(e, SPECIAL_ENERGY_COST as SIZE);
            cx.race_stats_mut(ss).characteristics.special_wait = SPECIAL_WAIT;
        }
    }

    pub(crate) fn uninit_pkunk(
        _world: &mut crate::engine::battle::World,
        stats: &mut crate::engine::battle::views::RaceStats,
    ) {
        stats.state = RaceState::None;
    }
}
