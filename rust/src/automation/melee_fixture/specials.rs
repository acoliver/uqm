//! Focused SPECIAL fixtures. The special ship is human-controlled and holds the
//! SPECIAL input in `state.input` across real battle frames, so the race's own
//! SPECIAL code decides whether and when the special activates. The opponent is
//! human-controlled with no input, so it never fires.

use super::*;
use crate::battle::ai::HUMAN_CONTROL;
use crate::engine::battle::{record::CHARACTERISTIC_STUFF, views::Info};

/// Frames a held SPECIAL may take to activate before the fixture fails.
const HOLD_FRAMES: u16 = 8;
/// Distance from the harvesting ship to the junk, inside the race's harvest range.
const JUNK_OFFSET: i32 = 40;
const DESCRIPTOR: &str = "fixture requires loaded descriptor fields";

pub(super) fn furnace(run: &mut Battle, world: &mut World) -> Result<Option<Vec<Effect>>, String> {
    let a = run.a();
    match run.stage {
        0 => {
            run.crew = info(world, a)?.crew_level;
            if run.crew < 2 {
                return Err("druuge-furnace requires a Druuge with at least two crew".into());
            }
            prepare(run, world);
            info_mut(world, a)?.energy_level = 0;
            run.next(vec![])
        }
        1 => {
            if world.ship(a).state.special_counter == 0 {
                return hold(run, world, "druuge-furnace");
            }
            let armed = characteristics(world, a)?.special_wait;
            let cost = characteristics(world, a)?.special_energy_cost;
            let stats = info(world, a)?;
            let (crew, energy) = (stats.crew_level, stats.energy_level);
            run.proof.require(
                armed > 0 && (1..=armed).contains(&world.ship(a).state.special_counter),
                "counter-armed",
            )?;
            run.proof.require(crew + 1 == run.crew, "crew-burned")?;
            run.proof.require(energy >= cost, "energy-restored")?;
            release(run, world);
            run.next(vec![])
        }
        _ => run.finish(8),
    }
}

pub(super) fn harvest(run: &mut Battle, world: &mut World) -> Result<Option<Vec<Effect>>, String> {
    let a = run.a();
    match run.stage {
        0 => {
            let ship_at = world.elements[&a].current.location;
            let junk = world
                .elements
                .iter()
                .find(|(id, e)| {
                    **id != a
                        && e.player == NEUTRAL_PLAYER_NUM as i16
                        && !e.has_finite_life()
                        && arrived(e)
                        && u32::from(e.mass) <= MAX_SHIP_MASS * 10
                })
                .map(|(id, _)| *id)
                .ok_or("slylandro-harvest requires an arrived neutral space junk object")?;
            let x = i32::from(ship_at.x) + (JUNK_OFFSET << ONE_SHIFT);
            let x = i16::try_from(x).map_err(|_| "junk placement leaves the arena")?;
            place(
                world
                    .elements
                    .get_mut(&junk)
                    .ok_or("fixture junk element is no longer live")?,
                x,
                ship_at.y,
            );
            run.object = Some(junk);
            prepare(run, world);
            info_mut(world, a)?.energy_level = 0;
            run.next(vec![])
        }
        1 => {
            if world.ship(a).state.special_counter == 0 {
                return hold(run, world, "slylandro-harvest");
            }
            let armed = characteristics(world, a)?.special_wait;
            let stats = info(world, a)?;
            let (energy, max_energy) = (stats.energy_level, stats.max_energy);
            let junk = run.object();
            run.proof.require(
                armed > 0 && (1..=armed).contains(&world.ship(a).state.special_counter),
                "counter-armed",
            )?;
            run.proof.require(
                world.elements.get(&junk).is_some_and(|e| e.life == 0),
                "junk-harvested",
            )?;
            run.proof.require(energy >= max_energy, "energy-restored")?;
            release(run, world);
            run.next(vec![])
        }
        _ => run.finish(8),
    }
}

fn info(world: &World, ship: ElementId) -> Result<&Info, String> {
    world
        .ship(ship)
        .info
        .as_ref()
        .ok_or_else(|| DESCRIPTOR.into())
}

fn info_mut(world: &mut World, ship: ElementId) -> Result<&mut Info, String> {
    world
        .ship_mut(ship)
        .info
        .as_mut()
        .ok_or_else(|| DESCRIPTOR.into())
}

fn characteristics(world: &World, ship: ElementId) -> Result<&CHARACTERISTIC_STUFF, String> {
    world
        .ship(ship)
        .characteristics
        .as_ref()
        .ok_or_else(|| DESCRIPTOR.into())
}

/// Human control for the special ship and the passive opponent, so no cyborg
/// intelligence overwrites the scripted input.
fn prepare(run: &Battle, world: &mut World) {
    for id in [run.a(), run.b()] {
        world.ship_mut(id).state.control = HUMAN_CONTROL;
    }
    world.ship_mut(run.b()).state.input = 0;
}

/// One held frame: SPECIAL stays in the scripted input until the race arms its counter.
fn hold(
    run: &mut Battle,
    world: &mut World,
    name: &'static str,
) -> Result<Option<Vec<Effect>>, String> {
    if run.wait == HOLD_FRAMES {
        return Err(format!(
            "{name}: live assertion failed: special did not fire"
        ));
    }
    run.wait += 1;
    world.ship_mut(run.a()).state.input = SPECIAL as u8;
    run.traverse()
}

/// Ends the held input and resets the frame counter for the settling frames.
fn release(run: &mut Battle, world: &mut World) {
    world.ship_mut(run.a()).state.input = 0;
    run.wait = 0;
}
