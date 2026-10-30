//! Syreen callbacks registered with the live engine race descriptor.
//!
//! Every value and expression below mirrors `sc2/src/uqm/ships/syreen/syreen.c`,
//! including C integer promotion and truncation, so battles replay identically.

#[cfg(not(test))]
pub(crate) mod engine {
    use crate::ships::native::abi::*;

    // Core characteristics
    const MAX_CREW: u16 = 12;
    /// SYREEN_MAX_CREW_SIZE is MAX_CREW_SIZE from element.h
    const SYREEN_MAX_CREW_SIZE: u16 = MAX_CREW_SIZE as u16;
    const MAX_ENERGY: u8 = 16;
    const ENERGY_REGENERATION: u8 = 1;
    const ENERGY_WAIT: u8 = 6;
    const MAX_THRUST: u16 = 36;
    const THRUST_INCREMENT: u16 = 9;
    const THRUST_WAIT: u8 = 1;
    const TURN_WAIT: u8 = 1;
    const SHIP_MASS: u8 = 2;

    // Particle Beam Stiletto
    const WEAPON_ENERGY_COST: u8 = 1;
    const WEAPON_WAIT: u8 = 8;
    const SYREEN_OFFSET: i16 = 30;
    /// DISPLAY_TO_WORLD (30)
    const MISSILE_SPEED: i16 = 30 << ONE_SHIFT;
    const MISSILE_LIFE: u16 = 10;
    const MISSILE_HITS: i16 = 1;
    const MISSILE_DAMAGE: i16 = 2;
    const MISSILE_OFFSET: i16 = 3;

    // Syreen song
    const SPECIAL_ENERGY_COST: u8 = 5;
    const SPECIAL_WAIT: u8 = 20;
    /// Originally SPACE_HEIGHT.
    const ABANDONER_RANGE: i32 = 208;
    /// (DWORD)((UWORD)ABANDONER_RANGE * (UWORD)ABANDONER_RANGE)
    const ABANDONER_RANGE_SQUARED: u32 = (ABANDONER_RANGE * ABANDONER_RANGE) as u32;
    const MAX_ABANDONERS: i32 = 8;

