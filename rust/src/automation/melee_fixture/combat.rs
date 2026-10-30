//! Combat setup and proof decisions over the currently borrowed production records.

use super::*;

pub(super) fn mutual_loss(
    run: &mut Battle,
    world: &mut World<'_>,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0 => {
            if world.activity != (SUPER_MELEE | IN_BATTLE) as u16 || world.counters != [2, 2] {
                return Err("mutual-loss fixture requires an active 2-vs-2 Super Melee".into());
            }
            [run.a, run.b] = world.fighters()?;
            if world.ship(run.a).state.SpeciesID != SPECIES_ID_SHOFIXTI_ID
                || world
                    .ship(run.b)
                    .info
                    .as_ref()
                    .ok_or("fixture requires loaded descriptor fields")?
                    .crew_level
                    > 6
            {
                return Err(
                    "mutual-loss fixture requires a Shofixti against at most six crew".into(),
                );
            }
            place(
                world
                    .elements
                    .get_mut(&run.a)
                    .ok_or("fixture element or owner is no longer live")?,
                4000,
                4000,
            );
            place(
                world
                    .elements
                    .get_mut(&run.b)
                    .ok_or("fixture element or owner is no longer live")?,
                4160,
                4000,
            );
            let ship = world.ship_mut(run.a);
            ship.state.old_status_flags &= !(SPECIAL as u16);
            ship.state.cur_status_flags |= SPECIAL as u16;
            run.next(vec![
                Effect::Intersect(run.a),
                Effect::Intersect(run.b),
                Effect::Captain(
                    ship.data
                        .as_ref()
                        .ok_or("fixture requires loaded descriptor fields")?
                        .captain_control
                        .special,
                ),
            ])
        }
        1 => {
            let ship = world.ship_mut(run.a);
            ship.data
                .as_mut()
                .ok_or("fixture requires loaded descriptor fields")?
                .captain_control
                .special = run.captain;
            run.next(vec![Effect::RaceProcess(true, run.a)])
        }
        _ => {
            assert_eq!(world.elements[&run.a].life_span, 0);
            Ok(None)
        }
    }
}

pub(super) fn blazer(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    let a = run.a;
    match run.stage {
        0 => {
            let special = world
                .ship(a)
                .data
                .as_ref()
                .ok_or("fixture requires loaded descriptor fields")?
                .special
                .as_ptr()
                .cast_mut()
                .cast();
            let transformed = world.elements[&a].next.farray == special;
            let ship = world.ship_mut(a);
            ship.state.cur_status_flags &= !(SPECIAL as u16);
            ship.info
                .as_mut()
                .ok_or("fixture requires loaded descriptor fields")?
                .energy_level = 0;
            run.next(if transformed {
                vec![Effect::RaceProcess(false, a)]
            } else {
                vec![]
            })
        }
        1 => {
            run.saved_hook = world.elements[&a].collision_func.map_or(0, |f| f.address());
            let ship = world.ship_mut(a);
            let info = ship
                .info
                .as_mut()
                .ok_or("fixture requires loaded descriptor fields")?;
            info.energy_level = info.max_energy;
            ship.state.cur_status_flags |= SPECIAL as u16;
            run.next(vec![
                Effect::RaceProcess(false, a),
                Effect::RaceProcess(true, a),
            ])
        }
        2 => {
            run.proof.require(
                world.elements[&a].collision_func.map_or(0, |f| f.address())
                    == hooks.blazer_collision,
                "blazer-installed",
            )?;
            let ship = world.ship_mut(a);
            ship.state.cur_status_flags &= !(SPECIAL as u16);
            ship.info
                .as_mut()
                .ok_or("fixture requires loaded descriptor fields")?
                .energy_level = 0;
            run.next(vec![Effect::RaceProcess(false, a)])
        }
        3 => {
            run.proof.require(
                world.elements[&a].collision_func.map_or(0, |f| f.address()) == run.saved_hook,
                "collision-restored",
            )?;
            run.proof.require(
                world.elements[&a].next.farray
                    == world
                        .ship(a)
                        .data
                        .as_ref()
                        .ok_or("fixture requires loaded descriptor fields")?
                        .ship
                        .as_ptr()
                        .cast_mut()
                        .cast(),
                "ship-frames-restored",
            )?;
            run.next(vec![])
        }
        _ => run.finish(8),
    }
}

pub(super) fn satellites(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0 => {
            let satellites: Vec<_> = world
                .order
                .iter()
                .copied()
                .filter(|h| {
                    world.elements[h].postprocess_func.map(|f| f.address())
                        == hooks.satellite.map(|f| f.address())
                })
                .collect();
            run.proof
                .require(satellites.len() == 3, "three-satellites")?;
            run.object = satellites[0];
            let location = world.elements[&run.object].current.location;
            place(
                world
                    .elements
                    .get_mut(&run.b)
                    .ok_or("fixture element or owner is no longer live")?,
                location.x + 100,
                location.y,
            );
            world
                .elements
                .get_mut(&run.object)
                .ok_or("fixture element or owner is no longer live")?
                .thrust_or_blast = 0;
            run.before = world.elements.keys().copied().collect();
            let hook = hooks.satellite;
            run.next(vec![
                Effect::Intersect(run.b),
                Effect::Invoke(hook, run.object),
            ])
        }
        1 => {
            let delayed = world
                .elements
                .iter()
                .find_map(|(&h, e)| {
                    (!run.before.contains(&h)
                        && e.death_func.map_or(0, |f| f.address()) == hooks.defense)
                        .then_some(e)
                })
                .ok_or("satellite did not queue delayed defense")?;
            run.proof.require(
                !delayed.pred.is_null() && delayed.h_target == run.object,
                "delayed-defense",
            )?;
            run.crew = world
                .ship(run.b)
                .info
                .as_ref()
                .ok_or("fixture requires loaded descriptor fields")?
                .crew_level;
            run.next(vec![])
        }
        2 if run.wait < 6 => {
            run.wait += 1;
            run.traverse()
        }
        _ => {
            run.proof.require(
                world
                    .ship(run.b)
                    .info
                    .as_ref()
                    .ok_or("fixture requires loaded descriptor fields")?
                    .crew_level
                    < run.crew,
                "defense-fired",
            )?;
            Ok(None)
        }
    }
}

