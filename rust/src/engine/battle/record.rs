//! Shared battle record layouts during the storage transition.
use crate::battle::velocity::VelocityDesc as VELOCITY_DESC;
use crate::collections::queue::{HLink as HELEMENT, HLink as HLINK};
use crate::graphics::native_abi::{
    BYTE, COUNT, FRAME, POINT, RESOURCE, SIZE, STRING, STRING as SOUND, UWORD,
    UWORD as ELEMENT_FLAGS, UWORD as STATUS_FLAGS,
};
use std::os::raw::c_uint as SPECIES_ID;

pub use crate::collections::queue::Queue;
pub use crate::graphics::native_abi::{INTERSECT_CONTROL, PRIMITIVE};

pub type QUEUE = Queue;

pub use abi::{
    captain_stuff, element, element__bindgen_ty_1, element__bindgen_ty_2, element__bindgen_ty_3,
    element__bindgen_ty_4, element__bindgen_ty_5, race_desc, state, state__bindgen_ty_1,
    tfb_soundsample, ElementCollisionFunc, ElementProcessFunc, IntelligenceFunc, TFB_SoundSample,
    CAPTAIN_STUFF, CHARACTERISTIC_STUFF, DATA_STUFF, ELEMENT, EVALUATE_DESC, FLEET_STUFF,
    INIT_WEAPON_FUNC, INTEL_STUFF, MAX_DISPLAY_PRIMS, MOVEMENT_STATE, MUSIC_REF, POSTPROCESS_FUNC,
    PREPROCESS_FUNC, PRIM_DESC, RACE_DESC, SHIP_INFO, STARSHIP, STATE, UNINIT_FUNC,
};

