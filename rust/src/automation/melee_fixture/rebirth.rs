//! Phoenix decisions use the actual element and copied per-instance hook state.

use super::*;

pub(super) fn advance(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    let a = run.a;
    match run.stage {
        0 => install(run, world, hooks),
        1 => {
            world
                .elements
                .get_mut(&a)
                .ok_or("fixture element or owner is no longer live")?
                .clear_appearing();
            run.proof
                .require(phoenix_exists(world, hooks.intercept), "phoenix-installed")?;
            let phoenix = world
                .ship(a)
                .phoenix
                .ok_or("Pkunk instance state is unavailable")?
                .handle;
            assert!(!phoenix.is_null(), "fixture Phoenix");
            let element = world
                .elements
                .get_mut(&phoenix)
                .ok_or("fixture element or owner is no longer live")?;
            assert_eq!(
                element.death_func.map_or(0, |f| f.address()),
                hooks.intercept
            );
            element.life_span = 0;
            let crew = world
                .ship(a)
                .info
                .as_ref()
                .ok_or("fixture requires loaded descriptor fields")?
                .crew_level;
            run.next(vec![
                Effect::PhoenixFirst(phoenix),
                Effect::Damage(a, crew as i16),
            ])
        }
        2 => {
            if world.elements[&a].death_func.map_or(0, |f| f.address()) == hooks.rebirth
                && world.elements[&a].life_span == 0
            {
                run.phoenix = world.ship(a).phoenix;
                run.next(vec![Effect::Death(a)])
            } else {
                if run.wait == 256 {
                    return Err("pkunk-rebirth: live assertion failed: reborn".into());
                }
                run.wait += 1;
                run.traverse()
            }
        }
        3 => {
            restore(
                &mut run.proof,
                world.elements[&a],
                world.ship(a),
                run.phoenix.ok_or("Phoenix saved hooks were not captured")?,
            )?;
            run.proof.require(true, "reborn")?;
            run.wait = 0;
            run.next(vec![])
        }
        _ => run.finish(24),
    }
}

fn install(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    let a = run.a;
    let exists = phoenix_exists(world, hooks.intercept);
    if !exists {
        crate::math::seed_random(1);
        world
            .elements
            .get_mut(&a)
            .ok_or("fixture element or owner is no longer live")?
            .set_appearing();
    }
    run.next(if exists {
        vec![]
    } else {
        vec![Effect::Invoke(hooks.pkunk, a)]
    })
}

fn phoenix_exists(world: &World<'_>, intercept: usize) -> bool {
    world
        .elements
        .values()
        .any(|e| e.death_func.map_or(0, |f| f.address()) == intercept)
}

fn restore(
    proof: &mut FixtureRun,
    element: &Element,
    ship: &Ship<'_>,
    saved: Phoenix,
) -> Result<(), String> {
    let info = ship
        .info
        .as_ref()
        .ok_or("fixture requires loaded descriptor fields")?;
    proof.require(
        info.crew_level == info.max_crew && element.h_target.cast() == ship.state.hShip,
        "target-hack",
    )?;
    proof.require(
        [
            element.preprocess_func.map_or(0, |f| f.address()),
            element.postprocess_func.map_or(0, |f| f.address()),
            element.death_func.map_or(0, |f| f.address()),
        ] == saved.hooks,
        "hooks-restored",
    )
}
