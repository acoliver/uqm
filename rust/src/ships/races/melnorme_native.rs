//! Melnorme callbacks and descriptor for the live battle engine.
//! Expressions preserve the C promotions, truncations, traversal and RNG order.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceDesc, RaceTemplate, Selection, ShipId, WeaponOrigin,
        WeaponSpawn, World,
    };

    use crate::ships::native::{abi::*, cosine, facing_to_angle, sine};

    const MAX_CREW: u16 = 20;
    const MAX_ENERGY: u8 = 42;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 4;
    const MAX_THRUST: u16 = 36;
    const THRUST_INCREMENT: u16 = 6;
    const THRUST_WAIT: u8 = 4;
    const TURN_WAIT: u8 = 4;
    const SHIP_MASS: u8 = 7;
    const WEAPON_ENERGY_COST: u8 = 5;
    const WEAPON_WAIT: u8 = 1;
    const MELNORME_OFFSET: i16 = 24;
    const MELNORME_SPEED: i16 = 24 << ONE_SHIFT;
    const LEVEL_COUNTER: u8 = 72;
    const MAX_PUMP: u8 = 4;
    const PUMPUP_SPEED: i16 = 45 << ONE_SHIFT;
    const PUMPUP_LIFE: u16 = 10;
    const PUMPUP_DAMAGE: u8 = 2;
    const MIN_PUMPITUDE_ANIMS: u16 = 3;
    const NUM_PUMP_ANIMS: u16 = 5;
    const REVERSE_DIR: u8 = 1 << 7;
    const SPECIAL_ENERGY_COST: u8 = 20;
    const SPECIAL_WAIT: u8 = 20;
    const CMISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const CMISSILE_LIFE: u16 = 20;
    const CMISSILE_HITS: i16 = 200;
    const CMISSILE_DAMAGE: i16 = 0;
    const CMISSILE_OFFSET: i16 = 4;
    const MAX_UNIVERSE: i16 = 9999;

    pub static MELNORME: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: Some(melnorme_postprocess),
        init_weapon: Some(initialize_pump_up),
        intelligence: Some(melnorme_intelligence),
        uninit: None,
    };

    fn descriptor(_world: &mut World) -> RaceDesc {
        let mut d = RaceTemplate::default();
        d.info.ship_flags = FIRES_FORE as u16;
        d.info.ship_cost = 18;
        d.info.crew_level = MAX_CREW;
        d.info.max_crew = MAX_CREW;
        d.info.energy_level = MAX_ENERGY;
        d.info.max_energy = MAX_ENERGY;
        d.resources.strings = Some(c"ship.melnorme.text");
        d.resources.icons = Some(c"ship.melnorme.icons");
        d.resources.melee_icon = Some(c"ship.melnorme.meleeicons");
        d.fleet_strength = u16::MAX;
        d.fleet_location = POINT {
            x: MAX_UNIVERSE >> 1,
            y: MAX_UNIVERSE >> 1,
        };
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
            special_wait: SPECIAL_WAIT,
            ship_mass: SHIP_MASS,
        };
        d.resources.ship = [
            Some(c"ship.melnorme.graphics.trader.large"),
            Some(c"ship.melnorme.graphics.trader.medium"),
            Some(c"ship.melnorme.graphics.trader.small"),
        ];
        d.resources.weapon = [
            Some(c"ship.melnorme.graphics.pumpup.large"),
            Some(c"ship.melnorme.graphics.pumpup.medium"),
            Some(c"ship.melnorme.graphics.pumpup.small"),
        ];
        d.resources.special = [
            Some(c"ship.melnorme.graphics.confuse.large"),
            Some(c"ship.melnorme.graphics.confuse.medium"),
            Some(c"ship.melnorme.graphics.confuse.small"),
        ];
        d.resources.captain = Some(c"ship.melnorme.graphics.captain");
        d.resources.victory = Some(c"ship.melnorme.ditty");
        d.resources.sounds = Some(c"ship.melnorme.sounds");
        d.intelligence.weapon_range = (PUMPUP_SPEED as i32 * PUMPUP_LIFE as i32) as u16;
        d.descriptor(&MELNORME)
    }
    pub(crate) fn pump_up_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        cx.element_mut(e).thrust = cx.element(e).thrust.wrapping_sub(1);
        if cx.element(e).thrust & 1 != 0 {
            let mut idx = cx.frame_number(cx.element(e).current.image.frame);
            let turning_back = cx.element(e).turn & REVERSE_DIR != 0;
            if (turning_back && !idx.is_multiple_of(NUM_PUMP_ANIMS))
                || (!turning_back && (i32::from(idx) + 1) % i32::from(NUM_PUMP_ANIMS) == 0)
            {
                idx = idx.wrapping_sub(1);
                cx.element_mut(e).turn |= REVERSE_DIR;
            } else {
                idx = idx.wrapping_add(1);
                cx.element_mut(e).turn &= !REVERSE_DIR;
            }
            cx.set_frame(
                e,
                true,
                cx.absolute_frame(cx.element(e).current.image.frame, idx),
            );
            cx.element_mut(e).flags |= CHANGING as u16;
        }
    }

    pub(crate) fn pump_up_postprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).flags & APPEARING as u16 != 0 {
            cx.element_mut(e).velocity = crate::engine::battle::record::VELOCITY_DESC::default();
            return;
        }
        let ss = cx.owner(e);
        let ship = cx.ship_element(ss);
        let origin = cx.weapon_origin(ship);
        let missile = initialize_pump_up(cx, ss, origin)[0];
        cx.delta_energy(ship, 0);
        let p = missile.unwrap_or_else(|| panic!("Melnorme replacement pulse allocation"));
        cx.set_frame(p, false, cx.element(e).current.image.frame);
        cx.element_mut(p).turn = cx.element(e).turn;
        cx.element_mut(p).thrust = cx.element(e).thrust;
        cx.element_mut(p).thrust = cx.element(p).thrust.wrapping_sub(1);
        if cx.element(p).thrust == 0 {
            if (cx.element(p).turn & !REVERSE_DIR) < MAX_PUMP - 1 {
                cx.element_mut(p).turn += 1;
                cx.set_frame(
                    p,
                    false,
                    cx.relative_frame(cx.element(p).current.image.frame, NUM_PUMP_ANIMS as i16),
                );
                cx.process_ship_sound(ss, 2, p);
            }
            cx.element_mut(p).thrust = LEVEL_COUNTER;
        }
        // C computes damage from the old pulse, even on the growth tick.
        let damage = i32::from(PUMPUP_DAMAGE) << (cx.element(e).turn & !REVERSE_DIR);
        cx.element_mut(p).hits = damage as COUNT;
        cx.element_mut(p).mass = cx.element(p).hits as BYTE;
        cx.set_parent(p, Some(ss));
        if cx.element(p).thrust & 1 != 0 {
            let mut idx = cx.frame_number(cx.element(p).current.image.frame);
            let back = cx.element(p).turn & REVERSE_DIR != 0;
            if (back && !idx.is_multiple_of(NUM_PUMP_ANIMS))
                || (!back && (i32::from(idx) + 1) % i32::from(NUM_PUMP_ANIMS) == 0)
            {
                idx = idx.wrapping_sub(1);
                cx.element_mut(p).turn |= REVERSE_DIR;
            } else {
                idx = idx.wrapping_add(1);
                cx.element_mut(p).turn &= !REVERSE_DIR;
            }
            cx.set_frame(
                p,
                false,
                cx.absolute_frame(cx.element(p).current.image.frame, idx),
            );
        }
        if cx.ship_state(ss).status & cx.ship_state(ss).old_status & WEAPON as u16 != 0 {
            cx.ship_state_mut(ss).weapon_counter = WEAPON_WAIT;
        } else {
            cx.element_mut(p).life = PUMPUP_LIFE;
            cx.element_mut(p).pre = Some(ElementHook::Native(pump_up_preprocess));
            cx.element_mut(p).post = None;
            let a = facing_to_angle(i32::from(cx.ship_state(ss).facing)) as COUNT;
            cx.element_mut(p).velocity.set_components(
                i32::from(cosine(
                    i32::from(a),
                    i32::from(PUMPUP_SPEED) << VELOCITY_SHIFT,
                )),
                i32::from(sine(
                    i32::from(a),
                    i32::from(PUMPUP_SPEED) << VELOCITY_SHIFT,
                )),
            );
            cx.process_ship_sound(ss, 3, p);
        }
        cx.put(p);
        cx.primitive_style_mut(e).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
        cx.element_mut(e).flags |= NONSOLID as u16;
    }

    pub(crate) fn pump_up_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) {
        let rect = cx.frame_rect(cx.element(a).next.image.frame);
        let wait = cx.element(a).thrust;
        cx.element_mut(a).thrust = (i32::from(rect.extent.width) >> 1) as BYTE;
        let blast = cx.weapon_collision(a, pa, b, pb);
        cx.element_mut(a).thrust = wait;
        if let Some(p) = blast {
            cx.element_mut(p).life =
                MIN_PUMPITUDE_ANIMS + u16::from(cx.element(a).turn & !REVERSE_DIR);
            cx.element_mut(p).turn = 0;
            cx.element_mut(p).thrust = 0;
            cx.element_mut(p).pre = Some(ElementHook::Native(animation_preprocess));
            cx.set_frames(p, false, cx.element(a).next.image.frames);
            cx.set_frame(
                p,
                false,
                cx.absolute_frame(
                    cx.element_frame_set_first(p, false),
                    u16::from(MAX_PUMP) * NUM_PUMP_ANIMS,
                ),
            );
        }
    }

    pub(crate) fn initialize_pump_up(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: 0,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: MELNORME_OFFSET,
            speed: MELNORME_SPEED,
            hit_points: PUMPUP_DAMAGE as i16,
            damage: PUMPUP_DAMAGE as i16,
            life: 2,
            preprocess: None,
            blast_offs: 0,
        };
        let missile = cx.missile(&mut block);
        if let Some(p) = missile {
            cx.element_mut(p).post = Some(ElementHook::Native(pump_up_postprocess));
            cx.element_mut(p).collide = Some(CollisionHook::Native(pump_up_collision));
            cx.element_mut(p).thrust = LEVEL_COUNTER;
        }
        vec![missile]
    }

    pub(crate) fn confuse_preprocess(cx: &mut Cx<'_>, e: ElementId) {
        if cx.element(e).flags & NONSOLID as u16 == 0 {
            cx.set_frame(
                e,
                true,
                cx.absolute_frame(
                    cx.element(e).current.image.frame,
                    (cx.frame_number(cx.element(e).current.image.frame)
                        .wrapping_add(1))
                        & 7,
                ),
            );
            cx.element_mut(e).flags |= CHANGING as u16;
        } else if cx.target(e).is_none() {
            cx.element_mut(e).life = 0;
            cx.element_mut(e).flags |= DISAPPEARING as u16;
        } else {
            let target = cx
                .target(e)
                .unwrap_or_else(|| panic!("Melnorme confusion target"));
            cx.element_mut(e).next.location = cx.element(target).next.location;
            if cx.element(e).turn != 0 {
                let mut next = cx.advance_frame(cx.element(e).current.image.frame);
                if cx.frame_number(next) == 0 {
                    next = cx.relative_frame(next, -8);
                }
                cx.set_frame(e, true, next);
                let target_ss = cx.owner(target);
                cx.ship_state_mut(target_ss).input = (cx.ship_state(target_ss).input
                    & !((LEFT | RIGHT | SPECIAL) as u8))
                    | cx.element(e).turn;
                if let Some(p) = cx.alloc_element() {
                    cx.element_mut(p).player = cx.element(e).player;
                    cx.element_mut(p).flags = (FINITE_LIFE | NONSOLID | CHANGING) as u16;
                    cx.element_mut(p).life = 1;
                    cx.element_mut(p).current.location = cx.element(e).next.location;
                    cx.copy_image_to_current(p, cx.element(e).next.image);
                    cx.copy_visual_to_next(p, cx.element(e).next);
                    cx.element_mut(p).pre = Some(ElementHook::Native(confuse_preprocess));
                    cx.primitive_style_mut(p).kind = gfx_object_STAMP_PRIM as GRAPHICS_PRIM;
                    cx.set_parent(p, Some(cx.owner(e)));
                    cx.set_target(p, cx.target(e));
                    cx.put(p);
                }
            }
        }
    }

    pub(crate) fn confusion_collision(
        cx: &mut Cx<'_>,
        a: ElementId,
        _pa: &mut POINT,
        b: ElementId,
        _pb: &mut POINT,
    ) {
        if cx.element(b).flags & PLAYER_SHIP as u16 == 0 {
            return;
        }
        let ss = cx.owner(a);
        let mut h = cx.elements().head();
        let mut found = None;
        while let Some(p) = h {
            if cx.element(p).player == cx.element(a).player
                && cx.element(p).current.image.frames == cx.ship_frame_set(ss, 2)
                && cx.element(p).flags & NONSOLID as u16 != 0
            {
                found = Some(p);
                break;
            }
            h = cx.elements().succ(p);
        }
        if found.is_none() {
            found = cx.alloc_element();
        }
        if let Some(p) = found {
            if cx.element(p).flags == 0 {
                cx.insert_before(p, cx.elements().head());
                cx.element_mut(p).current.location = cx.element(a).next.location;
                cx.copy_image_to_current(p, cx.element(a).next.image);
                cx.set_frame(
                    p,
                    false,
                    cx.absolute_frame(cx.element(p).current.image.frame, 8),
                );
                cx.copy_visual_to_next(p, cx.element(p).current);
                cx.element_mut(p).player = cx.element(a).player;
                cx.element_mut(p).flags = (FINITE_LIFE | NONSOLID | CHANGING) as u16;
                cx.element_mut(p).pre = Some(ElementHook::Native(confuse_preprocess));
                cx.primitive_style_mut(p).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                cx.set_parent(p, Some(ss));
                let target_ss = cx.owner(b);
                cx.set_target(p, Some(cx.ship_element(target_ss)));
            }
            cx.element_mut(p).life = 400;
            cx.element_mut(p).turn = 1u8 << (cx.random() as u8 & 1);
        }
        cx.element_mut(a).hits = 0;
        cx.element_mut(a).life = 0;
        cx.element_mut(a).flags |= (DISAPPEARING | COLLISION | NONSOLID) as u16;
    }

    pub(crate) fn initialize_confusion(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 2),
            index: 0,
            face: cx.ship_state(ss).facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: MELNORME_OFFSET,
            speed: CMISSILE_SPEED,
            hit_points: CMISSILE_HITS,
            damage: CMISSILE_DAMAGE,
            life: CMISSILE_LIFE,
            preprocess: Some(confuse_preprocess),
            blast_offs: CMISSILE_OFFSET,
        };
        let missile = cx.missile(&mut block);
        if let Some(p) = missile {
            cx.element_mut(p).collide = Some(CollisionHook::Native(confusion_collision));
            cx.set_parent(p, Some(ss));
        }
        vec![missile]
    }

    pub(crate) fn initialize_test_pump_up(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: 0,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: MELNORME_OFFSET,
            speed: PUMPUP_SPEED,
            hit_points: PUMPUP_DAMAGE as i16,
            damage: PUMPUP_DAMAGE as i16,
            life: PUMPUP_LIFE,
            preprocess: None,
            blast_offs: 0,
        };
        vec![cx.missile(&mut block)]
    }

    pub(crate) fn melnorme_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.set_weapon(ss, Selection::Override(initialize_test_pump_up));
        let old = cx.ship_state(ss).weapon_counter;
        if old == WEAPON_WAIT {
            cx.ship_state_mut(ss).weapon_counter = 0;
        }
        let enemy = ENEMY_SHIP_INDEX as usize;
        if let Some(target) = concerns.objects[enemy].object {
            if cx.race_stats(ss).ship_info.energy_level < SPECIAL_ENERGY_COST + WEAPON_ENERGY_COST
                && cx.ship_state(ss).old_status & WEAPON as u16 == 0
            {
                concerns.objects[enemy].movement = MOVEMENT_STATE_ENTICE;
            } else {
                let es = cx.owner(target);
                if cx.race_stats(es).ship_info.ship_flags & IMMEDIATE_WEAPON as u16 == 0 {
                    concerns.objects[enemy].movement = MOVEMENT_STATE_PURSUE;
                }
            }
        }
        cx.ship_intelligence(ss, concerns);
        if cx.ship_state(ss).weapon_counter == 0
            && (old != 0
                || ((cx.ship_state(ss).special_counter != 0
                    || cx.race_stats(ss).ship_info.energy_level
                        >= SPECIAL_ENERGY_COST + WEAPON_ENERGY_COST)
                    && cx.ship_state(ss).input & WEAPON as u8 == 0))
        {
            cx.ship_state_mut(ss).input ^= WEAPON as u8;
        }
        cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
        if cx.ship_state(ss).special_counter == 0
            && cx.race_stats(ss).ship_info.energy_level >= SPECIAL_ENERGY_COST
        {
            let old_input = cx.ship_state(ss).input;
            cx.set_weapon(ss, Selection::Override(initialize_confusion));
            concerns.subject.turn = concerns.subject.turn.wrapping_add(1);
            concerns.subject.thrust = concerns.subject.thrust.wrapping_add(1);
            let count = concerns.count;
            concerns.count = (ENEMY_SHIP_INDEX + 1) as COUNT;
            cx.ship_intelligence(ss, concerns);
            concerns.count = count;
            concerns.subject.thrust = concerns.subject.thrust.wrapping_sub(1);
            concerns.subject.turn = concerns.subject.turn.wrapping_sub(1);
            if cx.ship_state(ss).input & WEAPON as u8 != 0 {
                cx.ship_state_mut(ss).input &= !(WEAPON as u8);
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            }
            cx.ship_state_mut(ss).input = old_input | (cx.ship_state(ss).input & SPECIAL as u8);
        }
        cx.ship_state_mut(ss).weapon_counter = old;
        cx.set_weapon(ss, Selection::Override(initialize_pump_up));
    }

    pub(crate) fn melnorme_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(ship, -i16::from(SPECIAL_ENERGY_COST))
        {
            let origin = cx.weapon_origin(ship);
            if let Some(p) = initialize_confusion(cx, ss, origin)[0] {
                cx.process_ship_sound(ss, 1, p);
                cx.put(p);
                cx.ship_state_mut(ss).special_counter =
                    cx.race_stats(ss).characteristics.special_wait;
            }
        }
    }
}
