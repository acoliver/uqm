//! Live Sa-Matra callbacks translated from ships/lastbat/lastbat.c.

pub use super::samatra_state as observation;
#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::race::SaMatraState;
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceState, RaceTemplate, ShipId, StoryKey,
    };
    use crate::ships::native::{
        abi::*, angle_to_facing, cosine, normalize_facing, sine, wrap_delta,
    };

    use super::observation;
    const COMET_SPEED: SIZE = 12 << ONE_SHIFT;
    const SENTINEL_SPEED: SIZE = 8 << ONE_SHIFT;
    const RECOIL: i32 = (10 << ONE_SHIFT) << VELOCITY_SHIFT;
    const MAX_RECOIL: i32 = RECOIL * 4;
    const SHIP_MASS: u8 = (MAX_SHIP_MASS * 10) as u8;
    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = (IMMEDIATE_WEAPON | CREW_IMMUNE) as u16;
        d.info.ship_cost = 16;
        d.info.crew_level = 1;
        d.info.max_crew = 1;
        d.info.energy_level = MAX_ENERGY_SIZE as u8;
        d.info.max_energy = MAX_ENERGY_SIZE as u8;
        d.characteristics.energy_regeneration = 1;
        d.characteristics.weapon_energy_cost = 2;
        d.characteristics.special_energy_cost = 3;
        d.characteristics.energy_wait = 6;
        d.characteristics.weapon_wait = 240;
        d.characteristics.special_wait = 72;
        d.characteristics.ship_mass = SHIP_MASS;
        d.resources.ship = [
            Some(c"ship.samatra.graphics.samatra.large"),
            Some(c"ship.samatra.graphics.samatra.medium"),
            Some(c"ship.samatra.graphics.samatra.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.samatra.graphics.sentinel.large"),
            Some(c"ship.samatra.graphics.sentinel.medium"),
            Some(c"ship.samatra.graphics.sentinel.small"),
        ];
        d.resources.special = [
            Some(c"ship.samatra.graphics.generator.large"),
            Some(c"ship.samatra.graphics.generator.medium"),
            Some(c"ship.samatra.graphics.generator.small"),
        ];
        d.resources.captain = Some(c"ship.samatra.graphics.captain");
        d.resources.sounds = Some(c"ship.samatra.sounds");
        d.state = RaceState::SaMatra(SaMatraState::default());
        d.descriptor(&SAMATRA)
    }
    pub static SAMATRA: Race = Race {
        descriptor,
        preprocess: Some(samatra_preprocess),
        postprocess: Some(samatra_postprocess),
        init_weapon: None,
        intelligence: Some(samatra_intelligence),
        uninit: None,
    };
    fn arena(cx: &Cx<'_>) -> (i32, i32) {
        let (width, height) = cx.screen_dimensions();
        ((width - 64) << (ONE_SHIFT + 3), height << (ONE_SHIFT + 3))
    }
    fn state(cx: &Cx<'_>, ss: ShipId) -> SaMatraState {
        let RaceState::SaMatra(state) = cx.race_stats(ss).state else {
            panic!("Sa-Matra has no owned state")
        };
        state
    }
    fn edit_state(cx: &mut Cx<'_>, ss: ShipId, edit: impl FnOnce(&mut SaMatraState)) {
        let mut state = state(cx, ss);
        edit(&mut state);
        cx.race_stats_mut(ss).state = RaceState::SaMatra(state);
    }
    pub(crate) fn comet_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let index = cx.frame_number(cx.element(e).current.image.frame) + 1;
        if index < 29 {
            if index == 25 {
                let ss = cx.owner(e);
                cx.race_stats_mut(ss).characteristics.weapon_wait = cx
                    .race_stats(ss)
                    .characteristics
                    .weapon_wait
                    .wrapping_add(1);
                spawn_comet(cx, e);
                cx.element_mut(e).flags |= NONSOLID as u16;
                let (dx, dy) = cx.element(e).velocity.get_current_components();
                cx.element_mut(e)
                    .velocity
                    .set_components(i32::from(dx as SIZE / 2), i32::from(dy as SIZE / 2));
            }
            cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
        }
        cx.set_frame(
            e,
            true,
            cx.absolute_frame(cx.element(e).current.image.frame, index),
        );
        cx.element_mut(e).flags |= CHANGING as u16;
    }
    pub(crate) fn comet_collision(
        cx: &mut Cx<'_>,
        e: ElementId,
        p: &mut POINT,
        o: ElementId,
        q: &mut POINT,
    ) {
        if cx.element(o).player == RPG_PLAYER_NUM as SIZE {
            cx.element_mut(e).mass = if cx.element(o).flags & PLAYER_SHIP as u16 != 0 {
                2
            } else {
                50
            };
            let hits = cx.element(e).hits as u8;
            let life = cx.element(e).life;
            let blast = cx.weapon_collision(e, p, o, q);
            if cx.element(o).flags & PLAYER_SHIP as u16 != 0 {
                cx.element_mut(e).hits = u16::from(hits);
                cx.element_mut(e).life = life;
                cx.element_mut(e).flags &= !((DISAPPEARING | NONSOLID | COLLISION) as u16);
                if let Some(blast) = blast {
                    cx.unlink(blast);
                    cx.dispose(blast);
                }
            }
            if cx.element(e).flags & DISAPPEARING as u16 != 0 {
                let ss = cx.owner(e);
                cx.race_stats_mut(ss).characteristics.weapon_wait = cx
                    .race_stats(ss)
                    .characteristics
                    .weapon_wait
                    .wrapping_sub(1);
            }
        }
    }
    fn spawn_comet(cx: &mut Cx<'_>, e: ElementId) -> Option<ElementId> {
        let ss = cx.owner(e);
        let mut b = Missile {
            cx: cx.element(e).next.location.x,
            cy: cx.element(e).next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: 0,
            index: 24,
            sender: cx.element(e).player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: 0,
            speed: 0,
            hit_points: 12,
            damage: 2,
            life: 2,
            preprocess: Some(comet_preprocess),
            blast_offs: 0,
        };
        let h = cx.missile(&mut b);
        if let Some(h) = h {
            cx.put(h);
            let c = h;
            cx.element_mut(c).collide = Some(CollisionHook::Native(comet_collision));
            cx.set_parent(c, Some(ss));
            cx.element_mut(c).turn = cx.element(e).turn;
            cx.set_target(c, cx.target(e));
            let mut facing;
            if cx.element(e).flags & PLAYER_SHIP as u16 != 0 {
                cx.element_mut(c).turn = 0;
                facing = cx.random() as COUNT;
                cx.element_mut(c)
                    .velocity
                    .set_vector(i32::from(COMET_SPEED), facing);
            } else {
                cx.element_mut(c).velocity = cx.element(e).velocity;
                cx.element_mut(c).hits = cx.element(e).hits;
                facing = angle_to_facing(cx.element(c).velocity.travel_angle as i32) as COUNT;
            }
            if cx.element(c).turn != 0 {
                cx.element_mut(c).turn -= 1;
            } else {
                facing = normalize_facing(i32::from(facing));
                if cx.track_ship(c, &mut facing) > 0 {
                    cx.element_mut(c)
                        .velocity
                        .set_vector(i32::from(COMET_SPEED), facing);
                }
                cx.element_mut(c).turn = 3;
            }
        }
        h
    }
    pub(crate) fn turret_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).turn > 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            cx.set_frame(
                e,
                true,
                cx.absolute_frame(
                    cx.element(e).current.image.frame,
                    cx.frame_number(cx.element(e).current.image.frame) % 10 + 1,
                ),
            );
            cx.element_mut(e).flags |= CHANGING as u16;
            cx.element_mut(e).turn = 0;
        }
    }
    pub(crate) fn gate_collision(
        cx: &mut Cx<'_>,
        e: ElementId,
        p: &mut POINT,
        o: ElementId,
        q: &mut POINT,
    ) {
        if cx.element(o).player == RPG_PLAYER_NUM as SIZE {
            let ss = cx.owner(e);
            if cx.race_stats(ss).characteristics.max_thrust == 0 {
                if cx.element(o).flags & FINITE_LIFE as u16 == 0 {
                    cx.element_mut(e).flags |= COLLISION as u16;
                }
                if cx.element(o).flags & PLAYER_SHIP as u16 != 0
                    && cx.primitive_style_mut(e).kind == gfx_object_STAMPFILL_PRIM as u8
                    && cx.story_state(StoryKey::BombCarrier) != 0
                {
                    edit_state(cx, ss, |state| state.gate_entered = true);
                    cx.set_activity(cx.world.activity() & !(IN_BATTLE as u16));
                }
            } else {
                cx.element_mut(e).mass = if cx.element(o).flags & PLAYER_SHIP as u16 != 0 {
                    1
                } else {
                    50
                };
                cx.element_mut(e).hits = 100;
                let blast = cx.weapon_collision(e, p, o, q);
                cx.element_mut(e).flags &= !((DISAPPEARING | NONSOLID | COLLISION) as u16);
                cx.element_mut(e).life = NORMAL_LIFE as COUNT;
                if let Some(blast) = blast {
                    cx.unlink(blast);
                    cx.dispose(blast);
                }
            }
        }
    }
    pub(crate) fn gate_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        if cx.race_stats(ss).characteristics.max_thrust == 0 {
            cx.element_mut(e).mass = SHIP_MASS;
            cx.element_mut(e).flags &= !(FINITE_LIFE as u16);
            cx.element_mut(e).life = (NORMAL_LIFE + 1) as COUNT;
            cx.element_mut(e).pre = None;
            cx.primitive_style_mut(e).color = Color {
                r: 0,
                g: 0,
                b: 0,
                a: 255,
            };
            cx.primitive_style_mut(e).kind = gfx_object_STAMPFILL_PRIM as u8;
        } else {
            cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
            cx.set_frame(e, true, cx.advance_frame(cx.element(e).current.image.frame));
            if cx.frame_number(cx.element(e).next.image.frame) == 0 {
                cx.set_frame(
                    e,
                    true,
                    cx.absolute_frame(cx.element(e).next.image.frame, 11),
                );
            }
            cx.element_mut(e).flags |= CHANGING as u16;
        }
    }
    pub(crate) fn generator_death(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).flags & FINITE_LIFE as u16 == 0 {
            let ss = cx.owner(e);
            let id = state(cx, ss)
                .generators
                .iter()
                .position(|g| *g == Some(e))
                .unwrap_or_else(|| panic!("registered generator"));
            edit_state(cx, ss, |state| state.destroyed |= 1 << id);
            cx.race_stats_mut(ss).characteristics.max_thrust =
                cx.race_stats(ss).characteristics.max_thrust.wrapping_sub(1);
            cx.element_mut(e).flags |= (FINITE_LIFE | NONSOLID) as u16;
            cx.element_mut(e).pre = None;
            cx.element_mut(e).turn = 12;
            cx.element_mut(e).thrust = 0;
            cx.set_frame(
                e,
                false,
                cx.absolute_frame(cx.element(e).current.image.frame, 9),
            );
        }
        if cx.element(e).thrust != 0 {
            cx.element_mut(e).thrust -= 1;
            cx.element_mut(e).flags &= !(DISAPPEARING as u16);
            cx.element_mut(e).flags |= CHANGING as u16;
            cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
        } else {
            let wait = cx.element(e).turn;
            cx.element_mut(e).turn = wait.wrapping_sub(1);
            if wait != 0 {
                cx.element_mut(e).flags &= !(DISAPPEARING as u16);
                cx.element_mut(e).flags |= CHANGING as u16;
                cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
                cx.set_frame(e, true, cx.advance_frame(cx.element(e).current.image.frame));
                cx.element_mut(e).thrust = 1;
            }
        }
    }
    pub(crate) fn generator_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).turn > 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            cx.element_mut(e).turn = ((15 - i32::from(cx.element(e).hits)) / 5) as u8;
            if cx.element(e).turn < 3 {
                cx.set_frame(
                    e,
                    true,
                    cx.absolute_frame(
                        cx.element(e).current.image.frame,
                        (cx.frame_number(cx.element(e).current.image.frame) + 1) % 10,
                    ),
                );
                cx.element_mut(e).flags |= CHANGING as u16;
            }
        }
    }
    pub(crate) fn generator_collision(
        cx: &mut Cx<'_>,
        e: ElementId,
        _p: &mut POINT,
        o: ElementId,
        _q: &mut POINT,
    ) {
        if cx.element(o).flags & FINITE_LIFE as u16 == 0 {
            cx.element_mut(e).flags |= COLLISION as u16;
        }
    }
    fn sentinel_target(cx: &Cx<'_>, e: ElementId, ss: ShipId) -> Option<ElementId> {
        let own_ship = cx.optional_ship_element(ss);
        if cx.target(e).is_none() {
            own_ship
        } else if own_ship.is_none() {
            cx.target(e)
        } else {
            let ship = own_ship.unwrap_or_else(|| panic!("present boss element"));
            let enemy = cx
                .target(e)
                .unwrap_or_else(|| panic!("present enemy target"));
            let dx0 = cx
                .element(ship)
                .current
                .location
                .x
                .wrapping_sub(cx.element(e).current.location.x);
            let dy0 = cx
                .element(ship)
                .current
                .location
                .y
                .wrapping_sub(cx.element(e).current.location.y);
            let dx1 = cx
                .element(ship)
                .current
                .location
                .x
                .wrapping_sub(cx.element(enemy).current.location.x);
            let dy1 = cx
                .element(ship)
                .current
                .location
                .y
                .wrapping_sub(cx.element(enemy).current.location.y);
            if i64::from(dx0) * i64::from(dx0) + i64::from(dy0) * i64::from(dy0)
                > i64::from(dx1) * i64::from(dx1) + i64::from(dy1) * i64::from(dy1)
            {
                own_ship
            } else {
                cx.target(e)
            }
        }
    }
    pub(crate) fn sentinel_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        let ss = cx.owner(e);
        cx.race_stats_mut(ss).characteristics.special_wait = cx
            .race_stats(ss)
            .characteristics
            .special_wait
            .wrapping_add(1);
        cx.element_mut(e).life = cx.element(e).life.wrapping_add(1);
        if cx.element(e).thrust != 0 {
            cx.element_mut(e).thrust -= 1;
        } else {
            cx.set_frame(
                e,
                true,
                cx.absolute_frame(
                    cx.element(e).current.image.frame,
                    (cx.frame_number(cx.element(e).current.image.frame) + 1) % 6,
                ),
            );
            cx.element_mut(e).flags |= CHANGING as u16;
            cx.element_mut(e).thrust = 1;
        }
        if cx.element(e).turn > 0 {
            cx.element_mut(e).turn -= 1;
        } else {
            let mut facing;
            if cx.element(e).flags & NONSOLID as u16 == 0 {
                facing = angle_to_facing(cx.element(e).velocity.travel_angle as i32) as COUNT;
            } else {
                cx.element_mut(e).flags &= !(NONSOLID as u16);
                facing = cx.random() as COUNT;
                cx.element_mut(e)
                    .velocity
                    .set_vector(i32::from(SENTINEL_SPEED), facing);
            }
            facing = normalize_facing(i32::from(facing));
            if cx.target(e).is_none() {
                let mut f = facing;
                cx.track_ship(e, &mut f);
            }
            let target = sentinel_target(cx, e, ss);
            if let Some(t) = target {
                let (width, height) = arena(cx);
                let mut dx = wrap_delta(
                    cx.element(t)
                        .current
                        .location
                        .x
                        .wrapping_sub(cx.element(e).current.location.x),
                    width,
                );
                let mut dy = wrap_delta(
                    cx.element(t)
                        .current
                        .location
                        .y
                        .wrapping_sub(cx.element(e).current.location.y),
                    height,
                );
                let frames = (crate::math::square_root(
                    (i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)) as u32,
                ) >> 6)
                    .max(1);
                let mut v = cx.element(t).velocity;
                let (vx, vy) = v.get_next_components(frames);
                (dx, dy) = (vx as SIZE, vy as SIZE);
                dx = wrap_delta(
                    cx.element(t)
                        .current
                        .location
                        .x
                        .wrapping_add(dx)
                        .wrapping_sub(cx.element(e).current.location.x),
                    width,
                );
                dy = wrap_delta(
                    cx.element(t)
                        .current
                        .location
                        .y
                        .wrapping_add(dy)
                        .wrapping_sub(cx.element(e).current.location.y),
                    height,
                );
                dx = normalize_facing(
                    angle_to_facing(crate::math::arctan(dx, dy) as i32) - i32::from(facing),
                ) as SIZE;
                if dx > 0 {
                    if dx <= angle_to_facing(HALF_CIRCLE as i32) as SIZE {
                        facing = facing.wrapping_add(1);
                    } else {
                        facing = facing.wrapping_sub(1);
                    }
                }
                cx.element_mut(e)
                    .velocity
                    .set_vector(i32::from(SENTINEL_SPEED), facing);
            }
            cx.element_mut(e).turn = 1;
        }
    }
    fn apply_recoil(cx: &mut Cx<'_>, o: ElementId, angle: u16) {
        cx.element_mut(o).velocity.delta_components(
            i32::from(cosine(i32::from(angle), RECOIL)),
            i32::from(sine(i32::from(angle), RECOIL)),
        );
        let (dx, dy) = cx.element(o).velocity.get_current_components();
        let (dx, dy) = (dx as SIZE, dy as SIZE);
        if i64::from(dx) * i64::from(dx) + i64::from(dy) * i64::from(dy)
            > i64::from(MAX_RECOIL) * i64::from(MAX_RECOIL)
        {
            let angle = crate::math::arctan(dx, dy);
            cx.element_mut(o).velocity.set_components(
                i32::from(cosine(i32::from(angle), MAX_RECOIL)),
                i32::from(sine(i32::from(angle), MAX_RECOIL)),
            );
        }
    }
    pub(crate) fn sentinel_collision(
        cx: &mut Cx<'_>,
        e: ElementId,
        p: &mut POINT,
        o: ElementId,
        q: &mut POINT,
    ) {
        if cx.element(o).player == NPC_PLAYER_NUM as SIZE {
            if cx.element(e).pre.map(|f| f.address()) == cx.element(o).pre.map(|f| f.address())
                && cx.element(e).flags & DEFY_PHYSICS as u16 == 0
                && (p.x != cx.weapon_origin(e).intersection().IntersectStamp.origin.x
                    || p.y != cx.weapon_origin(e).intersection().IntersectStamp.origin.y)
            {
                let angle = crate::math::arctan(p.x.wrapping_sub(q.x), p.y.wrapping_sub(q.y));
                cx.element_mut(e).velocity.set_components(
                    i32::from(cosine(
                        i32::from(angle),
                        i32::from(SENTINEL_SPEED) << VELOCITY_SHIFT,
                    )),
                    i32::from(sine(
                        i32::from(angle),
                        i32::from(SENTINEL_SPEED) << VELOCITY_SHIFT,
                    )),
                );
                cx.element_mut(e).turn = 1;
                cx.element_mut(e).flags |= (COLLISION | DEFY_PHYSICS) as u16;
            }
        } else {
            let hits = cx.element(e).hits as u8;
            let life = cx.element(e).life;
            cx.element_mut(e).thrust = 0;
            let blast = cx.weapon_collision(e, p, o, q);
            cx.element_mut(e).thrust = 0;
            if cx.element(o).flags & PLAYER_SHIP as u16 != 0
                && cx.element(o).hits != 0
                && u32::from(cx.element(o).mass) < MAX_SHIP_MASS * 10
            {
                cx.element_mut(e).life = life;
                cx.element_mut(e).hits = u16::from(hits);
                cx.element_mut(e).flags &= !(DISAPPEARING as u16);
                cx.element_mut(e).flags |= DEFY_PHYSICS as u16;
                cx.element_mut(e).turn = 12;
                let ss = cx.owner(o);
                cx.ship_state_mut(ss).status &=
                    !((SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED) as u16);
                if cx.element(o).turn < COLLISION_TURN_WAIT as u8 {
                    cx.element_mut(o).turn += COLLISION_TURN_WAIT as u8;
                }
                if cx.element(o).thrust < COLLISION_THRUST_WAIT as u8 {
                    cx.element_mut(o).thrust += COLLISION_THRUST_WAIT as u8;
                }
                apply_recoil(cx, o, cx.element(e).velocity.travel_angle);
                cx.element_mut(e).velocity = VELOCITY_DESC::new();
            }
            if cx.element(e).flags & DISAPPEARING as u16 != 0 {
                let ss = cx.owner(e);
                cx.race_stats_mut(ss).characteristics.special_wait = cx
                    .race_stats(ss)
                    .characteristics
                    .special_wait
                    .wrapping_sub(1);
                if let Some(blast) = blast {
                    let b = blast;
                    cx.element_mut(b).life = 6;
                    cx.set_frame(
                        b,
                        false,
                        cx.absolute_frame(cx.element_frame_set_first(b, false), 6),
                    );
                }
            }
        }
    }
    pub(crate) fn samatra_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.ship_intelligence(ss, concerns);
    }
    pub(crate) fn samatra_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);

        if cx.race_stats(ss).characteristics.max_thrust != 0 {
            if cx.ship_state(ss).weapon_counter == 0
                && cx.race_stats(ss).characteristics.weapon_wait < 3
                && spawn_comet(cx, e).is_some()
            {
                cx.ship_state_mut(ss).weapon_counter = 240;
            }
            if cx.ship_state(ss).special_counter == 0
                && cx.race_stats(ss).characteristics.special_wait < 4
            {
                let mut b = Missile {
                    cx: cx.element(e).next.location.x,
                    cy: cx.element(e).next.location.y,
                    farray: cx.ship_frame_set(ss, 1),
                    face: 0,
                    index: 0,
                    sender: cx.element(e).player,
                    flags: 0,
                    pixoffs: 0,
                    speed: SENTINEL_SPEED,
                    hit_points: 10,
                    damage: 1,
                    life: 2,
                    preprocess: Some(sentinel_preprocess),
                    blast_offs: 0,
                };
                let h = cx.missile(&mut b);
                if let Some(h) = h {
                    let s = h;
                    cx.element_mut(s).collide = Some(CollisionHook::Native(sentinel_collision));
                    cx.element_mut(s).turn = 3;
                    cx.set_parent(s, Some(ss));
                    cx.ship_state_mut(ss).special_counter = 72;
                    cx.put(h);
                }
            }
        }
    }
    fn spawn_generators(cx: &mut Cx<'_>, ss: ShipId, e: ElementId, center_x: i32, center_y: i32) {
        let offsets = observation::OFFSETS;
        cx.race_stats_mut(ss).characteristics.max_thrust = 0;
        while cx.race_stats(ss).characteristics.max_thrust < 8 {
            let h = cx.alloc_element();
            if let Some(h) = h {
                let g = h;
                let slot = cx.race_stats(ss).characteristics.max_thrust as usize;
                edit_state(cx, ss, |state| state.generators[slot] = Some(g));
                cx.element_mut(g).hits = 15;
                cx.element_mut(g).mass = SHIP_MASS;
                cx.element_mut(g).life = NORMAL_LIFE as COUNT;
                cx.element_mut(g).player = cx.element(e).player;
                cx.element_mut(g).flags = (APPEARING | IGNORE_SIMILAR) as u16;
                cx.primitive_style_mut(g).kind = gfx_object_STAMP_PRIM as u8;
                let (x, y) = offsets[cx.race_stats(ss).characteristics.max_thrust as usize];
                cx.element_mut(g).current.location = POINT {
                    x: ((center_x + (x << ONE_SHIFT)) & !15) as i16,
                    y: ((center_y + (y << ONE_SHIFT)) & !15) as i16,
                };
                cx.set_frames(g, false, cx.ship_frame_set(ss, 2));
                let index = u16::from(cx.random() as u8 % 10);
                cx.set_frame(g, false, cx.absolute_frame(cx.ship_frame(ss, 2), index));
                cx.element_mut(g).pre = Some(ElementHook::Native(generator_preprocess));
                cx.element_mut(g).collide = Some(CollisionHook::Native(generator_collision));
                cx.element_mut(g).death = Some(ElementHook::Native(generator_death));
                cx.set_parent(g, Some(ss));
                cx.insert_before(h, cx.elements().head());
            }
            cx.race_stats_mut(ss).characteristics.max_thrust += 1;
        }
    }
    pub(crate) fn samatra_preprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let e = cx.ship_element(ss);

        cx.race_stats_mut(ss).characteristics.weapon_wait = 0;
        cx.race_stats_mut(ss).characteristics.special_wait = 0;
        if cx.element(e).flags & APPEARING as u16 == 0 {
            cx.element_mut(e).turn = cx.element(e).turn.wrapping_add(1);
            cx.element_mut(e).thrust = cx.element(e).thrust.wrapping_add(1);
        } else {
            edit_state(cx, ss, |state| {
                *state = SaMatraState {
                    generation: state.generation + 1,
                    ..SaMatraState::default()
                }
            });
            let (width, height) = arena(cx);
            let center_x = width >> 1;
            let center_y = height >> 1;
            spawn_generators(cx, ss, e, center_x, center_y);
            let h = cx.alloc_element();
            if let Some(h) = h {
                let t = h;
                cx.element_mut(t).hits = 1;
                cx.element_mut(t).life = NORMAL_LIFE as COUNT;
                cx.element_mut(t).player = cx.element(e).player;
                cx.element_mut(t).flags = (APPEARING | IGNORE_SIMILAR | NONSOLID) as u16;
                cx.primitive_style_mut(t).kind = gfx_object_STAMP_PRIM as u8;
                cx.element_mut(t).current.location = POINT {
                    x: center_x as i16,
                    y: center_y as i16,
                };
                cx.set_frames(t, false, cx.ship_frame_set(ss, 0));
                cx.set_frame(t, false, cx.absolute_frame(cx.ship_frame(ss, 0), 1));
                cx.element_mut(t).pre = Some(ElementHook::Native(turret_preprocess));
                cx.set_parent(t, Some(ss));
                cx.insert_before(h, cx.elements().succ(e));
            }
            let h = cx.alloc_element();
            if let Some(h) = h {
                let g = h;
                cx.element_mut(g).hits = 100;
                cx.element_mut(g).mass = 1;
                cx.element_mut(g).life = 2;
                cx.element_mut(g).player = cx.element(e).player;
                cx.element_mut(g).flags = (APPEARING | FINITE_LIFE | IGNORE_SIMILAR) as u16;
                cx.primitive_style_mut(g).kind = gfx_object_STAMP_PRIM as u8;
                cx.element_mut(g).current.location = POINT {
                    x: center_x as i16,
                    y: center_y as i16,
                };
                cx.set_frames(g, false, cx.ship_frame_set(ss, 0));
                cx.set_frame(g, false, cx.absolute_frame(cx.ship_frame(ss, 0), 11));
                cx.element_mut(g).pre = Some(ElementHook::Native(gate_preprocess));
                cx.element_mut(g).collide = Some(CollisionHook::Native(gate_collision));
                cx.set_parent(g, Some(ss));
                cx.insert_before(h, cx.elements().succ(e));
            }
            cx.ship_state_mut(ss).weapon_counter = 120;
            cx.ship_state_mut(ss).special_counter = 36;
        }
    }
}