#[allow(non_camel_case_types, non_snake_case)]
mod abi {
    use super::*;
    pub use crate::graphics::native_abi::PRIM_DESC;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct state {
        pub location: POINT,
        pub image: state__bindgen_ty_1,
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct state__bindgen_ty_1 {
        pub frame: FRAME,
        pub farray: *mut FRAME,
    }
    pub type STATE = state;
    pub type ELEMENT = element;
    const _: () = {
        use crate::battle::element::{Element, ElementVisualState as Visual};
        use std::mem::offset_of as offset;
        assert!(size_of::<ELEMENT>() == size_of::<Element>());
        assert!(align_of::<ELEMENT>() == align_of::<Element>());
        assert!(offset!(ELEMENT, succ) == offset!(Element, succ));
        assert!(offset!(ELEMENT, state_flags) == offset!(Element, state_flags));
        assert!(offset!(ELEMENT, current) == offset!(Element, current));
        assert!(offset!(STATE, location) == offset!(Visual, location));
        assert!(
            offset!(STATE, image) + offset!(state__bindgen_ty_1, frame) == offset!(Visual, frame)
        );
        assert!(
            offset!(STATE, image) + offset!(state__bindgen_ty_1, farray) == offset!(Visual, farray)
        );
    };
    pub type ElementProcessFunc = Option<super::super::hooks::ElementHook>;
    pub type ElementCollisionFunc = Option<super::super::hooks::CollisionHook>;
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct element {
        pub pred: HELEMENT,
        pub succ: HELEMENT,
        pub preprocess_func: ElementProcessFunc,
        pub postprocess_func: ElementProcessFunc,
        pub collision_func: ElementCollisionFunc,
        pub death_func: ElementProcessFunc,
        pub playerNr: SIZE,
        pub state_flags: ELEMENT_FLAGS,
        pub __bindgen_anon_1: element__bindgen_ty_1,
        pub __bindgen_anon_2: element__bindgen_ty_2,
        pub __bindgen_anon_3: element__bindgen_ty_3,
        pub __bindgen_anon_4: element__bindgen_ty_4,
        pub __bindgen_anon_5: element__bindgen_ty_5,
        pub colorCycleIndex: BYTE,
        pub velocity: VELOCITY_DESC,
        pub IntersectControl: INTERSECT_CONTROL,
        pub PrimIndex: COUNT,
        pub current: STATE,
        pub next: STATE,
        pub pParent: *mut ::std::os::raw::c_void,
        pub hTarget: HELEMENT,
    }
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union element__bindgen_ty_1 {
        pub life_span: COUNT,
        pub scan_node: COUNT,
    }
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union element__bindgen_ty_2 {
        pub crew_level: COUNT,
        pub hit_points: COUNT,
        pub facing: COUNT,
        pub cycle: COUNT,
    }
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union element__bindgen_ty_3 {
        pub mass_points: BYTE,
    }
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union element__bindgen_ty_4 {
        pub turn_wait: BYTE,
        pub sys_loc: BYTE,
    }
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub union element__bindgen_ty_5 {
        pub thrust_wait: BYTE,
        pub blast_offset: BYTE,
        pub next_turn: BYTE,
    }

    pub type RACE_DESC = race_desc;
    pub type PREPROCESS_FUNC = Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
    pub type POSTPROCESS_FUNC = Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
    pub type INIT_WEAPON_FUNC =
        Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT, Weapon: *mut HELEMENT) -> COUNT>;
    pub type UNINIT_FUNC = Option<unsafe extern "C" fn(pRaceDesc: *mut RACE_DESC)>;
    #[repr(C)]
    #[repr(align(16))]
    #[derive(Debug, Copy, Clone)]
    pub struct race_desc {
        pub ship_info: SHIP_INFO,
        pub fleet: FLEET_STUFF,
        pub __bindgen_padding_0: [u16; 5usize],
        pub characteristics: CHARACTERISTIC_STUFF,
        pub ship_data: DATA_STUFF,
        pub cyborg_control: INTEL_STUFF,
        pub uninit_func: UNINIT_FUNC,
        pub preprocess_func: PREPROCESS_FUNC,
        pub postprocess_func: POSTPROCESS_FUNC,
        pub init_weapon_func: INIT_WEAPON_FUNC,
        pub data: *mut ::std::os::raw::c_void,
        pub CodeRef: *mut ::std::os::raw::c_void,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct STARSHIP {
        pub pred: HLINK,
        pub succ: HLINK,
        pub SpeciesID: SPECIES_ID,
        pub captains_name_index: BYTE,
        pub RaceDescPtr: *mut RACE_DESC,
        pub crew_level: COUNT,
        pub max_crew: COUNT,
        pub ship_cost: BYTE,
        pub index: COUNT,
        pub race_strings: STRING,
        pub icons: FRAME,
        pub weapon_counter: BYTE,
        pub special_counter: BYTE,
        pub energy_counter: BYTE,
        pub ship_input_state: BYTE,
        pub cur_status_flags: STATUS_FLAGS,
        pub old_status_flags: STATUS_FLAGS,
        pub hShip: HELEMENT,
        pub ShipFacing: COUNT,
        pub playerNr: SIZE,
        pub control: BYTE,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct tfb_soundsample {
        _unused: [u8; 0],
    }

    pub type TFB_SoundSample = tfb_soundsample;

    pub type MUSIC_REF = *mut *mut TFB_SoundSample;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct captain_stuff {
        pub captain_rsc: RESOURCE,
        pub background: FRAME,
        pub turn: FRAME,
        pub thrust: FRAME,
        pub weapon: FRAME,
        pub special: FRAME,
    }

    pub type CAPTAIN_STUFF = captain_stuff;

    pub type MOVEMENT_STATE = ::std::os::raw::c_uint;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct EVALUATE_DESC {
        pub ObjectPtr: *mut ELEMENT,
        pub facing: COUNT,
        pub which_turn: COUNT,
        pub MoveState: MOVEMENT_STATE,
    }

    pub type IntelligenceFunc = Option<
        unsafe extern "C" fn(
            ShipPtr: *mut ELEMENT,
            ObjectsOfConcern: *mut EVALUATE_DESC,
            ConcernCounter: COUNT,
        ),
    >;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct INTEL_STUFF {
        pub ManeuverabilityIndex: COUNT,
        pub WeaponRange: COUNT,
        pub intelligence_func: IntelligenceFunc,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct CHARACTERISTIC_STUFF {
        pub max_thrust: COUNT,
        pub thrust_increment: COUNT,
        pub energy_regeneration: BYTE,
        pub weapon_energy_cost: BYTE,
        pub special_energy_cost: BYTE,
        pub energy_wait: BYTE,
        pub turn_wait: BYTE,
        pub thrust_wait: BYTE,
        pub weapon_wait: BYTE,
        pub special_wait: BYTE,
        pub ship_mass: BYTE,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct SHIP_INFO {
        pub ship_flags: UWORD,
        pub ship_cost: BYTE,
        pub crew_level: COUNT,
        pub max_crew: COUNT,
        pub energy_level: BYTE,
        pub max_energy: BYTE,
        pub race_strings_rsc: RESOURCE,
        pub icons_rsc: RESOURCE,
        pub melee_icon_rsc: RESOURCE,
        pub race_strings: STRING,
        pub icons: FRAME,
        pub melee_icon: FRAME,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct FLEET_STUFF {
        pub strength: COUNT,
        pub known_loc: POINT,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct DATA_STUFF {
        pub ship_rsc: [RESOURCE; 3usize],
        pub weapon_rsc: [RESOURCE; 3usize],
        pub special_rsc: [RESOURCE; 3usize],
        pub captain_control: CAPTAIN_STUFF,
        pub victory_ditty_rsc: RESOURCE,
        pub ship_sounds_rsc: RESOURCE,
        pub ship: [FRAME; 3usize],
        pub weapon: [FRAME; 3usize],
        pub special: [FRAME; 3usize],
        pub victory_ditty: MUSIC_REF,
        pub ship_sounds: SOUND,
    }

    pub const MAX_DISPLAY_PRIMS: u32 = 330;
}
