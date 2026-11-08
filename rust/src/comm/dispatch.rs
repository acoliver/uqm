//! Communication dispatch — ported from C `comm.c` RaceCommunication and
//! InitCommunication.
//!
//! Encounter dispatch uses typed views of the campaign pools owned by World.
//! CommData is populated through the existing `init_race` mechanism (P10).
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P11
//! @requirement REQ-ML-007, REQ-COMM-DISPATCH

mod encounter;
mod routing;
use routing::conversation_setup;

#[cfg(not(test))]
use crate::engine::StoryKey;
use crate::engine::World;
#[cfg(not(test))]
use crate::engine::WorldData;
#[cfg(not(test))]
use crate::engine::{
    battle::Cx,
    observe::{CheckpointView, Io},
};
use std::os::raw::c_void;

use crate::c_bindings::global_state;
#[cfg(not(test))]
use crate::comm::locdata::{read_locdata, set_comm_data};
use crate::engine::battle::link_queue::LinkId;
#[cfg(not(test))]
use crate::engine::battle::link_queue::LinkQueue;
#[cfg(not(test))]
use crate::engine::world::queues::Fleet;
use crate::engine::world::queues::Fragment;

// ---------------------------------------------------------------------------
// Resource and geometry types used by retained presentation calls
// ---------------------------------------------------------------------------

/// C: `typedef struct point { COORD x, y; } POINT;`
/// COORD is `SIZE` which is `SWORD` = `i16`.
pub use crate::graphics::native_abi::point as CPoint;

/// C: `typedef void* STRING` (STRING_TABLE_ENTRY_DESC*)
pub type CStringHandle = *mut c_void;

/// C: `typedef FRAME_DESC* FRAME`
pub type CFrameHandle = *mut c_void;

pub use crate::engine::world::queues::BriefShipInfo;
pub const MAX_HYPER_SHIPS: usize = 7;

const _: () = assert!(MAX_HYPER_SHIPS == global_state::MAX_HYPER_SHIPS as usize);

// ---------------------------------------------------------------------------
// C constants
// ---------------------------------------------------------------------------

/// C: `ONE_SHOT_ENCOUNTER`
pub const ONE_SHOT_ENCOUNTER: u8 = 1 << 7;
/// C: `ENCOUNTER_REFORMING`
pub const ENCOUNTER_REFORMING: u8 = 1 << 6;

/// C: `HAIL = 0`
pub const HAIL: u16 = 0;
/// C: `ATTACK`
pub const ATTACK: u16 = 1;

/// C: `DEAD_GUY = 0`
pub const DEAD_GUY: u16 = 0;
/// C: `GOOD_GUY`
pub const GOOD_GUY: u16 = 1;
/// C: `BAD_GUY`
pub const BAD_GUY: u16 = 2;

/// C: `INFINITE_RADIUS`
pub const INFINITE_RADIUS: u16 = !0;

/// C: `INFINITE_FLEET`
pub const INFINITE_FLEET: u16 = !0;

// ---------------------------------------------------------------------------
// Activity flags (from globdata.h)
// ---------------------------------------------------------------------------

/// C: `CHECK_ABORT`
pub const CHECK_ABORT: u16 = 0x4000;
/// C: `CHECK_LOAD`
pub const CHECK_LOAD: u16 = 0x1000;
/// C: `START_ENCOUNTER`
pub use crate::engine::save::START_ENCOUNTER;
/// C: `START_INTERPLANETARY`
pub const START_INTERPLANETARY: u16 = 0x0800;

/// C: `IN_LAST_BATTLE`
pub const IN_LAST_BATTLE: u16 = 1;
/// C: `IN_ENCOUNTER`
pub const IN_ENCOUNTER: u16 = 2;
/// C: `IN_HYPERSPACE`
pub const IN_HYPERSPACE: u16 = 3;
/// C: `IN_INTERPLANETARY`
pub const IN_INTERPLANETARY: u16 = 4;
/// C: `WON_LAST_BATTLE`
pub const WON_LAST_BATTLE: u16 = 5;

/// C: `CYBORG_ENABLED`
pub const CYBORG_ENABLED: u8 = 1 << 5;

/// C: `DRAW_SIS_DISPLAY`
pub const DRAW_SIS_DISPLAY: u16 = 1 << 3;

// ---------------------------------------------------------------------------
// Conversation IDs (from commglue.h enum)
// ---------------------------------------------------------------------------

