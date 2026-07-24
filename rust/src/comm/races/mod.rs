//! Per-race dialogue state machines, ported from C to Rust.
//!
//! Each race's dialogue is a state machine that:
//! 1. Calls `NPCPhrase(index)` to speak alien dialogue lines
//! 2. Offers `Response(phrase, callback)` choices to the player
//! 3. Reads/writes game state via `GET_GAME_STATE`/`SET_GAME_STATE`
//! 4. Uses `setSegue`/`getSegue` to control encounter flow
//!
//! All of these primitives are now Rust-owned:
//! - `NPCPhrase` → `rust_NPCPhrase_cb` (FFI to Rust comm system)
//! - `Response` → `rust_DoResponsePhrase` (FFI to Rust response system)
//! - `GET_GAME_STATE`/`SET_GAME_STATE` → Rust game state singleton (P09)
//! - `setSegue`/`getSegue` → Rust `comm::state::CommState`
//! - `PHRASE_ENABLED`/`DISABLE_PHRASE` → `rust_PhraseEnabled`/`rust_DisablePhrase`
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P12-P15

use std::os::raw::c_int;

use super::types::CommData;

pub mod arilou;
pub mod blackur;
pub mod chmmr;
pub mod comandr;
pub mod druuge;
pub mod ilwrath;
pub mod melnorm;
pub mod mycon;
pub mod orz;
pub mod pkunk;
pub mod rebel;
pub mod shofixt;
pub mod slyhome;
pub mod slyland;
pub mod spahome;
pub mod spathi;
pub mod starbas;
pub mod supox;
pub mod syreen;
pub mod talkpet;
pub mod thradd;
pub mod umgah;
pub mod urquan;
pub mod utwig;
pub mod vux;
pub mod yehat;
pub mod zoqfot;

/// Trait implemented by each race's dialogue module.
///
/// Each race provides:
/// - `init()`: Populate CommData with resource keys, animation descriptors, and
///   set the initial segue mode based on game state
/// - `intro()`: The initial dialogue entry point (called as `init_encounter_func`)
/// - `post_encounter()`: Post-encounter processing (called as `post_encounter_func`)
/// - `uninit()`: Cleanup after encounter (called as `uninit_encounter_func`)
pub trait RaceDialogue: Send + Sync {
    /// Populate CommData for this race and set the initial segue.
    fn init(&self) -> CommData;

    /// Initial dialogue entry point. Called when the encounter starts.
    fn intro(&self);

    /// Post-encounter processing. Called when the encounter ends normally.
    fn post_encounter(&self);

    /// Cleanup. Called after the encounter ends.
    fn uninit(&self) -> u32;
}

// Conversation IDs from commglue.h enum
const ARILOU_CONVERSATION: i32 = 0;
const CHMMR_CONVERSATION: i32 = 1;
const COMMANDER_CONVERSATION: i32 = 2;
const ORZ_CONVERSATION: i32 = 3;
const PKUNK_CONVERSATION: i32 = 4;
const SHOFIXTI_CONVERSATION: i32 = 5;
const SPATHI_CONVERSATION: i32 = 6;
const SUPOX_CONVERSATION: i32 = 7;
const THRADD_CONVERSATION: i32 = 8;
const UTWIG_CONVERSATION: i32 = 9;
const VUX_CONVERSATION: i32 = 10;
const YEHAT_CONVERSATION: i32 = 11;
const MELNORME_CONVERSATION: i32 = 12;
const DRUUGE_CONVERSATION: i32 = 13;
const ILWRATH_CONVERSATION: i32 = 14;
const MYCON_CONVERSATION: i32 = 15;
const SLYLANDRO_CONVERSATION: i32 = 16;
const UMGAH_CONVERSATION: i32 = 17;
const URQUAN_CONVERSATION: i32 = 18;
const ZOQFOTPIK_CONVERSATION: i32 = 19;
const SYREEN_CONVERSATION: i32 = 20;
const BLACKURQ_CONVERSATION: i32 = 21;
const TALKING_PET_CONVERSATION: i32 = 22;
const SLYLANDRO_HOME_CONVERSATION: i32 = 23;
const YEHAT_REBEL_CONVERSATION: i32 = 25;

