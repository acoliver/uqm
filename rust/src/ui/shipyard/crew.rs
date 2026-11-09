//! Campaign crew-purchase accounting.

use crate::engine::world::campaign::*;
use crate::engine::{
    battle::{link_queue::LinkId, Cx},
    world::queues::Fragment,
    StoryKey, World,
};
use crate::graphics::native_abi::*;

pub(super) fn purchased_crew(world: &World) -> SIZE {
    u16::from_le_bytes([
        world.story_state(StoryKey::CREW_PURCHASED0) as BYTE,
        world.story_state(StoryKey::CREW_PURCHASED1) as BYTE,
    ]) as SIZE
}

pub(super) fn crew_transaction(cx: &mut Cx<'_>, delta: SIZE, redraw: impl FnOnce(&mut Cx<'_>)) {
    if delta == 0 {
        return;
    }
    let mut bought = purchased_crew(cx.world).wrapping_add(delta);
    if bought < 0 {
        bought = if delta < 0 { 0 } else { 0x7fff };
    } else if delta > 0 {
        if bought as i32 >= CREW_EXPENSE_THRESHOLD as i32
            && (bought as i32 - delta as i32) < CREW_EXPENSE_THRESHOLD as i32
        {
            cx.world.data.game.crew_cost += 2;
            redraw(cx);
        }
    } else if (bought as i32) < CREW_EXPENSE_THRESHOLD as i32
        && bought as i32 - delta as i32 >= CREW_EXPENSE_THRESHOLD as i32
    {
        cx.world.data.game.crew_cost -= 2;
        redraw(cx);
    }
    if crate::engine::world::fleet::check_alliance(cx.world, SHOFIXTI_SHIP as COUNT)
        != GOOD_GUY as COUNT
    {
        cx.world
            .set_story_state(StoryKey::CREW_PURCHASED0, bought as BYTE as u32);
        cx.world
            .set_story_state(StoryKey::CREW_PURCHASED1, ((bought as u16) >> 8) as u32);
    }
}

pub(super) fn label(world: &mut World, id: LinkId<Fragment>) -> (bool, String) {
    let ship = *world.built_ships().get(id);
    let max_crew = {
        let races = world.avail_races();
        let Some(template) = races.at(usize::from(ship.race_id)) else {
            panic!("a hangar ship must have a fleet template");
        };
        races.get(template).crew_level
    };
    let buffer = if ship.crew_level >= max_crew {
        format!("{}\0", ship.crew_level)
    } else if ship.crew_level == 0 {
        String::from("SCRAP\0")
    } else {
        format!("{}/{}\0", ship.crew_level, max_crew)
    };
    (ship.crew_level != 0, buffer)
}