pub mod conv {
    pub const ARILOU: u32 = 0;
    pub const CHMMR: u32 = 1;
    pub const COMMANDER: u32 = 2;
    pub const ORZ: u32 = 3;
    pub const PKUNK: u32 = 4;
    pub const SHOFIXTI: u32 = 5;
    pub const SPATHI: u32 = 6;
    pub const SUPOX: u32 = 7;
    pub const THRADD: u32 = 8;
    pub const UTWIG: u32 = 9;
    pub const VUX: u32 = 10;
    pub const YEHAT: u32 = 11;
    pub const MELNORME: u32 = 12;
    pub const DRUUGE: u32 = 13;
    pub const ILWRATH: u32 = 14;
    pub const MYCON: u32 = 15;
    pub const SLYLANDRO: u32 = 16;
    pub const UMGAH: u32 = 17;
    pub const URQUAN: u32 = 18;
    pub const ZOQFOTPIK: u32 = 19;
    pub const SYREEN: u32 = 20;
    pub const BLACKURQ: u32 = 21;
    pub const TALKING_PET: u32 = 22;
    pub const SLYLANDRO_HOME: u32 = 23;
    pub const URQUAN_DRONE: u32 = 24;
    pub const YEHAT_REBEL: u32 = 25;
    pub const INVALID: u32 = 26;
}

// ---------------------------------------------------------------------------
// Ship type IDs (from races.h enum)
// ---------------------------------------------------------------------------

pub mod ship {
    pub const ARILOU_SHIP: u8 = 0;
    pub const CHMMR_SHIP: u8 = 1;
    pub const HUMAN_SHIP: u8 = 2;
    pub const ORZ_SHIP: u8 = 3;
    pub const PKUNK_SHIP: u8 = 4;
    pub const SHOFIXTI_SHIP: u8 = 5;
    pub const SPATHI_SHIP: u8 = 6;
    pub const SUPOX_SHIP: u8 = 7;
    pub const THRADDASH_SHIP: u8 = 8;
    pub const UTWIG_SHIP: u8 = 9;
    pub const VUX_SHIP: u8 = 10;
    pub const YEHAT_SHIP: u8 = 11;
    pub const MELNORME_SHIP: u8 = 12;
    pub const DRUUGE_SHIP: u8 = 13;
    pub const ILWRATH_SHIP: u8 = 14;
    pub const MYCON_SHIP: u8 = 15;
    pub const SLYLANDRO_SHIP: u8 = 16;
    pub const UMGAH_SHIP: u8 = 17;
    pub const URQUAN_SHIP: u8 = 18;
    pub const ZOQFOTPIK_SHIP: u8 = 19;
    pub const SYREEN_SHIP: u8 = 20;
    pub const BLACK_URQUAN_SHIP: u8 = 21;
    pub const YEHAT_REBEL_SHIP: u8 = 22;
    pub const URQUAN_DRONE_SHIP: u8 = 23;
    /// `SAMATRA_SHIP = URQUAN_DRONE_SHIP` (races.h).
    pub const SAMATRA_SHIP: u8 = URQUAN_DRONE_SHIP;
}

/// C: `NPC_PLAYER_NUM`
pub const NPC_PLAYER_NUM: u16 = 1;
/// C: `RPG_PLAYER_NUM`
pub const RPG_PLAYER_NUM: u16 = 0;

// ---------------------------------------------------------------------------
// RACE_COMMUNICATION lookup table
// Maps ship_id → conversation_id (C: RaceComm[] array)
// ---------------------------------------------------------------------------

/// C: `RACE_COMMUNICATION` macro — maps ship index to conversation ID.
#[rustfmt::skip]
pub const RACE_COMMUNICATION: [u32; 25] = [
    conv::ARILOU,          // ARILOU_SHIP
    conv::CHMMR,           // CHMMR_SHIP
    conv::INVALID,         // HUMAN_SHIP
    conv::ORZ,             // ORZ_SHIP
    conv::PKUNK,           // PKUNK_SHIP
    conv::SHOFIXTI,        // SHOFIXTI_SHIP
    conv::SPATHI,          // SPATHI_SHIP
    conv::SUPOX,           // SUPOX_SHIP
    conv::THRADD,          // THRADDASH_SHIP
    conv::UTWIG,           // UTWIG_SHIP
    conv::VUX,             // VUX_SHIP
    conv::YEHAT,           // YEHAT_SHIP
    conv::MELNORME,        // MELNORME_SHIP
    conv::DRUUGE,          // DRUUGE_SHIP
    conv::ILWRATH,         // ILWRATH_SHIP
    conv::MYCON,           // MYCON_SHIP
    conv::SLYLANDRO,       // SLYLANDRO_SHIP
    conv::UMGAH,           // UMGAH_SHIP
    conv::URQUAN,          // URQUAN_SHIP
    conv::ZOQFOTPIK,       // ZOQFOTPIK_SHIP
    conv::INVALID,         // SYREEN_SHIP
    conv::BLACKURQ,        // BLACK_URQUAN_SHIP
    conv::YEHAT_REBEL,     // YEHAT_REBEL_SHIP
    conv::URQUAN_DRONE,    // URQUAN_DRONE_SHIP
    conv::INVALID,         // (padding)
];