/// Get the race dialogue implementation for a conversation ID.
///
/// Returns `None` for conversation IDs that haven't been ported yet.
/// Only Arilou has a full implementation — others have stubs with
/// resource keys and string indices but TODO state machines.
#[must_use]
pub fn get_race_dialogue(comm_id: i32) -> Option<Box<dyn RaceDialogue>> {
    match comm_id {
        ARILOU_CONVERSATION => Some(Box::new(arilou::ArilouDialogue)),
        CHMMR_CONVERSATION => Some(Box::new(chmmr::ChmmrDialogue)),
        COMMANDER_CONVERSATION => Some(Box::new(comandr::ComandrDialogue)),
        ORZ_CONVERSATION => Some(Box::new(orz::OrzDialogue)),
        PKUNK_CONVERSATION => Some(Box::new(pkunk::PkunkDialogue)),
        SHOFIXTI_CONVERSATION => Some(Box::new(shofixt::ShofixtDialogue)),
        SPATHI_CONVERSATION => Some(Box::new(spathi::SpathiDialogue)),
        SUPOX_CONVERSATION => Some(Box::new(supox::SupoxDialogue)),
        THRADD_CONVERSATION => Some(Box::new(thradd::ThraddDialogue)),
        UTWIG_CONVERSATION => Some(Box::new(utwig::UtwigDialogue)),
        VUX_CONVERSATION => Some(Box::new(vux::VuxDialogue)),
        YEHAT_CONVERSATION => Some(Box::new(yehat::YehatDialogue)),
        MELNORME_CONVERSATION => Some(Box::new(melnorm::MelnormDialogue)),
        DRUUGE_CONVERSATION => Some(Box::new(druuge::DruugeDialogue)),
        ILWRATH_CONVERSATION => Some(Box::new(ilwrath::IlwrathDialogue)),
        MYCON_CONVERSATION => Some(Box::new(mycon::MyconDialogue)),
        SLYLANDRO_CONVERSATION => Some(Box::new(slyland::SlylandDialogue)),
        UMGAH_CONVERSATION => Some(Box::new(umgah::UmgahDialogue)),
        URQUAN_CONVERSATION => Some(Box::new(urquan::UrquanDialogue)),
        ZOQFOTPIK_CONVERSATION => Some(Box::new(zoqfot::ZoqfotDialogue)),
        SYREEN_CONVERSATION => Some(Box::new(syreen::SyreenDialogue)),
        BLACKURQ_CONVERSATION => Some(Box::new(blackur::BlackurDialogue)),
        TALKING_PET_CONVERSATION => Some(Box::new(talkpet::TalkpetDialogue)),
        SLYLANDRO_HOME_CONVERSATION => Some(Box::new(slyhome::SlyhomeDialogue)),
        YEHAT_REBEL_CONVERSATION => Some(Box::new(rebel::RebelDialogue)),
        _ => None,
    }
}

/// FFI entry point: initialize a race dialogue from Rust.
///
/// Called from C's `init_race()` in `commglue.c`. If this returns 1,
/// the race has a Rust dialogue implementation and the Rust CommData
/// has been populated with resource keys and segue mode. If it returns 0,
/// the C `init_*_comm()` function should be used instead.
///
/// # Safety
///
/// This is an FFI function called from C.
#[no_mangle]
pub unsafe extern "C" fn rust_init_race_dialogue(comm_id: c_int) -> c_int {
    match get_race_dialogue(comm_id) {
        Some(dialogue) => {
            let data = dialogue.init();
            // Sync to the global CommData singleton
            crate::comm::locdata::set_comm_data(data);
            1
        }
        None => 0,
    }
}