pub(super) fn death_spawn(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0 => {
            let crew = world
                .ship(run.b)
                .info
                .as_ref()
                .ok_or("fixture requires loaded descriptor fields")?
                .crew_level;
            let charge = run.prepare_weapon(world, run.a)?;
            let mut effects = vec![charge];
            if crew > 1 {
                effects.push(Effect::Crew(run.b, 1 - crew as i16));
            }
            effects.push(Effect::Weapon(run.a));
            run.next(effects)
        }
        1 => {
            let weapon = run.weapon(world, run.a)?;
            run.next(vec![Effect::Collide(weapon, run.b)])
        }
        2 => {
            run.proof.require(
                world
                    .ship(run.b)
                    .info
                    .as_ref()
                    .ok_or("fixture requires loaded descriptor fields")?
                    .crew_level
                    == 0
                    && world.elements[&run.b].life_span == 0,
                "collision-killed",
            )?;
            run.proof.require(
                world.elements[&run.b].death_func.map(|f| f.address())
                    == hooks.ship_death.map(|f| f.address()),
                "death-hook",
            )?;
            run.before = world.elements.keys().copied().collect();
            run.next(vec![])
        }
        3 if run.wait < 9 => {
            run.wait += 1;
            run.traverse()
        }
        _ => {
            run.proof.require(
                world
                    .elements
                    .iter()
                    .any(|(h, e)| !run.before.contains(h) && e.has_finite_life()),
                "spawn-during-traversal",
            )?;
            Ok(None)
        }
    }
}

pub(super) fn mixed(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0 => {
            run.crew = u16::from(
                world
                    .ship(run.b)
                    .characteristics
                    .as_ref()
                    .ok_or("fixture requires loaded descriptor fields")?
                    .turn_wait,
            );
            run.before = world.elements.keys().copied().collect();
            run.next(vec![Effect::Invoke(hooks.limpets, run.a)])
        }
        1 => {
            run.object = *world
                .order
                .iter()
                .find(|h| !run.before.contains(h))
                .ok_or("production VUX limpet allocation failed")?;
            run.next(vec![Effect::Collide(run.object, run.b)])
        }
        2 => {
            run.proof.require(
                u16::from(
                    world
                        .ship(run.b)
                        .characteristics
                        .as_ref()
                        .ok_or("fixture requires loaded descriptor fields")?
                        .turn_wait,
                ) == run.crew + 1
                    && world.elements[&run.object].life_span == 0,
                "limpet-hit",
            )?;
            run.crew = world
                .ship(run.a)
                .info
                .as_ref()
                .ok_or("fixture requires loaded descriptor fields")?
                .crew_level;
            let charge = run.prepare_weapon(world, run.b)?;
            run.next(vec![charge, Effect::Weapon(run.b)])
        }
        3 => {
            run.object = run.weapon(world, run.b)?;
            run.next(vec![Effect::Collide(run.object, run.a)])
        }
        4 => {
            run.proof.require(
                world
                    .ship(run.a)
                    .info
                    .as_ref()
                    .ok_or("fixture requires loaded descriptor fields")?
                    .crew_level
                    < run.crew
                    && world.elements[&run.object].life_span == 0,
                "howitzer-hit",
            )?;
            run.next(vec![])
        }
        _ => run.finish(8),
    }
}

pub(super) fn pool(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0 => {
            let objects = world
                .order
                .iter()
                .copied()
                .filter(|h| !world.elements[h].is_player_ship())
                .collect();
            run.next(vec![Effect::Remove(objects)])
        }
        1 => {
            assert_eq!(
                world.elements.len(),
                2,
                "fixture retains both Ur-Quan ships"
            );
            let info = world
                .ship_mut(run.a)
                .info
                .as_mut()
                .ok_or("fixture requires loaded descriptor fields")?;
            info.max_crew = 200;
            info.crew_level = 200;
            world
                .elements
                .get_mut(&run.a)
                .ok_or("fixture element or owner is no longer live")?
                .crew_or_hp = 200;
            run.before = world.elements.keys().copied().collect();
            run.next(
                (0..76)
                    .map(|_| Effect::Invoke(hooks.fighters, run.a))
                    .collect(),
            )
        }
        2 => {
            let fighters = world
                .elements
                .values()
                .filter(|e| {
                    e.preprocess_func.map_or(0, |f| f.address()) == hooks.fighter_preprocess
                })
                .count();
            run.proof.require(
                world.elements.len() == 150 && fighters >= 140,
                "fighters-filled-pool",
            )?;
            run.next(vec![Effect::ProbePool])
        }
        3 => {
            let objects = world
                .order
                .iter()
                .filter(|h| !run.before.contains(h))
                .copied()
                .collect();
            run.next(vec![Effect::Remove(objects), Effect::ProbeRecovery])
        }
        _ => run.finish(8),
    }
}
