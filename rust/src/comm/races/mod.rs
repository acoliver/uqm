//! Per-race dialogue state machines, ported from C to Rust.
//!
//! Each race's dialogue is a state machine that:
//! 1. Calls `NPCPhrase(index)` to speak alien dialogue lines
//! 2. Offers `Response(phrase, callback)` choices to the player
//! 3. Reads/writes game state via `GET_GAME_STATE`/`SET_GAME_STATE`
//! 4. Uses `setSegue`/`getSegue` to control encounter flow
//!
//! All of these primitives are now Rust-owned:
//! - `NPCPhrase` → `rust_npc_phrase_cb` (FFI to Rust comm system)
//! - `Response` → `rust_DoResponsePhrase` (FFI to Rust response system)
//! - `GET_GAME_STATE`/`SET_GAME_STATE` → Rust game state singleton (P09)
//! - `setSegue`/`getSegue` → Rust `comm::state::Comm`
//! - `PHRASE_ENABLED`/`DISABLE_PHRASE` → `rust_phrase_enabled`/`rust_disable_phrase`
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P12-P15

pub use script::{DialogueHost, Supplies};

pub(crate) mod alliance;
pub mod arilou;
pub mod blackur;
pub mod chmmr;
pub mod comandr;
pub mod druuge;
pub(crate) mod encounter;
pub mod ilwrath;
pub mod melnorm;
pub mod mycon;
pub mod orz;
pub mod pkunk;
pub mod rebel;
pub(crate) mod script;
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

#[derive(Default)]
pub(crate) struct RuntimeScratch {
    pub druuge: Option<druuge::Session>,
    pub mycon: Option<mycon::Session>,
    pub melnorm: Option<melnorm::Session>,
    pub spathi: Option<spathi::Session>,
    pub starbas: Option<starbas::Session>,
    pub slyland_choices: [u16; 5],
    pub last_alien: u8,
}
