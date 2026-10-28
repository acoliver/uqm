//! Input-driven contact decisions and exact first-save persistence receipts.

use super::*;

pub(super) fn witness(
    proof: &mut FixtureRun,
    position: SavedPosition,
    loading: bool,
) -> Result<(), String> {
    if proof.fixture != FixtureName::StarContact {
        return Ok(());
    }
    log::info!("star-contact persistence witness: loading={loading} position={position:?}");
    if loading {
        proof.require(proof.star.saved == Some(position), "save-reload").map_err(|error| {
            format!("{error}: expected={:?}, actual={position:?}; fields: slot={} log_x={} log_y={} star={:?}",
                proof.star.saved, position.slot, position.log_x, position.log_y, position.star)
        })?;
        proof.star.loaded = true;
    } else {
        if proof.star.phase < 3 {
            return Err("star-contact save preceded system return".into());
        }
        // The picker also saves another slot before loading the first save.
        proof.star.saved.get_or_insert(position);
    }
    Ok(())
}

pub(super) fn advance(
    run: &mut Battle,
    world: &mut World<'_>,
    hooks: &Hooks,
) -> Result<Option<Vec<Effect>>, String> {
    match run.stage {
        0 => {
            match run.proof.star.phase {
                0 if world.activity as u8 == IN_INTERPLANETARY as u8 => run.proof.star.phase = 1,
                1 if world.activity as u8 == IN_HYPERSPACE as u8
                    && world.activity & IN_BATTLE as u16 != 0 =>
                {
                    let ship = world.order.iter().copied().find(|h| {
                        let e = &world.elements[h];
                        e.is_player_ship()
                            && !e
                                .state_flags
                                .intersects(ElementFlags::APPEARING | ElementFlags::NONSOLID)
                    });
                    let source = world.order.iter().copied().find(|h| {
                        world.elements[h].collision_func.map_or(0, |f| f.address())
                            == hooks.star_collision
                    });
                    if let (Some(ship), Some(source)) = (ship, source) {
                        run.a = ship;
                        run.b = source;
                        return contact(run, world);
                    }
                }
                2 if world.activity as u8 == IN_INTERPLANETARY as u8 => run.proof.star.phase = 3,
                3 if run.proof.star.loaded && world.activity as u8 == IN_HYPERSPACE as u8 => {
                    run.proof.star.phase = 4
                }
                _ => {}
            }
            Ok(None)
        }
        1 => {
            run.proof.require(
                world.elements[&run.a]
                    .preprocess_func
                    .map_or(0, |f| f.address())
                    == hooks.unhyper,
                "star-contact",
            )?;
            run.star = world.star;
            run.velocity = world.elements[&run.a].velocity;
            run.next(vec![Effect::Collide(run.b, run.a)])
        }
        _ => {
            run.proof.require(
                world.star == run.star
                    && world.elements[&run.b]
                        .state_flags
                        .contains(ElementFlags::DEFY_PHYSICS)
                    && world.elements[&run.a].velocity.vector == run.velocity.vector,
                "repeat-suppressed",
            )?;
            run.proof.star.phase = 2;
            run.proof.steps += 2;
            checkpoint();
            Ok(None)
        }
    }
}

fn contact(run: &mut Battle, world: &mut World<'_>) -> Result<Option<Vec<Effect>>, String> {
    run.proof.require(true, "system-exit")?;
    let location = world.elements[&run.b].current.location;
    let ship = world
        .elements
        .get_mut(&run.a)
        .ok_or("fixture element or owner is no longer live")?;
    place(ship, location.x, location.y);
    ship.velocity.set_components(1 << VELOCITY_SHIFT, 0);
    world.ship_mut(run.a).state.cur_status_flags |= THRUST as u16;
    run.next(vec![
        Effect::Intersect(run.a),
        Effect::ClearStar,
        Effect::Collide(run.b, run.a),
    ])
}

pub(super) fn start(activity: u16) -> Result<FixtureRun, String> {
    if activity as u8 != IN_INTERPLANETARY as u8 {
        return Err("star-contact must start in the real solar mode before system exit".into());
    }
    let mut proof = FixtureRun::new(FixtureName::StarContact);
    proof.star.phase = 1;
    Ok(proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_follows_the_first_slot_and_compares_every_position_field() {
        let mut proof = start(IN_INTERPLANETARY as u16).unwrap();
        let original = SavedPosition {
            slot: 0,
            log_x: 27987,
            log_y: 164141,
            star: Some((1752, 1450)),
        };
        assert!(witness(&mut proof, original, false).is_err());
        proof.star.phase = 3;
        witness(&mut proof, original, false).unwrap();
        witness(
            &mut proof,
            SavedPosition {
                slot: 1,
                ..original
            },
            false,
        )
        .unwrap();
        assert_eq!(proof.star.saved, Some(original));
        for altered in [
            SavedPosition {
                slot: 1,
                ..original
            },
            SavedPosition {
                log_x: 27988,
                ..original
            },
            SavedPosition {
                log_y: 164142,
                ..original
            },
            SavedPosition {
                star: None,
                ..original
            },
        ] {
            assert!(witness(&mut proof, altered, true).is_err());
            assert!(!proof.star.loaded);
        }
        witness(&mut proof, original, true).unwrap();
        assert!(proof.star.loaded);
    }
}
