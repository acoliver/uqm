//! Shared battle record layouts during the storage transition.
#![allow(non_camel_case_types, non_snake_case)]

use crate::collections::queue::HLink;
use crate::graphics::native_abi::{Color, GRAPHICS_PRIM, PRIM_DESC, PRIM_LINKS};
use crate::ships::native::abi::{
    BYTE, CHARACTERISTIC_STUFF, COUNT, DATA_STUFF, ELEMENT_FLAGS, FLEET_STUFF, FRAME, HELEMENT,
    HLINK, INTEL_STUFF, INTERSECT_CONTROL, POINT, SHIP_INFO, SIZE, SPECIES_ID, STATUS_FLAGS,
    STRING, VELOCITY_DESC,
};

/// QUEUE — the queue control structure
/// C: `typedef struct queue { HLINK head; HLINK tail; BYTE *pq_tab;
///      HLINK free_list; COUNT object_size; BYTE num_objects; } QUEUE;`
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Queue {
    pub head: HLink,
    pub tail: HLink,
    pub pq_tab: *mut u8,
    pub free_list: HLink,
    pub object_size: u16,
    pub num_objects: u8,
}

pub type QUEUE = Queue;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct PRIMITIVE {
    pub Links: PRIM_LINKS,
    pub Type: GRAPHICS_PRIM,
    pub color: Color,
    pub Object: PRIM_DESC,
}

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
    use crate::battle::element::{Element, ElementVisualState};
    assert!(size_of::<ELEMENT>() == size_of::<Element>());
    assert!(align_of::<ELEMENT>() == align_of::<Element>());
    assert!(std::mem::offset_of!(ELEMENT, succ) == std::mem::offset_of!(Element, succ));
    assert!(
        std::mem::offset_of!(ELEMENT, state_flags) == std::mem::offset_of!(Element, state_flags)
    );
    assert!(std::mem::offset_of!(ELEMENT, current) == std::mem::offset_of!(Element, current));
    assert!(
        std::mem::offset_of!(STATE, location) == std::mem::offset_of!(ElementVisualState, location)
    );
    assert!(
        std::mem::offset_of!(STATE, image) + std::mem::offset_of!(state__bindgen_ty_1, frame)
            == std::mem::offset_of!(ElementVisualState, frame)
    );
    assert!(
        std::mem::offset_of!(STATE, image) + std::mem::offset_of!(state__bindgen_ty_1, farray)
            == std::mem::offset_of!(ElementVisualState, farray)
    );
};
pub type ElementProcessFunc = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type ElementCollisionFunc = ::std::option::Option<
    unsafe extern "C" fn(
        ElementPtr0: *mut ELEMENT,
        pPt0: *mut POINT,
        ElementPtr1: *mut ELEMENT,
        pPt1: *mut POINT,
    ),
>;
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
pub type PREPROCESS_FUNC = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type POSTPROCESS_FUNC = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type INIT_WEAPON_FUNC = ::std::option::Option<
    unsafe extern "C" fn(ElementPtr: *mut ELEMENT, Weapon: *mut HELEMENT) -> COUNT,
>;
pub type UNINIT_FUNC = ::std::option::Option<unsafe extern "C" fn(pRaceDesc: *mut RACE_DESC)>;
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
