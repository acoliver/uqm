//! Unchanged ABI storage declarations used at the Stage B boundary.
#![allow(non_snake_case, non_camel_case_types)]
use super::super::record::{QUEUE, STARSHIP, VELOCITY_DESC};
use crate::collections::queue::HLink as HLINK;
use crate::graphics::native_abi::*;
use crate::graphics::native_abi::{
    BYTE as BATTLE_INPUT_STATE, DWORD as TimeCount, UWORD as ACTIVITY,
};

pub type HELEMENT = HLINK;

pub type HSTARSHIP = HLINK;

pub type GETMELEE_STATE = getmelee_struct;

#[repr(C)]
pub struct getmelee_struct {
    pub InputFunc:
        ::std::option::Option<unsafe extern "C" fn(pInputState: *mut getmelee_struct) -> BOOLEAN>,
    pub Initialized: BOOLEAN,
    pub player: [getmelee_struct__bindgen_ty_1; 2usize],
    pub net: Option<std::rc::Rc<dyn crate::engine::battle::net::BattleNet>>,
}

#[repr(C)]
pub struct getmelee_struct__bindgen_ty_1 {
    pub timeIn: TimeCount,
    pub hBattleShip: HSTARSHIP,
    pub choice: COUNT,
    pub row: COUNT,
    pub col: COUNT,
    pub ships_left: COUNT,
    pub randomIndex: COUNT,
    pub selecting: BOOLEAN,
    pub done: BOOLEAN,
    pub flashContext: *mut FlashContext,
    pub remoteSelected: BOOLEAN,
}

pub type BattleFrameInputFunction = ::std::option::Option<
    unsafe extern "C" fn(
        context: *mut InputContext,
        StarShipPtr: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE,
>;

pub type SelectShipFunction = ::std::option::Option<
    unsafe extern "C" fn(context: *mut InputContext, gms: *mut GETMELEE_STATE) -> BOOLEAN,
>;

pub type BattleEndReadyFunction =
    ::std::option::Option<unsafe extern "C" fn(context: *mut InputContext) -> bool>;

pub type DeleteInputContextFunction =
    ::std::option::Option<unsafe extern "C" fn(context: *mut InputContext)>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BattleInputHandlers {
    pub frameInput: BattleFrameInputFunction,
    pub selectShip: SelectShipFunction,
    pub battleEndReady: BattleEndReadyFunction,
    pub deleteContext: DeleteInputContextFunction,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct InputContext {
    pub handlers: *mut BattleInputHandlers,
    pub playerNr: COUNT,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CLOCK_STATE {
    pub day_index: BYTE,
    pub month_index: BYTE,
    pub year_index: COUNT,
    pub tick_count: SIZE,
    pub day_in_ticks: SIZE,
    pub event_q: QUEUE,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GAME_STATE {
    pub glob_flags: BYTE,
    pub CrewCost: BYTE,
    pub FuelCost: BYTE,
    pub ModuleCost: [BYTE; 20usize],
    pub ElementWorth: [BYTE; 8usize],
    pub _display_array: *mut PRIMITIVE,
    pub CurrentActivity: ACTIVITY,
    pub GameClock: CLOCK_STATE,
    pub autopilot: POINT,
    pub ip_location: POINT,
    pub ShipStamp: STAMP,
    pub ShipFacing: UWORD,
    pub ip_planet: BYTE,
    pub in_orbit: BYTE,
    pub velocity: VELOCITY_DESC,
    pub BattleGroupRef: DWORD,
    pub avail_race_q: QUEUE,
    pub npc_built_ship_q: QUEUE,
    pub ip_group_q: QUEUE,
    pub encounter_q: QUEUE,
    pub built_ship_q: QUEUE,
}

pub type FlashState = ::std::os::raw::c_uint;

pub type FlashType = ::std::os::raw::c_uint;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct FlashContext {
    pub gfxContext: CONTEXT,
    pub rect: RECT,
    pub original: FRAME,
    pub type_: FlashType,
    pub u: FlashContext__bindgen_ty_1,
    pub startNumer: ::std::os::raw::c_int,
    pub endNumer: ::std::os::raw::c_int,
    pub denom: ::std::os::raw::c_int,
    pub fadeInTime: TimeCount,
    pub onTime: TimeCount,
    pub fadeOutTime: TimeCount,
    pub offTime: TimeCount,
    pub frameTime: TimeCount,
    pub state: FlashState,
    pub lastStateTime: TimeCount,
    pub lastFrameTime: TimeCount,
    pub started: BOOLEAN,
    pub paused: BOOLEAN,
    pub cache: *mut FRAME,
    pub cacheSize: COUNT,
    pub lastFrameIndex: COUNT,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union FlashContext__bindgen_ty_1 {
    pub transition: FlashContext__bindgen_ty_1__bindgen_ty_1,
    pub overlay: FlashContext__bindgen_ty_1__bindgen_ty_2,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlashContext__bindgen_ty_1__bindgen_ty_1 {
    pub first: FRAME,
    pub final_: FRAME,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlashContext__bindgen_ty_1__bindgen_ty_2 {
    pub frame: FRAME,
}

pub(crate) const DISPLAY_ARRAY_OFFSET: usize = std::mem::offset_of!(GAME_STATE, _display_array);