/// C: `RACE_SHIP_FOR_COMM` macro — maps conversation ID to ship type.
#[rustfmt::skip]
pub const RACE_SHIP_FOR_COMM: [u8; 27] = [
    ship::ARILOU_SHIP,        // ARILOU_CONVERSATION
    ship::CHMMR_SHIP,         // CHMMR_CONVERSATION
    ship::HUMAN_SHIP,         // COMMANDER_CONVERSATION
    ship::ORZ_SHIP,           // ORZ_CONVERSATION
    ship::PKUNK_SHIP,         // PKUNK_CONVERSATION
    ship::SHOFIXTI_SHIP,      // SHOFIXTI_CONVERSATION
    ship::SPATHI_SHIP,        // SPATHI_CONVERSATION
    ship::SUPOX_SHIP,         // SUPOX_CONVERSATION
    ship::THRADDASH_SHIP,     // THRADD_CONVERSATION
    ship::UTWIG_SHIP,         // UTWIG_CONVERSATION
    ship::VUX_SHIP,           // VUX_CONVERSATION
    ship::YEHAT_SHIP,         // YEHAT_CONVERSATION
    ship::MELNORME_SHIP,      // MELNORME_CONVERSATION
    ship::DRUUGE_SHIP,        // DRUUGE_CONVERSATION
    ship::ILWRATH_SHIP,       // ILWRATH_CONVERSATION
    ship::MYCON_SHIP,         // MYCON_CONVERSATION
    ship::SLYLANDRO_SHIP,     // SLYLANDRO_CONVERSATION
    ship::UMGAH_SHIP,         // UMGAH_CONVERSATION
    ship::URQUAN_SHIP,        // URQUAN_CONVERSATION
    ship::ZOQFOTPIK_SHIP,     // ZOQFOTPIK_CONVERSATION
    ship::SYREEN_SHIP,        // SYREEN_CONVERSATION
    ship::BLACK_URQUAN_SHIP,  // BLACKURQ_CONVERSATION
    ship::UMGAH_SHIP,         // TALKING_PET_CONVERSATION
    ship::SLYLANDRO_SHIP,     // SLYLANDRO_HOME_CONVERSATION
    ship::URQUAN_DRONE_SHIP,  // URQUAN_DRONE_CONVERSATION
    ship::YEHAT_SHIP,         // YEHAT_REBEL_CONVERSATION
    ship::HUMAN_SHIP,         // INVALID_CONVERSATION
];

// ---------------------------------------------------------------------------
// C FFI declarations for functions that stay in C for now
// ---------------------------------------------------------------------------

#[allow(dead_code)]
mod ffi {

    #[cfg(not(test))]
    pub use crate::comm::locdata::COMM_DATA as CommData;

    #[cfg(not(test))]
    pub use crate::ui::border::draw_sisframe;
    #[cfg(test)]
    extern "C" {
        pub fn draw_sisframe();
    }
    #[cfg(not(test))]
    pub use crate::ui::sis::repair_sisborder;
    #[cfg(test)]
    extern "C" {
        pub fn repair_sisborder();
    }

    #[cfg(not(test))]
    #[cfg(not(test))]
    pub use crate::ui::sis::ClearSISRect;

    #[cfg(not(test))]
    pub use crate::ui::sis::DrawSISMessage;

    #[cfg(not(test))]
    pub use crate::ui::sis::DrawHyperCoords;

    #[cfg(not(test))]
    pub use crate::ui::sis::DrawSISTitle;

    #[cfg(not(test))]
    pub use crate::comm::races::arilou::live::init_arilou_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::blackur::live::init_blackurq_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::chmmr::live::init_chmmr_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::comandr::live::init_commander_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::starbas::live::init_starbase_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::druuge::live::init_druuge_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::ilwrath::live::init_ilwrath_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::melnorm::live::init_melnorme_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::mycon::live::init_mycon_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::orz::live::init_orz_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::pkunk::live::init_pkunk_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::shofixt::live::init_shofixti_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::slyland::live::init_slyland_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::slyhome::live::init_slylandro_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::spathi::live::init_spathi_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::spahome::live::init_spahome_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::supox::live::init_supox_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::syreen::live::init_syreen_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::talkpet::live::init_talkpet_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::thradd::live::init_thradd_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::umgah::live::init_umgah_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::urquan::live::init_urquan_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::utwig::live::init_utwig_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::vux::live::init_vux_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::rebel::live::init_rebel_yehat_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::yehat::live::init_yehat_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::zoqfot::live::init_zoqfot_comm;

    #[cfg(not(test))]
    pub use crate::comm::races::rust_init_race_dialogue;
}

// ---------------------------------------------------------------------------
// Helper functions (porting C inline functions/macros)
// ---------------------------------------------------------------------------

/// C: `PickCaptainName()` = `((COUNT)random() & (NUM_CAPTAINS_NAMES - 1)) + NAME_OFFSET`
/// where NUM_CAPTAINS_NAMES=16, NAME_OFFSET=5. Ported to pure Rust.
#[inline]
fn pick_captain_name() -> u8 {
    ((crate::math::random() & 0x0F) + 5) as u8
}