    fn descriptor(_world: &mut crate::engine::battle::World) -> crate::engine::battle::RaceDesc {
        // All zero fields are null handles, absent callbacks, or zero counters.
        let mut desc = RaceTemplate::default();
        desc.info.ship_flags = FIRES_FORE as u16;
        desc.info.ship_cost = 13;
        desc.info.crew_level = MAX_CREW;
        desc.info.max_crew = SYREEN_MAX_CREW_SIZE;
        desc.info.energy_level = MAX_ENERGY;
        desc.info.max_energy = MAX_ENERGY;
        desc.resources.strings = Some(c"ship.syreen.text");
        desc.resources.icons = Some(c"ship.syreen.icons");
        desc.resources.melee_icon = Some(c"ship.syreen.meleeicons");
        desc.fleet_strength = 0;
        desc.fleet_location = POINT { x: 0, y: 0 };
        desc.characteristics = CHARACTERISTIC_STUFF {
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
        desc.resources.ship = [
            Some(c"ship.syreen.graphics.penetrator.large"),
            Some(c"ship.syreen.graphics.penetrator.medium"),
            Some(c"ship.syreen.graphics.penetrator.small"),
        ];
        desc.resources.weapon = [
            Some(c"ship.syreen.graphics.dagger.large"),
            Some(c"ship.syreen.graphics.dagger.medium"),
            Some(c"ship.syreen.graphics.dagger.small"),
        ];
        // special_rsc stays NULL_RESOURCE for all three views.
        desc.resources.captain = Some(c"ship.syreen.graphics.captain");
        desc.resources.victory = Some(c"ship.syreen.ditty");
        desc.resources.sounds = Some(c"ship.syreen.sounds");
        desc.intelligence.maneuverability = 0;
        desc.intelligence.weapon_range =
            (MISSILE_SPEED as i32 * MISSILE_LIFE as i32 * 2 / 3) as u16;
        desc.descriptor(&SYREEN)
    }
    use crate::engine::battle::views::Missile;
    use crate::engine::battle::{
        Concerns, Cx, ElementId, Race, RaceTemplate, ShipId, WeaponOrigin, WeaponSpawn,
    };
    pub static SYREEN: Race = Race {
        descriptor,
        preprocess: None,
        postprocess: Some(syreen_postprocess),
        init_weapon: Some(initialize_dagger),
        intelligence: Some(syreen_intelligence),
        uninit: None,
    };
    pub(crate) fn initialize_dagger(
        cx: &mut Cx<'_>,
        ss: ShipId,
        origin: WeaponOrigin,
    ) -> WeaponSpawn {
        let mut out = Vec::new();
        let mut block = Missile {
            cx: origin.next.location.x,
            cy: origin.next.location.y,
            farray: cx.ship_frame_set(ss, 1),
            face: cx.ship_state(ss).facing,
            index: cx.ship_state(ss).facing,
            sender: origin.player,
            flags: IGNORE_SIMILAR as u16,
            pixoffs: SYREEN_OFFSET,
            speed: MISSILE_SPEED,
            hit_points: MISSILE_HITS,
            damage: MISSILE_DAMAGE,
            life: MISSILE_LIFE,
            preprocess: None,
            blast_offs: MISSILE_OFFSET,
        };
        out.push(cx.missile(&mut block));
        out
    }

    /// Queued by the song's first pass and run again as the element's death
    /// function, which is when the nearby enemy ships actually lose crew.
    pub(crate) fn spawn_crew(cx: &mut Cx<'_>, source: ElementId) {
        if cx.element(source).flags & PLAYER_SHIP as u16 != 0 {
            let crew_handle = cx.alloc_element();
            if let Some(crew) = crew_handle {
                cx.element_mut(crew).next.location = cx.element(source).next.location;
                cx.element_mut(crew).player = cx.element(source).player;
                cx.element_mut(crew).flags = (APPEARING | NONSOLID | FINITE_LIFE) as u16;
                cx.element_mut(crew).life = 0;
                cx.element_mut(crew).death = Some(ElementHook::Native(spawn_crew));
                cx.set_parent(crew, Some(cx.owner(source)));
                cx.set_target(crew, None);

                cx.put(crew);
            }
            return;
        }

        let mut handle = cx.elements().head();
        while let Some(object) = handle {
            // AbandonShip may append to the queue, so read the link first.
            let next = cx.elements().succ(object);

            if cx.element(object).flags & PLAYER_SHIP as u16 != 0
                && cx.element(object).player != cx.element(source).player
                && cx.element(object).hits > 1
            {
                let mut dx = (i32::from(cx.element(object).next.location.x)
                    - i32::from(cx.element(source).next.location.x))
                    as SIZE;
                if dx < 0 {
                    dx = dx.wrapping_neg();
                }
                let mut dy = (i32::from(cx.element(object).next.location.y)
                    - i32::from(cx.element(source).next.location.y))
                    as SIZE;
                if dy < 0 {
                    dy = dy.wrapping_neg();
                }

                // WORLD_TO_DISPLAY
                dx >>= ONE_SHIFT;
                dy >>= ONE_SHIFT;
                if i32::from(dx) <= ABANDONER_RANGE && i32::from(dy) <= ABANDONER_RANGE {
                    // (DWORD)((UWORD)dx * (UWORD)dx): the product is formed in
                    // `int` before it is widened.
                    let square =
                        |d: SIZE| i32::from(d as u16).wrapping_mul(i32::from(d as u16)) as u32;
                    let d_squared = square(dx).wrapping_add(square(dy));
                    if d_squared <= ABANDONER_RANGE_SQUARED {
                        let mut crew_loss = ((MAX_ABANDONERS
                            * (ABANDONER_RANGE - i32::from(crate::math::square_root(d_squared))))
                            / ABANDONER_RANGE
                            + 1) as COUNT;
                        if crew_loss >= cx.element(object).hits {
                            crew_loss = cx.element(object).hits - 1;
                        }

                        cx.abandon_ship(object, source, crew_loss);
                    }
                }
            }

            handle = next;
        }
    }

    pub(crate) fn syreen_intelligence(cx: &mut Cx<'_>, ss: ShipId, concerns: &mut Concerns) {
        cx.ship_intelligence(ss, concerns);

        let enemy = ENEMY_SHIP_INDEX as usize;
        if concerns.objects[enemy].object.is_some() {
            let enemy_ss = cx.owner(
                concerns.objects[enemy]
                    .object
                    .unwrap_or_else(|| panic!("enemy ship")),
            );
            if cx.race_stats(enemy_ss).ship_info.ship_flags & CREW_IMMUNE as u16 == 0
                && cx.ship_state(ss).special_counter == 0
                && cx
                    .element(
                        concerns.objects[enemy]
                            .object
                            .unwrap_or_else(|| panic!("AI concern")),
                    )
                    .hits
                    > 1
                && concerns.objects[enemy].which_turn <= 14
            {
                cx.ship_state_mut(ss).input |= SPECIAL as u8;
            } else {
                cx.ship_state_mut(ss).input &= !(SPECIAL as u8);
            }
        }
    }

    pub(crate) fn syreen_postprocess(cx: &mut Cx<'_>, ss: ShipId) {
        let ship = cx.ship_element(ss);
        if cx.ship_state(ss).status & SPECIAL as u16 != 0
            && cx.ship_state(ss).special_counter == 0
            && cx.delta_energy(ship, -i16::from(SPECIAL_ENERGY_COST))
        {
            // SYREEN_SONG
            cx.process_ship_sound(ss, 1, ship);
            spawn_crew(cx, ship);

            cx.ship_state_mut(ss).special_counter = cx.race_stats(ss).characteristics.special_wait;
        }
    }
}
