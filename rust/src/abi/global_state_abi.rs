// Shared engine records used by the Rust runtime.

pub const MAX_ENCOUNTERS: u32 = 16;
pub const MAX_BATTLE_GROUPS: u32 = 32;
pub const PLAYER_SHIP: u32 = 4;
pub const RPG_PLAYER_NUM: u32 = 0;
pub const MAX_SUNS: u32 = 1;
pub const MAX_PLANETS: u32 = 16;
pub const MAX_MOONS: u32 = 4;
pub const MAX_HYPER_SHIPS: u32 = 7;
pub type uint8 = ::std::os::raw::c_uchar;
pub type sint8 = ::std::os::raw::c_schar;
pub type uint16 = ::std::os::raw::c_ushort;
pub type sint16 = ::std::os::raw::c_short;
pub type uint32 = ::std::os::raw::c_uint;
pub type sint32 = ::std::os::raw::c_int;
pub type BYTE = uint8;
pub type SBYTE = sint8;
pub type UWORD = uint16;
pub type SWORD = sint16;
pub type DWORD = uint32;
pub type SDWORD = sint32;
pub type COUNT = UWORD;
pub type SIZE = SWORD;
pub type UNICODE = ::std::os::raw::c_char;
pub const BOOLEAN_FALSE: BOOLEAN = 0;
pub const BOOLEAN_TRUE: BOOLEAN = 1;
pub type BOOLEAN = ::std::os::raw::c_uint;
pub type QUEUE_HANDLE = *mut ::std::os::raw::c_void;
pub type HLINK = QUEUE_HANDLE;
pub use crate::ships::native::abi::CLOCK_STATE;
pub use crate::engine::battle::record::QUEUE;
pub type HEVENT = HLINK;
pub use crate::navigation::abi::event;
pub type EVENT = event;
pub use crate::graphics::native_abi::Color;
pub type RESOURCE = *const ::std::os::raw::c_char;
pub use crate::graphics::native_abi::string_table_entry;
pub type STRING_TABLE_ENTRY_DESC = string_table_entry;
pub type STRING = *mut STRING_TABLE_ENTRY_DESC;
pub type FRAME_DESC = frame_desc;
pub use crate::graphics::native_abi::font_desc;
pub type FONT_DESC = font_desc;
pub type FRAME = *mut FRAME_DESC;
pub type FONT = *mut FONT_DESC;
pub type TIME_VALUE = UWORD;
pub type COORD = SWORD;
pub use crate::graphics::native_abi::extent;
pub type EXTENT = extent;
pub use crate::graphics::native_abi::point;
pub type POINT = point;
pub use crate::graphics::native_abi::stamp;
pub type STAMP = stamp;
pub use crate::graphics::native_abi::rect;
pub type RECT = rect;
pub use crate::graphics::native_abi::line;
pub type LINE = line;
pub const TEXT_ALIGN_ALIGN_LEFT: TEXT_ALIGN = 0;
pub const TEXT_ALIGN_ALIGN_CENTER: TEXT_ALIGN = 1;
pub const TEXT_ALIGN_ALIGN_RIGHT: TEXT_ALIGN = 2;
pub type TEXT_ALIGN = ::std::os::raw::c_uint;
pub const TEXT_VALIGN_VALIGN_TOP: TEXT_VALIGN = 0;
pub const TEXT_VALIGN_VALIGN_MIDDLE: TEXT_VALIGN = 1;
pub const TEXT_VALIGN_VALIGN_BOTTOM: TEXT_VALIGN = 2;
pub type TEXT_VALIGN = ::std::os::raw::c_uint;
pub use crate::graphics::native_abi::text;
pub type TEXT = text;
pub type COLORMAP = STRING;
pub const gfx_object_POINT_PRIM: gfx_object = 0;
pub const gfx_object_STAMP_PRIM: gfx_object = 1;
pub const gfx_object_STAMPFILL_PRIM: gfx_object = 2;
pub const gfx_object_LINE_PRIM: gfx_object = 3;
pub const gfx_object_TEXT_PRIM: gfx_object = 4;
pub const gfx_object_RECT_PRIM: gfx_object = 5;
pub const gfx_object_RECTFILL_PRIM: gfx_object = 6;
pub const gfx_object_NUM_PRIMS: gfx_object = 7;
pub type gfx_object = ::std::os::raw::c_uint;
pub type GRAPHICS_PRIM = BYTE;
pub use crate::graphics::native_abi::PRIM_DESC;
pub type PRIM_LINKS = DWORD;
pub use crate::engine::battle::record::PRIMITIVE;
pub type BATCH_FLAGS = BYTE;
pub use crate::ships::native::abi::INTERSECT_CONTROL;
pub type INTERSECT_CODE = BYTE;
pub type HOT_SPOT = POINT;
pub use crate::graphics::context_abi::{
    GetContextClipRect, GetContextDrawMode, GetContextForeGroundColor,
};
pub use crate::graphics::drawable_abi::GetContextFGFrame;
pub use crate::graphics::frame_abi::{DrawBatch, DrawLine};
pub use crate::graphics::native_abi::DrawMode;
pub use crate::ships::native::abi::tfb_soundsample;
pub type TFB_SoundSample = tfb_soundsample;
pub type MUSIC_REF = *mut *mut TFB_SoundSample;
pub use crate::ships::native::abi::velocity_desc;
pub use crate::ships::native::abi::SIS_STATE;
pub type VELOCITY_DESC = velocity_desc;
pub use crate::ships::native::abi::ANIMATION_DESC;
pub use crate::ships::native::abi::NUMBER_SPEECH_DESC;
pub use crate::ships::native::abi::SPEECH_DIGIT;
pub use crate::ships::native::abi::SPEECH_DIGITNAME;
pub type NUMBER_SPEECH = *const NUMBER_SPEECH_DESC;
pub type LDAS_FLAGS = DWORD;
pub use crate::ships::native::abi::LOCDATA;
pub type ACTIVITY = UWORD;
pub use crate::ships::native::abi::GAME_STATE;
pub use crate::ships::native::abi::GLOBDATA;
pub const ARILOU_ENTRANCE_EVENT: _bindgen_ty_10 = 0;
pub const ARILOU_EXIT_EVENT: _bindgen_ty_10 = 1;
pub const HYPERSPACE_ENCOUNTER_EVENT: _bindgen_ty_10 = 2;
pub const KOHR_AH_VICTORIOUS_EVENT: _bindgen_ty_10 = 3;
pub const ADVANCE_PKUNK_MISSION: _bindgen_ty_10 = 4;
pub const ADVANCE_THRADD_MISSION: _bindgen_ty_10 = 5;
pub const ZOQFOT_DISTRESS_EVENT: _bindgen_ty_10 = 6;
pub const ZOQFOT_DEATH_EVENT: _bindgen_ty_10 = 7;
pub const SHOFIXTI_RETURN_EVENT: _bindgen_ty_10 = 8;
pub const ADVANCE_UTWIG_SUPOX_MISSION: _bindgen_ty_10 = 9;
pub const KOHR_AH_GENOCIDE_EVENT: _bindgen_ty_10 = 10;
pub const SPATHI_SHIELD_EVENT: _bindgen_ty_10 = 11;
pub const ADVANCE_ILWRATH_MISSION: _bindgen_ty_10 = 12;
pub const ADVANCE_MYCON_MISSION: _bindgen_ty_10 = 13;
pub const ARILOU_UMGAH_CHECK: _bindgen_ty_10 = 14;
pub const YEHAT_REBEL_EVENT: _bindgen_ty_10 = 15;
pub const SLYLANDRO_RAMP_UP: _bindgen_ty_10 = 16;
pub const SLYLANDRO_RAMP_DOWN: _bindgen_ty_10 = 17;
pub const NUM_EVENTS: _bindgen_ty_10 = 18;
pub type _bindgen_ty_10 = ::std::os::raw::c_uint;
extern "C" {
    pub static mut ScreenWidth: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut ScreenHeight: ::std::os::raw::c_int;
}
pub type HELEMENT = HLINK;
pub type ELEMENT_FLAGS = UWORD;
pub use crate::engine::battle::record::state;
pub use crate::engine::battle::record::state__bindgen_ty_1;
pub type STATE = state;
pub use crate::engine::battle::record::ELEMENT;
pub use crate::engine::battle::record::{ElementProcessFunc, ElementCollisionFunc};
pub use crate::engine::battle::record::element;
pub use crate::engine::battle::record::element__bindgen_ty_1;
pub use crate::engine::battle::record::element__bindgen_ty_2;
pub use crate::engine::battle::record::element__bindgen_ty_3;
pub use crate::engine::battle::record::element__bindgen_ty_4;
pub use crate::engine::battle::record::element__bindgen_ty_5;
pub type STATUS_FLAGS = UWORD;
pub const SPECIES_ID_NO_ID: SPECIES_ID = 0;
pub const SPECIES_ID_ARILOU_ID: SPECIES_ID = 1;
pub const SPECIES_ID_CHMMR_ID: SPECIES_ID = 2;
pub const SPECIES_ID_EARTHLING_ID: SPECIES_ID = 3;
pub const SPECIES_ID_ORZ_ID: SPECIES_ID = 4;
pub const SPECIES_ID_PKUNK_ID: SPECIES_ID = 5;
pub const SPECIES_ID_SHOFIXTI_ID: SPECIES_ID = 6;
pub const SPECIES_ID_SPATHI_ID: SPECIES_ID = 7;
pub const SPECIES_ID_SUPOX_ID: SPECIES_ID = 8;
pub const SPECIES_ID_THRADDASH_ID: SPECIES_ID = 9;
pub const SPECIES_ID_UTWIG_ID: SPECIES_ID = 10;
pub const SPECIES_ID_VUX_ID: SPECIES_ID = 11;
pub const SPECIES_ID_YEHAT_ID: SPECIES_ID = 12;
pub const SPECIES_ID_MELNORME_ID: SPECIES_ID = 13;
pub const SPECIES_ID_DRUUGE_ID: SPECIES_ID = 14;
pub const SPECIES_ID_ILWRATH_ID: SPECIES_ID = 15;
pub const SPECIES_ID_MYCON_ID: SPECIES_ID = 16;
pub const SPECIES_ID_SLYLANDRO_ID: SPECIES_ID = 17;
pub const SPECIES_ID_UMGAH_ID: SPECIES_ID = 18;
pub const SPECIES_ID_UR_QUAN_ID: SPECIES_ID = 19;
pub const SPECIES_ID_ZOQFOTPIK_ID: SPECIES_ID = 20;
pub const SPECIES_ID_SYREEN_ID: SPECIES_ID = 21;
pub const SPECIES_ID_KOHR_AH_ID: SPECIES_ID = 22;
pub const SPECIES_ID_ANDROSYNTH_ID: SPECIES_ID = 23;
pub const SPECIES_ID_CHENJESU_ID: SPECIES_ID = 24;
pub const SPECIES_ID_MMRNMHRM_ID: SPECIES_ID = 25;
pub const SPECIES_ID_LAST_MELEE_ID: SPECIES_ID = 25;
pub const SPECIES_ID_SIS_SHIP_ID: SPECIES_ID = 26;
pub const SPECIES_ID_SA_MATRA_ID: SPECIES_ID = 27;
pub const SPECIES_ID_UR_QUAN_PROBE_ID: SPECIES_ID = 28;
pub const SPECIES_ID_NUM_SPECIES_ID: SPECIES_ID = 29;
pub type SPECIES_ID = ::std::os::raw::c_uint;
pub use crate::engine::battle::record::RACE_DESC;
pub use crate::ships::native::abi::SHIP_FRAGMENT;
pub use crate::engine::battle::record::STARSHIP;
pub type HFLEETINFO = HLINK;
pub use crate::ships::native::abi::FLEET_INFO;
pub type HIPGROUP = HLINK;
pub use crate::navigation::abi::IP_GROUP;
pub type BRIEF_SHIP_INFO = brief_ship_info;
pub type ENCOUNTER = encounter;
pub type PLANET_DESC = planet_desc;
pub type STAR_DESC = star_desc;
pub type NODE_INFO = node_info;
pub type PLANET_ORBIT = planet_orbit;
pub type SOLARSYS_STATE = solarsys_state;
pub type InitNpcsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type ReinitNpcsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type UninitNpcsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type GeneratePlanetsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type GenerateMoonsFunction = ::std::option::Option<
    unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE, planet: *mut PLANET_DESC) -> bool,