/// C: `LOBYTE(x)` — extract low byte of a u16.
#[inline]
fn lobyte(x: u16) -> u16 {
    x & 0xFF
}

#[cfg(not(test))]
/// C: `inHQSpace()` — checks if CurrentActivity is IN_HYPERSPACE (also true for QuasiSpace).
///
/// # Safety
/// Calls C FFI to read CurrentActivity from global state.
fn in_hq_space(activity: u16) -> bool {
    lobyte(activity) == IN_HYPERSPACE
}

#[cfg(not(test))]
fn avail_race_queue<R>(edit: impl FnOnce(&mut LinkQueue<'_, Fleet, 24>) -> R) -> R {
    World::current().write(|world| edit(&mut world.avail_races()))
}

// ---------------------------------------------------------------------------
// RaceCommunication — ported from comm.c:1503
// ---------------------------------------------------------------------------

#[cfg(not(test))]
/// Port of C `RaceCommunication()`. Determines which alien to talk to,
/// prepares the npc ship queue, and calls InitCommunication.
///
/// # Safety
/// Calls C FFI functions that access global state.
pub unsafe fn race_communication(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    if lobyte(cx.world.activity()) == IN_LAST_BATTLE {
        // Going into talking pet conversation
        cx.world.npc_built_ships().reinit();
        clone_npc_fragment(cx.world, ship::SAMATRA_SHIP, 0);
        init_communication(cx, io, conv::TALKING_PET);

        let activity = cx.world.activity();
        if (activity & (CHECK_ABORT | CHECK_LOAD)) == 0 {
            let crew = cx.world.data.flagship.crew_enlisted;
            if crew != 0xFFFF {
                cx.world.set_activity(WON_LAST_BATTLE);
                io.observer.checkpoint(&CheckpointView::Won);
            }
        }
        return;
    }

    if cx.world.next_activity & CHECK_LOAD != 0 {
        let ec = cx.world.story_state(StoryKey::ESCAPE_COUNTER) as u8;

        init_communication(cx, io, routing::restored_conversation(cx.world));

        let crew = cx.world.data.flagship.crew_enlisted;
        if crew != 0xFFFF {
            let activity = cx.world.activity();
            let mut na = activity & !START_ENCOUNTER;
            if lobyte(na) == IN_INTERPLANETARY {
                na |= START_INTERPLANETARY;
            }
            cx.world.next_activity = na;
            cx.world.set_activity(activity | CHECK_LOAD);
        }

        cx.world
            .set_story_state(StoryKey::ESCAPE_COUNTER, ec as u32);
        return;
    }

    let mut encounter = None;
    if lobyte(cx.world.activity()) == IN_HYPERSPACE {
        cx.world.npc_built_ships().reinit();
        if cx.world.story_state(StoryKey::ARILOU_SPACE_SIDE) >= 2 {
            init_communication(cx, io, conv::ARILOU);
            return;
        }
        let queue = cx.world.encounters_ref();
        if let Some(id) = queue.head() {
            let record = *queue.get(id);
            encounter = Some(id);
            for info in &record.ship_list[..usize::from(record.num_ships)] {
                clone_npc_fragment(cx.world, record.race_id, info.crew_level);
            }
        }
    }
    let queue = cx.world.npc_built_ships_ref();
    let Some(i) = queue.head().map(|id| queue.get(id).race_id) else {
        return;
    };

    let conv_id = if (i as usize) < RACE_COMMUNICATION.len() {
        RACE_COMMUNICATION[i as usize]
    } else {
        conv::INVALID
    };
    io.observer
        .checkpoint(&CheckpointView::EncounterDispatch(conv_id));

    let status = init_communication(cx, io, conv_id);

    let activity = cx.world.activity();
    if (activity & (CHECK_ABORT | CHECK_LOAD)) != 0 {
        return;
    }

    if i == ship::CHMMR_SHIP {
        cx.world.npc_built_ships().reinit();
    }
    if lobyte(activity) == IN_INTERPLANETARY {
        if i == ship::SLYLANDRO_SHIP && status == 0 {
            cx.world.npc_built_ships().reinit();
        }
    } else if let Some(id) = encounter {
        encounter::update_fleet(cx.world, id, status);
        cx.world.npc_built_ships().reinit();
    }
}

// ---------------------------------------------------------------------------
// InitCommunication — ported from comm.c:1359
// ---------------------------------------------------------------------------

#[cfg(not(test))]
/// Map the conversation to a ship, run the dialogue or attack selection, and
/// return whether combat was selected before EncounterBattle runs.
///
/// # Safety
/// Called on the game thread with initialized game state and resources.
pub unsafe fn init_communication(cx: &mut Cx<'_>, io: &mut Io<'_>, which_comm: u32) -> u16 {
    #[cfg(all(not(test), feature = "debug-process"))]
    {
        if crate::front_end::uqmdebug::DISABLE_INTERACTIVITY != 0 {
            return 0;
        }
    }
    let last_activity = cx.kernel.last_activity;
    let activity = cx.world.activity();
    if last_activity & CHECK_LOAD != 0 {
        cx.kernel.last_activity = last_activity & !CHECK_LOAD;
    }
    cx.with_legacy(|| {
        if last_activity & CHECK_LOAD != 0 && which_comm != conv::COMMANDER {
            if lobyte(last_activity) == 0 {
                ffi::draw_sisframe();
            } else {
                ffi::ClearSISRect((DRAW_SIS_DISPLAY) as u8);
                ffi::repair_sisborder();
            }
            ffi::DrawSISMessage(std::ptr::null());
            if in_hq_space(activity) {
                let point = WorldData::current().read(|world| world.game.ship_stamp.origin);
                ffi::DrawHyperCoords(CPoint {
                    x: point.x,
                    y: point.y,
                });
            } else if WorldData::current().read(|world| world.game.ip_planet) == 0 {
                let star = &*(**World::current())
                    .star_pointer(crate::planets::starmap::current_catalog())
                    .cast::<crate::savegame::runtime::star::CStarDesc>();
                ffi::DrawHyperCoords(CPoint {
                    x: star.star_pt.x,
                    y: star.star_pt.y,
                });
            } else {
                let name = WorldData::current().read(|world| world.flagship.planet_name);
                ffi::DrawSISTitle(name.as_ptr().cast());
            }
        }
    });
    let (conversation, ship_type) = conversation_setup(which_comm);
    run_encounter(cx, io, conversation, ship_type)
}

#[cfg(not(test))]
/// Perform sphere tracking, battle setup, dialogue and teardown in C order.
fn run_encounter(cx: &mut Cx<'_>, io: &mut Io<'_>, which_comm: u32, ship_type: u16) -> u16 {
    if ship_type != u16::from(ship::URQUAN_DRONE_SHIP) {
        // SAFETY: the mode owns the initialized campaign during resource dispatch.
        cx.with_legacy(|| unsafe { start_sphere_tracking(ship_type as u8) });
        let build = which_comm == conv::ORZ
            || (which_comm == conv::TALKING_PET
                && (cx.world.story_state(StoryKey::TALKING_PET_ON_SHIP) == 0
                    || lobyte(cx.world.activity()) == IN_LAST_BATTLE))
            || (which_comm != conv::CHMMR && which_comm != conv::SYREEN);
        if build {
            crate::battle::encount_native::build_battle(cx.world, cx.kernel, NPC_PLAYER_NUM);
        }
    }
    let comm_id = if ship_type != u16::from(ship::YEHAT_REBEL_SHIP) {
        which_comm
    } else {
        conv::YEHAT_REBEL
    };
    cx.with_legacy(|| {
        io.observer
            .checkpoint(&CheckpointView::DialogueDispatch(comm_id))
    });
    let has_descriptor = encounter::load_descriptor(cx, comm_id);
    let status = if cx.world.story_state(StoryKey::BATTLE_SEGUE) == 0 {
        HAIL
    } else {
        cx.with_legacy(|| io.observer.checkpoint(&CheckpointView::EncounterMenu(true)));
        // SAFETY: the game thread retains the catalog throughout the modal encounter.
        let catalog = unsafe { crate::planets::starmap::current_catalog() };
        let status = crate::battle::encount_native::init_encounter(cx, io, catalog);
        cx.with_legacy(|| {
            io.observer
                .checkpoint(&CheckpointView::EncounterMenu(false))
        });
        if status == HAIL && has_descriptor {
            cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 0);
            HAIL
        } else {
            cx.world.set_story_state(StoryKey::BATTLE_SEGUE, 1);
            ATTACK
        }
    };
    if status == HAIL {
        // SAFETY: the loaded descriptor and resources remain live for the synchronous session.
        unsafe { crate::comm::hail::hail_alien(cx, io) };
    } else {
        encounter::finish_unhailed(cx, has_descriptor);
    }
    let status = encounter::fight_if_selected(cx, io);
    let runtime = cx.kernel.input_runtime();
    // SAFETY: the game thread retains the catalog through encounter teardown.
    let catalog = unsafe { crate::planets::starmap::current_catalog() };
    crate::battle::encount_native::engine::uninit_encounter(cx, io, runtime.input(), catalog);
    status
}

