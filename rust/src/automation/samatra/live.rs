//! Read-only boss observations over the game thread's published battle context.

#[cfg(not(test))]
use super::{Events, Snapshot};
#[cfg(not(test))]
use crate::{
    engine::battle::{Cx, ElementId, RaceState, ShipId},
    ships::{
        native::abi::*,
        races::samatra::{engine, observation},
    },
};

#[cfg(not(test))]
pub fn observe() -> (Events, Option<Snapshot>) {
    // SAFETY: the synchronous input service owns the published game-thread context.
    unsafe { crate::battle::context_access::ai_legacy::with_context(snapshot) }
}

#[cfg(not(test))]
fn project_events(state: &crate::engine::battle::race::SaMatraState) -> Events {
    Events {
        generation: state.generation,
        destroyed: state.destroyed,
        gate_entered: state.gate_entered,
    }
}

#[cfg(not(test))]
fn snapshot(cx: &mut Cx<'_>) -> (Events, Option<Snapshot>) {
    let activity = cx.world.activity();
    if activity & 0xff != IN_LAST_BATTLE as u16 {
        return (Events::default(), None);
    }
    if let Some(RaceState::SaMatra(state)) = cx.terminal_outcome() {
        return (project_events(state), None);
    }
    let (width, height) = cx.screen_dimensions();
    let arena = ((width - 64) << (ONE_SHIFT + 3), height << (ONE_SHIFT + 3));
    let mut result = Snapshot::initial(arena);
    let mut events = Events::default();
    let mut boss = false;
    let mut current = cx.elements().head();
    while let Some(id) = current {
        let e = cx.element(id);
        if e.flags & PLAYER_SHIP as u16 != 0 {
            let owner = cx.owner(id);
            if e.player == RPG_PLAYER_NUM as SIZE {
                project_player(cx, id, owner, &mut result);
            } else if let RaceState::SaMatra(state) = cx.race_stats(owner).state {
                boss = true;
                events = project_events(&state);
                result.remaining = cx.race_stats(owner).characteristics.max_thrust as u8;
            }
        }
        if e.death.map(|f| f.address()) == Some(engine::generator_death as *const () as usize)
            && e.flags & FINITE_LIFE as u16 == 0
        {
            let RaceState::SaMatra(state) = cx.race_stats(cx.owner(id)).state else {
                panic!("generator has no boss state")
            };
            let index = state
                .generators
                .iter()
                .position(|generator| *generator == Some(id))
                .unwrap_or_else(|| panic!("generator registered by its owning spawn callback"));
            result.generators.push(observation::Generator {
                id: index as u8,
                x: i32::from(e.current.location.x),
                y: i32::from(e.current.location.y),
                hit_points: e.hits,
            });
        }
        if [
            engine::comet_collision as *const () as usize,
            engine::sentinel_collision as *const () as usize,
        ]
        .contains(&e.collide.map_or(0, |f| f.address()))
            && e.flags & DISAPPEARING as u16 == 0
        {
            let (vx, vy) = e.velocity.preview_next_components(1);
            result.hazards.push(observation::Hazard {
                x: i32::from(e.current.location.x),
                y: i32::from(e.current.location.y),
                velocity: (i32::from(vx as SIZE), i32::from(vy as SIZE)),
            });
        }
        if e.collide.map(|f| f.address()) == Some(engine::gate_collision as *const () as usize) {
            result.gate_open = cx.primitive_style(id).kind == gfx_object_STAMPFILL_PRIM as u8;
            result.gate = (
                i32::from(e.current.location.x),
                i32::from(e.current.location.y),
            );
        }
        current = cx.elements().succ(id);
    }
    let active = activity & (0xff | IN_BATTLE as u16) == IN_LAST_BATTLE as u16 | IN_BATTLE as u16;
    (
        events,
        (active && boss && events.generation != 0).then_some(result),
    )
}

#[cfg(not(test))]
fn project_player(cx: &Cx<'_>, id: ElementId, owner: ShipId, result: &mut Snapshot) {
    let e = cx.element(id);
    let species = cx.pick_ship(owner).species;
    let (vx, vy) = e.velocity.preview_next_components(1);
    result.selected_species = Some(species);
    result.bomb_carrier = species == SPECIES_ID_SIS_SHIP_ID
        && cx.story_state(crate::engine::battle::StoryKey::BombCarrier) != 0;
    result.velocity = (i32::from(vx as SIZE), i32::from(vy as SIZE));
    result.player = Some(crate::automation::battle_fight::ShipState {
        x: i32::from(e.current.location.x),
        y: i32::from(e.current.location.y),
        facing: (cx.ship_state(owner).facing % 16) as u8,
        crew: e.hits,
    });
}