>;
pub type GenerateOrbitalFunction = ::std::option::Option<
    unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool,
>;
pub type GenerateNameFunction = ::std::option::Option<
    unsafe extern "C" fn(arg1: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> bool,
>;
pub type GenerateMineralsFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        whichNode: COUNT,
        arg2: *mut NODE_INFO,
    ) -> COUNT,
>;
pub type GenerateEnergyFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        whichNode: COUNT,
        arg2: *mut NODE_INFO,
    ) -> COUNT,
>;
pub type GenerateLifeFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        whichNode: COUNT,
        arg2: *mut NODE_INFO,
    ) -> COUNT,
>;
pub type PickupMineralsFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut PLANET_DESC,
        whichNode: COUNT,
    ) -> bool,
>;
pub type PickupEnergyFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut PLANET_DESC,
        whichNode: COUNT,
    ) -> bool,
>;
pub type PickupLifeFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut PLANET_DESC,
        whichNode: COUNT,
    ) -> bool,
>;
pub use crate::navigation::abi::FlashContext;
pub use crate::ships::native::abi::menu_state;
pub use crate::ships::native::abi::GenerateFunctions;
pub type MENU_STATE = menu_state;
pub use crate::navigation::abi::star_desc;
#[cfg(not(test))]
pub use crate::planets::render::c_api::{
    draw_filled_oval as DrawFilledOval, draw_oval as DrawOval,
};
#[cfg(not(test))]
pub use crate::planets::solarsys::{planetOuterLocation, XFormIPLoc};
pub use crate::ships::native::abi::node_info;
pub use crate::ships::native::abi::planet_desc;
pub use crate::ships::native::abi::planet_orbit;
pub use crate::ships::native::abi::solarsys_state;
pub use crate::ships::native::abi::PlanetFrame;
pub use crate::ships::native::abi::ELEMENT_ENTRY;
pub use crate::ships::native::abi::PLANET_INFO;
pub use crate::ships::native::abi::SYSTEM_INFO;
// The runtime providers are excluded from the isolated unit-test build.
#[cfg(test)]
#[cfg(not(test))]
pub use crate::battle::build_native::engine::EscortFeasibilityStudy;
#[cfg(test)]
#[cfg(not(test))]
pub use crate::planets::render::c_api::draw_filled_oval as DrawFilledOval;
#[cfg(test)]
#[cfg(not(test))]
pub use crate::planets::render::c_api::draw_oval as DrawOval;
#[cfg(test)]
#[cfg(not(test))]
pub use crate::planets::solarsys::planetOuterLocation;
#[cfg(test)]
#[cfg(not(test))]
pub use crate::planets::solarsys::XFormIPLoc;
pub type HENCOUNTER = HLINK;
pub use crate::navigation::abi::brief_ship_info;
pub use crate::navigation::abi::encounter;
extern "C" {
    pub static mut EncounterRace: SIZE;
}
extern "C" {
    pub static mut EncounterGroup: BYTE;
}
pub const STAR_STRING_BASE: _bindgen_ty_24 = 0;
pub const DEVICE_STRING_BASE: _bindgen_ty_24 = 133;
pub const CARGO_STRING_BASE: _bindgen_ty_24 = 162;
pub const ELEMENTS_STRING_BASE: _bindgen_ty_24 = 172;
pub const SCAN_STRING_BASE: _bindgen_ty_24 = 305;
pub const STAR_NUMBER_BASE: _bindgen_ty_24 = 361;
pub const PLANET_NUMBER_BASE: _bindgen_ty_24 = 375;
pub const MONTHS_STRING_BASE: _bindgen_ty_24 = 408;
pub const FEEDBACK_STRING_BASE: _bindgen_ty_24 = 420;
pub const STARBASE_STRING_BASE: _bindgen_ty_24 = 422;
pub const ENCOUNTER_STRING_BASE: _bindgen_ty_24 = 427;
pub const NAVIGATION_STRING_BASE: _bindgen_ty_24 = 435;
pub const NAMING_STRING_BASE: _bindgen_ty_24 = 441;
pub const MELEE_STRING_BASE: _bindgen_ty_24 = 445;
pub const SAVEGAME_STRING_BASE: _bindgen_ty_24 = 454;
pub const OPTION_STRING_BASE: _bindgen_ty_24 = 459;
pub const QUITMENU_STRING_BASE: _bindgen_ty_24 = 464;
pub const STATUS_STRING_BASE: _bindgen_ty_24 = 467;
pub const FLAGSHIP_STRING_BASE: _bindgen_ty_24 = 473;
pub const ORBITSCAN_STRING_BASE: _bindgen_ty_24 = 486;
pub const MAINMENU_STRING_BASE: _bindgen_ty_24 = 505;
pub const NETMELEE_STRING_BASE: _bindgen_ty_24 = 560;
pub const GAMESTR_COUNT: _bindgen_ty_24 = 579;
pub type _bindgen_ty_24 = ::std::os::raw::c_uint;
pub const SOL_DEFINED: _bindgen_ty_25 = 1;
pub const SHOFIXTI_DEFINED: _bindgen_ty_25 = 2;
pub const MAIDENS_DEFINED: _bindgen_ty_25 = 3;
pub const START_COLONY_DEFINED: _bindgen_ty_25 = 4;
pub const SPATHI_DEFINED: _bindgen_ty_25 = 5;
pub const ZOQFOT_DEFINED: _bindgen_ty_25 = 6;
pub const MELNORME0_DEFINED: _bindgen_ty_25 = 7;
pub const MELNORME1_DEFINED: _bindgen_ty_25 = 8;
pub const MELNORME2_DEFINED: _bindgen_ty_25 = 9;
pub const MELNORME3_DEFINED: _bindgen_ty_25 = 10;
pub const MELNORME4_DEFINED: _bindgen_ty_25 = 11;
pub const MELNORME5_DEFINED: _bindgen_ty_25 = 12;
pub const MELNORME6_DEFINED: _bindgen_ty_25 = 13;
pub const MELNORME7_DEFINED: _bindgen_ty_25 = 14;
pub const MELNORME8_DEFINED: _bindgen_ty_25 = 15;
pub const TALKING_PET_DEFINED: _bindgen_ty_25 = 16;
pub const CHMMR_DEFINED: _bindgen_ty_25 = 17;
pub const SYREEN_DEFINED: _bindgen_ty_25 = 18;
pub const BURVIXESE_DEFINED: _bindgen_ty_25 = 19;
pub const SLYLANDRO_DEFINED: _bindgen_ty_25 = 20;
pub const DRUUGE_DEFINED: _bindgen_ty_25 = 21;
pub const BOMB_DEFINED: _bindgen_ty_25 = 22;
pub const AQUA_HELIX_DEFINED: _bindgen_ty_25 = 23;
pub const SUN_DEVICE_DEFINED: _bindgen_ty_25 = 24;
pub const TAALO_PROTECTOR_DEFINED: _bindgen_ty_25 = 25;
pub const SHIP_VAULT_DEFINED: _bindgen_ty_25 = 26;
pub const URQUAN_WRECK_DEFINED: _bindgen_ty_25 = 27;
pub const VUX_BEAST_DEFINED: _bindgen_ty_25 = 28;
pub const SAMATRA_DEFINED: _bindgen_ty_25 = 29;
pub const ZOQ_SCOUT_DEFINED: _bindgen_ty_25 = 30;
pub const MYCON_DEFINED: _bindgen_ty_25 = 31;
pub const EGG_CASE0_DEFINED: _bindgen_ty_25 = 32;
pub const EGG_CASE1_DEFINED: _bindgen_ty_25 = 33;
pub const EGG_CASE2_DEFINED: _bindgen_ty_25 = 34;
pub const PKUNK_DEFINED: _bindgen_ty_25 = 35;
pub const UTWIG_DEFINED: _bindgen_ty_25 = 36;
pub const SUPOX_DEFINED: _bindgen_ty_25 = 37;
pub const YEHAT_DEFINED: _bindgen_ty_25 = 38;
pub const VUX_DEFINED: _bindgen_ty_25 = 39;
pub const ORZ_DEFINED: _bindgen_ty_25 = 40;
pub const THRADD_DEFINED: _bindgen_ty_25 = 41;
pub const RAINBOW_DEFINED: _bindgen_ty_25 = 42;
pub const ILWRATH_DEFINED: _bindgen_ty_25 = 43;
pub const ANDROSYNTH_DEFINED: _bindgen_ty_25 = 44;
pub const MYCON_TRAP_DEFINED: _bindgen_ty_25 = 45;
pub type _bindgen_ty_25 = ::std::os::raw::c_uint;
pub use crate::graphics::native_abi::tfb_image;
pub type TFB_Image = tfb_image;
pub use crate::graphics::geometry_ffi::BresenhamLine as bresenham_line;
pub type BRESENHAM_LINE = bresenham_line;
pub type DRAWABLE_TYPE = UWORD;
#[cfg(not(test))]
pub use crate::battle::build_native::EscortFeasibilityStudy;
pub use crate::graphics::geometry_ffi::_clip_line;
pub use crate::graphics::native_abi::drawable_desc;
pub use crate::graphics::native_abi::frame_desc;
pub use crate::navigation::abi::SUMMARY_DESC;

#[cfg(test)]
mod signature_tests {
    use super::*;
    use crate::graphics::{geometry_ffi::BresenhamLine, native_abi as gfx};

    const _: unsafe extern "C" fn() -> gfx::Color = GetContextForeGroundColor;
    const _: unsafe extern "C" fn() -> gfx::FRAME = GetContextFGFrame;
    const _: unsafe extern "C" fn(*mut gfx::RECT) -> BOOLEAN = GetContextClipRect;
    const _: unsafe extern "C" fn() -> gfx::DrawMode = GetContextDrawMode;
    const _: unsafe extern "C" fn(*mut gfx::PRIMITIVE, PRIM_LINKS, BATCH_FLAGS) = DrawBatch;
    const _: unsafe extern "C" fn(*mut gfx::LINE) = DrawLine;
    const _: unsafe extern "C" fn(*const gfx::RECT, *mut BresenhamLine) -> INTERSECT_CODE =
        _clip_line;

    const _: () = {
        assert!(std::mem::size_of::<BRESENHAM_LINE>() == std::mem::size_of::<BresenhamLine>());
        assert!(
            std::mem::offset_of!(BRESENHAM_LINE, intersect_code)
                == std::mem::offset_of!(BresenhamLine, intersect_code)
        );
    };
}