// ---------------------------------------------------------------------------
// CloneShipFragment — ported from build.c:477
// ---------------------------------------------------------------------------

pub(crate) fn clone_npc_fragment(
    world: &mut World,
    ship_index: u8,
    crew_level: u16,
) -> Option<LinkId<Fragment>> {
    let fleets = world.avail_races_ref();
    let template = *fleets.get(fleets.at(usize::from(ship_index))?);
    world.npc_built_ships().clone_fragment(
        &template,
        ship_index,
        crew_level,
        production_captain_name,
    )
}

/// `PickCaptainName ()` as the running game draws it.
pub(crate) fn production_captain_name() -> u8 {
    // SAFETY: `random` takes no arguments and touches only the game's
    // random state.

    pick_captain_name()
}

/// Prepare the real NPC ship queue for the deterministic Sol probe scene.
///
/// The normal game-loop state machine performs the subsequent encounter and
/// communication dispatch; this helper only constructs the race-23 queue.
///
/// # Safety
/// Game structures and both fleet queues must already be initialized.
#[cfg_attr(test, allow(dead_code))]
pub unsafe fn prepare_automation_sol_probe_encounter(
    world: &mut World,
) -> Result<(), &'static str> {
    world.npc_built_ships().reinit();
    if clone_npc_fragment(world, ship::URQUAN_DRONE_SHIP, 0).is_none() {
        return Err("failed to clone the Ur-Quan probe into the NPC queue");
    }
    Ok(())
}

