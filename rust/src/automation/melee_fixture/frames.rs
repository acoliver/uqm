//! Keep the outgoing descriptor alive until every projectile and blast retires.

use super::*;

pub(super) fn advance(
    run: &mut Battle,
    world: &mut World,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0..=2 => opening(run, world, hooks),
        3..=5 => replacement(run, world),
        6 => threat(run, world),
        7 => collision(run, world),
        8 => blast(run, world),
        9 => retire(run, world),
        _ => zoom(run, world),
    }
}

fn opening(
    run: &mut Battle,
    world: &mut World,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0 => {
            place(
                world
                    .elements
                    .get_mut(&run.a())
                    .ok_or("fixture element or owner is no longer live")?,
                1000,
                1000,
            );
            place(
                world
                    .elements
                    .get_mut(&run.b())
                    .ok_or("fixture element or owner is no longer live")?,
                5000,
                5000,
            );
            let charge = run.prepare_weapon(world, run.b())?;
            run.next(vec![
                Effect::Intersect(run.a()),
                Effect::Intersect(run.b()),
                charge,
                Effect::Weapon(run.b()),
            ])
        }
        1 => {
            run.object = Some(run.weapon(world, run.b())?);
            run.array = Some(world.elements[&run.object()].current.image.frames);
            run.owner = Some(world.owner(run.b()));
            run.species = world.ship(run.b()).pick.species;
            let mut effects = Vec::new();
            for (&h, e) in &mut world.elements {
                if e.current.image.frames == run.array() {
                    place(e, 7000, 1000);
                    effects.push(Effect::Intersect(h));
                }
            }
            run.proof.steps += 1;
            effects.push(Effect::Redraw(true));
            run.next(effects)
        }
        2 => {
            alive_projectile(run, world)?;
            let crew = world
                .ship(run.b())
                .info
                .as_ref()
                .ok_or("fixture requires loaded descriptor fields")?
                .crew_level;
            run.next(vec![
                Effect::Damage(run.b(), crew as i16),
                Effect::Invoke(hooks.ship_death, run.b()),
            ])
        }
        _ => unreachable!(),
    }
}

fn replacement(run: &mut Battle, world: &mut World) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        3 => {
            run.proof.require(
                world.elements[&run.object()].life > 0
                    && world
                        .ship(run.b())
                        .info
                        .as_ref()
                        .ok_or("fixture requires loaded descriptor fields")?
                        .crew_level
                        == 0,
                "orphan-weapon",
            )?;
            run.next(vec![
                Effect::RetainDescriptor(run.owner()),
                Effect::Remove(vec![run.b()]),
            ])
        }
        4 => {
            let ship = world
                .ships
                .get_mut(&run.owner())
                .ok_or("fixture element or owner is no longer live")?;
            ship.element = None;
            ship.pick.species = run.species;
            run.next(vec![Effect::Spawn(run.owner())])
        }
        5 => {
            let ship = &world.ships[&run.owner()];
            run.replacement = Some(
                ship.element
                    .unwrap_or_else(|| panic!("spawned ship element")),
            );
            // Different addresses of the same descriptor field prove distinct
            // descriptors while the outgoing descriptor is still owned.
            run.proof.require(
                world.elements[&run.object()].current.image.frames == run.array()
                    && run.array()
                        != ship
                            .data
                            .as_ref()
                            .ok_or("fixture requires loaded descriptor fields")?
                            .weapon,
                "same-species-entry",
            )?;
            place(
                world
                    .elements
                    .get_mut(&run.replacement())
                    .ok_or("fixture element or owner is no longer live")?,
                5000,
                5000,
            );
            run.next(vec![Effect::Intersect(run.replacement())])
        }
        _ => unreachable!(),
    }
}

fn alive_projectile(run: &Battle, world: &World) -> Result<(), String> {
    if world
        .elements
        .get(&run.object())
        .is_none_or(|e| e.life == 0)
    {
        Err("frame-set projectile did not survive its production traversal".into())
    } else {
        Ok(())
    }
}

fn threat(run: &mut Battle, world: &mut World) -> Result<Option<Vec<Effect>>, String> {
    alive_projectile(run, world)?;
    if !arrived(&world.elements[&run.replacement()]) {
        if run.wait == 96 {
            return Err("same-species production arrival did not complete".into());
        }
        run.wait += 1;
        return run.traverse();
    }
    let mut concerns = [Concern {
        object: None,
        facing: 0,
        which_turn: 0,
        movement: 0,
    }; 4];
    concerns[ENEMY_SHIP_INDEX as usize].object = Some(run.replacement());
    concerns[ENEMY_SHIP_INDEX as usize].movement = MOVEMENT_STATE_ENTICE;
    concerns[ENEMY_WEAPON_INDEX as usize].object = Some(run.object());
    concerns[ENEMY_WEAPON_INDEX as usize].which_turn = 1;
    run.wait = 0;
    run.next(vec![Effect::Intelligence(
        world.ship(run.a()).intelligence,
        run.a(),
        concerns,
    )])
}

fn collision(run: &mut Battle, world: &mut World) -> Result<Option<Vec<Effect>>, String> {
    run.proof.require(
        world.elements[&run.object()].next.image.frames == run.array(),
        "arilou-threat",
    )?;
    world.ship_mut(run.a()).state.input = 0;
    run.before = world.elements.keys().copied().collect();
    run.next(vec![Effect::Collide(run.object(), run.a())])
}

fn blast(run: &mut Battle, world: &World) -> Result<Option<Vec<Effect>>, String> {
    let blast = world
        .order
        .iter()
        .find(|h| !run.before.contains(h))
        .ok_or("production collision did not create blast")?;
    if world.elements[blast].current.image.frames != run.array() {
        return Err("Human missile blast did not retain its weapon frames".into());
    }
    run.next(vec![Effect::Remove(vec![run.object()])])
}

fn retire(run: &mut Battle, world: &World) -> Result<Option<Vec<Effect>>, String> {
    let retained = world
        .elements
        .values()
        .any(|e| e.current.image.frames == run.array() || e.next.image.frames == run.array());
    if retained && run.wait < 96 {
        run.wait += 1;
        return run.traverse();
    }
    run.proof.require(!retained, "blast-teardown")?;
    run.wait = 0;
    run.next(vec![Effect::FreeRetained, Effect::Scale(3)])
}

fn zoom(run: &mut Battle, world: &mut World) -> Result<Option<Vec<Effect>>, String> {
    if run.wait < 3 {
        let view = usize::from(run.wait);
        let distance = [160, 1400, 3000][view];
        let ship = world.ship_mut(run.a());
        ship.state.input = 0;
        let index = ship.state.facing;
        let frames = ship
            .data
            .as_ref()
            .ok_or("fixture requires loaded descriptor fields")?
            .ship;
        place(
            world
                .elements
                .get_mut(&run.a())
                .ok_or("fixture element or owner is no longer live")?,
            1000,
            1000,
        );
        place(
            world
                .elements
                .get_mut(&run.replacement())
                .ok_or("fixture element or owner is no longer live")?,
            1000 + distance,
            1000,
        );
        run.wait += 1;
        run.proof.steps += 1;
        return Ok(Some(vec![
            Effect::Intersect(run.a()),
            Effect::Intersect(run.replacement()),
            Effect::Redraw(true),
            Effect::ZoomProof(frames, run.a(), view, index),
        ]));
    }
    if run.wait == 3 {
        run.wait += 1;
        run.proof.steps += 1;
        return Ok(Some(vec![Effect::RestoreScale, Effect::Redraw(true)]));
    }
    Ok(None)
}
