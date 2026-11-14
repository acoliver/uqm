//! Campaign helpers shared by the planet generation hosts.

use crate::comm::dispatch::{ship, CPoint, ONE_SECOND};
use crate::planets::universe_units::{universe_to_logx, universe_to_logy};

pub use crate::log_abi::log_init;

pub use crate::navigation::groups::live::init_group_info;

pub use crate::navigation::groups::live::uninit_group_info;

pub use crate::planets::orbit::live::{free_planet, load_planet};

/// `GROUP_INIT_IP`, `GROUP_SAVE_IP` (`state.h`): `(BYTE)~0`.
pub(super) const GROUP_SAVE_IP: u8 = !0;
/// `GROUP_LOAD_IP` is `GROUP_LIST` (`state.h`).
pub(super) const GROUP_LOAD_IP: u8 = 0;

/// Clear the guard encounter so the interplanetary groups do not chase.
pub fn reset_encounter(world: &mut crate::engine::World) {
    world.encounter_group = 0;
    world.encounter_race = -1;
}

/// The remaining legacy escort queue's feasibility check for rebel Yehat.
///
/// # Safety
/// Game thread, with the available-race and escort queues initialised.
pub fn rebel_escort_feasible(world: &crate::engine::World) -> bool {
    {
        crate::engine::world::fleet::escort_feasibility(world, u16::from(ship::YEHAT_REBEL_SHIP))
            != 0
    }
}

/// `GROUPS_RANDOM`.
pub(super) const GROUPS_RANDOM: u32 = 0;

/// `GROUPS_ADD_NEW` (`state.h`).
pub(super) const GROUPS_ADD_NEW: u32 = !0;
/// `SYREEN_CONVERSATION` and `SLYLANDRO_HOME_CONVERSATION` (`commglue.h`).
pub const SYREEN_CONVERSATION: u32 = 20;
pub const SLYLANDRO_HOME_CONVERSATION: u32 = 23;

/// `YEHAT_CONVERSATION` (`commglue.h`) and `YEHAT_SHIP` (`races.h`).
pub const YEHAT_CONVERSATION: u32 = 11;
pub const YEHAT_SHIP: u16 = 11;
/// `INFINITE_FLEET` (`races.h`): `(COUNT)~0`.
const INFINITE_FLEET: u16 = !0;

/// `StartSphereTracking (race)` as the truth value the generators test.
///
/// # Safety
/// Game thread, with the fleet queue initialised.
pub fn start_sphere_tracking(world: &mut crate::engine::World, race: u16) -> bool {
    crate::engine::world::fleet::start_sphere_tracking(world, race) != 0
}

fn clone_npc_ship(
    world: &mut crate::engine::World,
    ship: u8,
    crew: u16,
    captain: Option<u8>,
) -> Option<crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Fragment>> {
    {
        let fleets = world.avail_races_ref();
        let template = *fleets.get(fleets.at(usize::from(ship))?);
        let mut ships = world.npc_built_ships();
        let id = ships.clone_fragment(
            &template,
            ship,
            crew,
            crate::comm::dispatch::production_captain_name,
        )?;
        if let Some(captain) = captain {
            ships.get_mut(id).captains_name_index = captain;
        }
        Some(id)
    }
}

/// `CloneShipFragment (ship, &GLOBAL (npc_built_ship_q), INFINITE_FLEET)`.
pub fn clone_infinite_ship_into_built_queue(world: &mut crate::engine::World, ship: u16) {
    clone_npc_ship(
        world,
        u8::try_from(ship).expect("ship index fits a byte"),
        INFINITE_FLEET,
        None,
    );
}

/// `CountLinks (&GLOBAL (npc_built_ship_q))`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub fn built_ship_count(world: &crate::engine::World) -> u16 {
    world.npc_built_ships_ref().count() as u16
}

/// `GetHeadLink (&GLOBAL (npc_built_ship_q)) != 0`.
///
/// # Safety
/// Game thread; `GlobData` is initialised.
pub fn built_ship_queue_has_ships(world: &crate::engine::World) -> bool {
    world.npc_built_ships_ref().head().is_some()
}

/// `ONE_SHOT_ENCOUNTER` (`encount.h`).
const ONE_SHOT_ENCOUNTER: u8 = 1 << 7;

/// The one-shot Ur-Quan encounter `ZapToUrquanEncounter` builds at the
/// universe point `loc` from the `URQUAN_SHIP` fleet template, with the SIS
/// logical position set to match. False when there is no encounter to use.
pub fn begin_urquan_encounter(world: &mut crate::engine::World, loc: (i16, i16)) -> bool {
    {
        let mut encounters = world.encounters();
        let Some(id) = encounters.alloc().or_else(|| encounters.head()) else {
            return false;
        };
        if Some(id) == encounters.head() {
            encounters.remove(id);
        }
        *encounters.get_mut(id) = crate::engine::world::queues::EncounterRecord::default();
        encounters.insert_before(id, encounters.head());
        let fleets = world.avail_races_ref();
        let template = *fleets.get(
            fleets
                .at(usize::from(ship::URQUAN_SHIP))
                .expect("Ur-Quan fleet"),
        );
        let mut encounters = world.encounters();
        let encounter = encounters.get_mut(id);
        encounter.origin = template.loc;
        encounter.radius = template.actual_strength;
        encounter.race_id = ship::URQUAN_SHIP;
        encounter.num_ships = 1;
        encounter.flags = ONE_SHOT_ENCOUNTER;
        encounter.ship_list[0] = crate::engine::world::queues::BriefShipInfo {
            race_id: ship::URQUAN_SHIP,
            crew_level: template.crew_level,
            max_crew: template.max_crew,
            max_energy: template.max_energy,
        };
        encounter.loc_pt = CPoint { x: loc.0, y: loc.1 };
        encounter.log_x = universe_to_logx(loc.0);
        encounter.log_y = universe_to_logy(loc.1);
        let position = (encounter.log_x, encounter.log_y);
        world.data.flagship.log_x = position.0;
        world.data.flagship.log_y = position.1;
        true
    }
}

/// Fade to black while the supplied graphics owner admits presentation and pacing.
pub fn fade_to_black_for_two_seconds(context: crate::engine::graphics::context_abi::ContextLease) {
    let interval = i16::try_from(ONE_SECOND * 2)
        .unwrap_or_else(|_| panic!("two seconds must fit the fade interval"));
    let deadline = context.fade_screen(crate::graphics::menu::ScreenFade::Black as u32, interval);
    crate::engine::threading::pacing::GameThreadPacer::new(context).sleep_until(deadline);
}