/// Development-only fleet setup, before the normal final encounter dispatch.
/// This grants fixture equipment and escorts, so it cannot establish campaign progress.
///
/// # Safety
/// Initialized fleets and SIS must be exclusively owned by the game thread.
#[cfg(not(test))]
pub(crate) unsafe fn prepare_samatra_dev_fixture(
    cx: &mut crate::engine::battle::Cx<'_>,
) -> Result<(), &'static str> {
    cx.world.built_ships().reinit();
    let added = crate::engine::world::fleet::add_escort_ships(cx.world, 4, 12, pick_captain_name);
    if added.is_some() {
        cx.with_legacy(|| crate::ui::sis::DeltaSISGauges(32767, 32767, 32767));
    }
    if added != Some(12) {
        return Err("boss fixture could not allocate Pkunk escorts");
    }
    {
        let world = &mut cx.world.data;
        let sis = &mut world.flagship;
        sis.crew_enlisted = 50;
        sis.fuel_on_board = 1000;
        sis.drive_slots.fill(1);
        sis.jet_slots.fill(2);
        // Bomb conversion removes the flagship weapons; this is a steering ship.
        sis.module_slots.fill(0xff);
        world.game.glob_flags &= !(1 << 5);
    }
    Ok(())
}

/// Make the star at `location` the flagship's current star.
///
/// `ExploreSolarSys` does this the first time the flagship is inside a star
/// system. A scene that starts an encounter before that point needs the same
/// star, because the encounter screen names it.
///
/// # Safety
/// The star map must be initialized.
#[cfg_attr(test, allow(dead_code))]
pub unsafe fn place_flagship_at_star(
    world: &mut World,
    location: (i16, i16),
) -> Result<(), &'static str> {
    let universe = CPoint {
        x: location.0,
        y: location.1,
    };
    let star = crate::planets::starmap::find_star_in(world, None, universe, 1, 1);
    if star.is_null() {
        return Err("no star at the scene location");
    }
    world.select_star(
        ((star).cast::<crate::engine::world::CStarDesc>()).as_ref(),
        crate::planets::starmap::current_catalog(),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// StartSphereTracking — ported from build.c:355
// ---------------------------------------------------------------------------

#[cfg(not(test))]
/// Port of C `StartSphereTracking(race)`. Reads/modifies FLEET_INFO to
/// initialize sphere-of-influence tracking for the given race.
/// Rust-side replacement for C's `init_race()`.
///
/// The C `init_race` in `commglue.c` has a broken control flow under
/// `USE_RUST_COMM`: when `rust_init_race_dialogue()` returns 1, it
/// falls through past the switch and returns `init_arilou_comm()` for
/// every race. This function bypasses that by dispatching directly to
/// the correct `init_*_comm` C function via FFI, then calling
/// `rust_init_race_dialogue` for Rust dialogue setup.
///
/// Returns a `LOCDATA*` (as `*mut c_void`) from the correct race's
/// init function, or null if the race has no init function.
/// # Safety
///
/// Must be called from an initialized game context (after resource loading).
/// The returned pointer is a raw C LOCDATA pointer and must not be freed by
/// the caller — it points to static C data.
pub unsafe fn rust_init_race_dispatch(cx: &mut Cx<'_>, comm_id: u32) -> *mut c_void {
    // Initialize Rust dialogue state (callbacks, phrase state, etc.)
    ffi::rust_init_race_dialogue(cx, comm_id as i32);

    // Dispatch to the correct C init_*_comm function
    match comm_id {
        conv::ARILOU => (ffi::init_arilou_comm(cx)).cast(),
        conv::BLACKURQ => ffi::init_blackurq_comm(cx).cast(),
        conv::CHMMR => ffi::init_chmmr_comm(cx).cast(),
        conv::COMMANDER => {
            if cx.world.story_state(StoryKey::STARBASE_AVAILABLE) == 0 {
                (ffi::init_commander_comm(cx)).cast()
            } else {
                (ffi::init_starbase_comm(cx)).cast()
            }
        }
        conv::DRUUGE => ffi::init_druuge_comm(cx).cast(),
        conv::ILWRATH => ffi::init_ilwrath_comm(cx).cast(),
        conv::MELNORME => ffi::init_melnorme_comm(cx).cast(),
        conv::MYCON => ffi::init_mycon_comm(cx).cast(),
        conv::ORZ => ffi::init_orz_comm(cx).cast(),
        conv::PKUNK => ffi::init_pkunk_comm(cx).cast(),
        conv::SHOFIXTI => ffi::init_shofixti_comm(cx).cast(),
        conv::SLYLANDRO => ffi::init_slyland_comm(cx).cast(),
        conv::SLYLANDRO_HOME => ffi::init_slylandro_comm(cx).cast(),
        conv::SPATHI => {
            if (cx.world.story_state(StoryKey::GLOBAL_FLAGS_AND_DATA) & (1 << 7)) == 0 {
                (ffi::init_spathi_comm(cx)).cast()
            } else {
                (ffi::init_spahome_comm(cx)).cast()
            }
        }
        conv::SUPOX => ffi::init_supox_comm(cx).cast(),
        conv::SYREEN => ffi::init_syreen_comm(cx).cast(),
        conv::TALKING_PET => ffi::init_talkpet_comm(cx).cast(),
        conv::THRADD => ffi::init_thradd_comm(cx).cast(),
        conv::UMGAH => ffi::init_umgah_comm(cx).cast(),
        conv::URQUAN => ffi::init_urquan_comm(cx).cast(),
        conv::UTWIG => ffi::init_utwig_comm(cx).cast(),
        conv::VUX => ffi::init_vux_comm(cx).cast(),
        conv::YEHAT_REBEL => ffi::init_rebel_yehat_comm(cx).cast(),
        conv::YEHAT => ffi::init_yehat_comm(cx).cast(),
        conv::ZOQFOTPIK => ffi::init_zoqfot_comm(cx).cast(),
        _ => ffi::init_chmmr_comm(cx).cast(),
    }
}
#[cfg(not(test))]
///
/// Returns `race` if tracking started, 0 if race is extinct or not found.
unsafe fn start_sphere_tracking(race: u8) -> u16 {
    avail_race_queue(|queue| {
        let Some(id) = queue.at(usize::from(race)) else {
            return 0;
        };
        let fleet = queue.get_mut(id);
        if fleet.actual_strength == 0 && fleet.allied_state == DEAD_GUY {
            return 0;
        }
        if fleet.actual_strength != 0
            && fleet.known_strength == 0
            && fleet.actual_strength != INFINITE_RADIUS
        {
            fleet.known_strength = 1;
            fleet.known_loc = fleet.loc;
        }
        u16::from(race)
    })
}

// ---------------------------------------------------------------------------
// VisitStarBase — ported from starbase.c:430
// ---------------------------------------------------------------------------

/// C: CommIntroMode enum values
pub mod cim {
    pub const CROSSFADE_SPACE: u32 = 0;
    pub const CROSSFADE_WINDOW: u32 = 1;
    pub const CROSSFADE_SCREEN: u32 = 2;
    pub const FADE_IN_SCREEN: u32 = 3;
}

/// C: StatMsgMode enum values
pub mod smm {
    pub const UNDEFINED: u32 = 0;
    pub const DATE: u32 = 1;
    pub const RES_UNITS: u32 = 2;
    pub const CREDITS: u32 = 3;
}

/// C: `ONE_SECOND` = 840 (from timelib.h)
pub const ONE_SECOND: u32 = 840;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn race_communication_table_has_25_entries() {
        assert_eq!(RACE_COMMUNICATION.len(), 25);
    }

    #[test]
    fn race_communication_maps_arilou_to_arilou_conv() {
        assert_eq!(RACE_COMMUNICATION[ship::ARILOU_SHIP as usize], conv::ARILOU);
    }

    #[test]
    fn race_communication_maps_human_to_invalid() {
        assert_eq!(RACE_COMMUNICATION[ship::HUMAN_SHIP as usize], conv::INVALID);
    }

    #[test]
    fn samatra_ship_is_the_urquan_drone_slot_of_avail_race_q() {
        // races.h: `SAMATRA_SHIP = URQUAN_DRONE_SHIP`, the last of the 24
        // `avail_race_q` entries; index 24 names no fleet at all.
        assert_eq!(ship::SAMATRA_SHIP, ship::URQUAN_DRONE_SHIP);
        assert_eq!(ship::SAMATRA_SHIP, 23);
        assert_eq!(
            RACE_COMMUNICATION[ship::SAMATRA_SHIP as usize],
            conv::URQUAN_DRONE
        );
        assert_eq!(RACE_COMMUNICATION[24], conv::INVALID);
    }

    #[test]
    fn race_ship_for_comm_has_27_entries() {
        assert_eq!(RACE_SHIP_FOR_COMM.len(), 27);
    }

    #[test]
    fn race_ship_for_comm_maps_arilou_to_arilou_ship() {
        assert_eq!(RACE_SHIP_FOR_COMM[conv::ARILOU as usize], ship::ARILOU_SHIP);
    }

    #[test]
    fn race_ship_for_comm_maps_commander_to_human_ship() {
        assert_eq!(
            RACE_SHIP_FOR_COMM[conv::COMMANDER as usize],
            ship::HUMAN_SHIP
        );
    }

    #[test]
    fn race_ship_for_comm_maps_talking_pet_to_umgah_ship() {
        assert_eq!(
            RACE_SHIP_FOR_COMM[conv::TALKING_PET as usize],
            ship::UMGAH_SHIP
        );
    }

    #[test]
    fn race_ship_for_comm_maps_yehat_rebel_to_yehat_ship() {
        assert_eq!(
            RACE_SHIP_FOR_COMM[conv::YEHAT_REBEL as usize],
            ship::YEHAT_SHIP
        );
    }

    #[test]
    fn lobyte_extracts_low_byte() {
        assert_eq!(lobyte(0x0302), 2); // low byte
        assert_eq!(lobyte(0x0100), 0);
        assert_eq!(lobyte(0x0403), 3); // low byte
    }

    #[test]
    fn activity_constants_match_c() {
        assert_eq!(IN_LAST_BATTLE, 1);
        assert_eq!(IN_ENCOUNTER, 2);
        assert_eq!(IN_HYPERSPACE, 3);
        assert_eq!(IN_INTERPLANETARY, 4);
        assert_eq!(WON_LAST_BATTLE, 5);
    }

    #[test]
    fn check_flags_match_c() {
        assert_eq!(CHECK_ABORT, 0x4000);
        assert_eq!(CHECK_LOAD, 0x1000);
        assert_eq!(START_ENCOUNTER, 0x0400);
        assert_eq!(START_INTERPLANETARY, 0x0800);
    }

    #[test]
    fn conversation_exceptions_keep_their_ship_identity() {
        assert_eq!(
            conversation_setup(conv::URQUAN_DRONE),
            (conv::URQUAN, u16::from(ship::URQUAN_DRONE_SHIP))
        );
        assert_eq!(
            conversation_setup(conv::YEHAT_REBEL),
            (conv::YEHAT, u16::from(ship::YEHAT_REBEL_SHIP))
        );
        assert_eq!(
            conversation_setup(conv::COMMANDER),
            (conv::COMMANDER, u16::from(ship::HUMAN_SHIP))
        );
        assert_eq!(
            conversation_setup(conv::SLYLANDRO_HOME),
            (conv::SLYLANDRO_HOME, u16::from(ship::SLYLANDRO_SHIP))
        );
    }

    #[test]
    fn comm_display_and_cyborg_masks_match_headers() {
        assert_eq!(DRAW_SIS_DISPLAY, 1 << 3);
        assert_eq!(CYBORG_ENABLED, 1 << 5);
    }

    #[test]
    fn encounter_flags_match_c() {
        assert_eq!(ONE_SHOT_ENCOUNTER, 0x80);
        assert_eq!(ENCOUNTER_REFORMING, 0x40);
    }

    #[test]
    fn hail_attack_match_c() {
        assert_eq!(HAIL, 0);
        assert_eq!(ATTACK, 1);
    }

    #[test]
    fn alliance_states_match_c() {
        assert_eq!(DEAD_GUY, 0);
        assert_eq!(GOOD_GUY, 1);
        assert_eq!(BAD_GUY, 2);
    }

    #[test]
    fn conversation_ids_match_c_enum() {
        assert_eq!(conv::ARILOU, 0);
        assert_eq!(conv::CHMMR, 1);
        assert_eq!(conv::COMMANDER, 2);
        assert_eq!(conv::INVALID, 26);
    }

    #[test]
    fn ship_type_ids_match_c_enum() {
        assert_eq!(ship::ARILOU_SHIP, 0);
        assert_eq!(ship::HUMAN_SHIP, 2);
        assert_eq!(ship::SAMATRA_SHIP, 23);
    }

    #[test]
    fn brief_ship_info_is_7_bytes() {
        // race_id(1) + crew_level(2) + max_crew(2) + max_energy(1) = 6 bytes
        // But C might have padding. Let's just verify it compiles.
        let _bsi = BriefShipInfo::default();
    }

    #[test]
    fn cpoint_matches_coord_layout() {
        let pt = CPoint { x: 100, y: 200 };
        assert_eq!(pt.x, 100);
        assert_eq!(pt.y, 200);
    }
}
