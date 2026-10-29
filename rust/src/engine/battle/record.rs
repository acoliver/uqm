//! Shared battle record layouts during the storage transition.
pub use crate::battle::velocity::VelocityDesc as VELOCITY_DESC;
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
    pub use super::RaceDescriptor as race_desc;

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

impl ELEMENT {
    pub fn life_span(&self) -> u16 {
        // SAFETY: every member of this legacy union is a u16.
        unsafe { self.__bindgen_anon_1.life_span }
    }
    pub fn set_life_span(&mut self, value: u16) {
        self.__bindgen_anon_1.life_span = value;
    }
    pub fn hit_points(&self) -> u16 {
        // SAFETY: every member of this legacy union is a u16.
        unsafe { self.__bindgen_anon_2.hit_points }
    }
    pub fn set_hit_points(&mut self, value: u16) {
        self.__bindgen_anon_2.hit_points = value;
    }
    pub fn mass_points(&self) -> u8 {
        // SAFETY: the legacy union contains only a u8.
        unsafe { self.__bindgen_anon_3.mass_points }
    }
    pub fn set_mass_points(&mut self, value: u8) {
        self.__bindgen_anon_3.mass_points = value;
    }
    pub fn turn_wait(&self) -> u8 {
        // SAFETY: every member of this legacy union is a u8.
        unsafe { self.__bindgen_anon_4.turn_wait }
    }
    pub fn set_turn_wait(&mut self, value: u8) {
        self.__bindgen_anon_4.turn_wait = value;
    }
    pub fn thrust_wait(&self) -> u8 {
        // SAFETY: every member of this legacy union is a u8.
        unsafe { self.__bindgen_anon_5.thrust_wait }
    }
    pub fn set_thrust_wait(&mut self, value: u8) {
        self.__bindgen_anon_5.thrust_wait = value;
    }
}

impl ElementState {
    pub(super) fn decode(e: &ELEMENT) -> Self {
        Self {
            player: e.playerNr,
            flags: e.state_flags,
            life: e.life_span(),
            hits: e.hit_points(),
            mass: e.mass_points(),
            turn: e.turn_wait(),
            thrust: e.thrust_wait(),
            color_cycle: e.colorCycleIndex,
            velocity: e.velocity,
            current: Visual {
                location: e.current.location,
                image: Image {
                    frame: Frame(e.current.image.frame),
                    frames: Frames(e.current.image.farray),
                },
            },
            next: Visual {
                location: e.next.location,
                image: Image {
                    frame: Frame(e.next.image.frame),
                    frames: Frames(e.next.image.farray),
                },
            },
            pre: e.preprocess_func,
            post: e.postprocess_func,
            collide: e.collision_func,
            death: e.death_func,
        }
    }
    pub(super) fn apply(e: &mut ELEMENT, value: Self) {
        e.playerNr = value.player;
        e.state_flags = value.flags;
        e.set_life_span(value.life);
        e.set_hit_points(value.hits);
        e.set_mass_points(value.mass);
        e.set_turn_wait(value.turn);
        e.set_thrust_wait(value.thrust);
        e.colorCycleIndex = value.color_cycle;
        e.velocity = value.velocity;
        e.current.location = value.current.location;
        e.next.location = value.next.location;
        e.current.image.frame = value.current.image.frame.0;
        e.current.image.farray = value.current.image.frames.0;
        e.next.image.frame = value.next.image.frame.0;
        e.next.image.farray = value.next.image.frames.0;
        e.preprocess_func = value.pre;
        e.postprocess_func = value.post;
        e.collision_func = value.collide;
        e.death_func = value.death;
    }
}

impl ShipState {
    pub(super) fn decode(s: &STARSHIP) -> Self {
        Self {
            facing: s.ShipFacing,
            player: s.playerNr,
            input: s.ship_input_state,
            status: s.cur_status_flags,
            old_status: s.old_status_flags,
            weapon_counter: s.weapon_counter,
            special_counter: s.special_counter,
            energy_counter: s.energy_counter,
            crew: s.crew_level,
        }
    }
    fn apply(s: &mut STARSHIP, v: Self) {
        s.ShipFacing = v.facing;
        s.playerNr = v.player;
        s.ship_input_state = v.input;
        s.cur_status_flags = v.status;
        s.old_status_flags = v.old_status;
        s.weapon_counter = v.weapon_counter;
        s.special_counter = v.special_counter;
        s.energy_counter = v.energy_counter;
        s.crew_level = v.crew;
    }
}

impl RaceStats {
    pub(super) fn decode(d: &RACE_DESC) -> Self {
        let i = d.ship_info;
        Self {
            ship_info: Info {
                ship_flags: i.ship_flags,
                ship_cost: i.ship_cost,
                crew_level: i.crew_level,
                max_crew: i.max_crew,
                energy_level: i.energy_level,
                max_energy: i.max_energy,
            },
            characteristics: d.characteristics,
            intelligence: Intelligence {
                maneuverability: d.cyborg_control.ManeuverabilityIndex,
                weapon_range: d.cyborg_control.WeaponRange,
            },
            state: d.race_state,
        }
    }
    pub(super) fn apply(d: &mut RACE_DESC, v: Self) {
        d.ship_info.ship_flags = v.ship_info.ship_flags;
        d.ship_info.ship_cost = v.ship_info.ship_cost;
        d.ship_info.crew_level = v.ship_info.crew_level;
        d.ship_info.max_crew = v.ship_info.max_crew;
        d.ship_info.energy_level = v.ship_info.energy_level;
        d.ship_info.max_energy = v.ship_info.max_energy;
        d.characteristics = v.characteristics;
        d.cyborg_control.ManeuverabilityIndex = v.intelligence.maneuverability;
        d.cyborg_control.WeaponRange = v.intelligence.weapon_range;
        d.race_state = v.state;
    }
}

use super::views::*;
use super::{Cx, ElementId, Parent, ShipId};

#[cfg(not(test))]
impl Cx<'_> {
    /// # Safety
    /// Every pointer in the supplied legacy record must belong to this context's live resources.
    pub(crate) unsafe fn legacy_subject(&self, e: &ELEMENT) -> super::Subject {
        self.decode_subject(e)
    }
}

pub(crate) type Tracker = unsafe extern "C" fn(*mut ELEMENT, *mut u16) -> i16;

impl Cx<'_> {
    pub(crate) fn ship_id(&self, address: usize) -> ShipId {
        for side in 0..2 {
            // SAFETY: queue authorities are owned by this game-thread context.
            let queue = self.project(ship_queue(side));
            let stride = usize::from(queue.object_size);
            if let Some(offset) = address.checked_sub(queue.pq_tab as usize) {
                if stride != 0
                    && offset.is_multiple_of(stride)
                    && offset / stride < usize::from(queue.num_objects)
                {
                    return ShipId {
                        side: side as u8,
                        slot: (offset / stride) as u8,
                    };
                }
            }
        }
        panic!("ship address {address:#x} is outside the ship pools")
    }
    pub(crate) fn ship_pointer(&self, id: ShipId) -> *mut STARSHIP {
        assert!(id.side < 2, "invalid ship side {id:?}");
        // SAFETY: queue storage is address-stable for the battle's lifetime.
        {
            let queue = self.project(ship_queue(usize::from(id.side)));
            assert!(
                usize::from(id.slot) < usize::from(queue.num_objects),
                "invalid ship {id:?}"
            );
            queue
                .pq_tab
                .wrapping_add(usize::from(id.slot) * usize::from(queue.object_size))
                .cast()
        }
    }
    pub(crate) fn ship(&self, id: ShipId) -> &STARSHIP {
        // SAFETY: the validated slot belongs to this context's ship authority.
        self.project(
            std::ptr::NonNull::new(self.ship_pointer(id))
                .unwrap_or_else(|| panic!("{}", "ship slot")),
        )
    }
    pub(super) fn ship_mut(&mut self, id: ShipId) -> &mut STARSHIP {
        // SAFETY: this exclusive context borrow scopes the one-record projection.
        self.project_mut(
            std::ptr::NonNull::new(self.ship_pointer(id))
                .unwrap_or_else(|| panic!("{}", "ship slot")),
        )
    }
    pub(crate) fn desc(&self, id: ShipId) -> &RACE_DESC {
        let pointer = self.ship(id).RaceDescPtr;
        assert!(!pointer.is_null(), "ship {id:?} has no descriptor");
        // SAFETY: each live ship owns its descriptor until teardown.
        self.project(std::ptr::NonNull::new(pointer).unwrap_or_else(|| panic!("{}", "descriptor")))
    }
    pub(super) fn desc_mut(&mut self, id: ShipId) -> &mut RACE_DESC {
        let pointer = self.ship(id).RaceDescPtr;
        assert!(!pointer.is_null(), "ship {id:?} has no descriptor");
        // SAFETY: the exclusive context borrow scopes the descriptor projection.
        self.project_mut(
            std::ptr::NonNull::new(pointer).unwrap_or_else(|| panic!("{}", "descriptor")),
        )
    }
    #[cfg(not(test))]
    pub(crate) fn captain_controls(&mut self, id: ShipId) -> std::ptr::NonNull<CAPTAIN_STUFF> {
        std::ptr::NonNull::from(&mut self.desc_mut(id).ship_data.captain_control)
    }
    pub(super) fn decode_subject(&self, e: &ELEMENT) -> super::Subject {
        super::Subject {
            state: ElementState::decode(e),
            target: self.b.elements.id(e.hTarget),
            parent: if e.pParent.is_null() {
                Parent::None
            } else {
                Parent::Ship(self.ship_id(e.pParent as usize))
            },
            intersect: e.IntersectControl,
            primitive: e.PrimIndex,
            predecessor: self.b.elements.id(e.pred),
            successor: self.b.elements.id(e.succ),
        }
    }
    pub(crate) fn encode_subject(&self, subject: &super::Subject) -> ELEMENT {
        let mut e = subject.state.encode();
        ElementState::apply(&mut e, subject.state);
        e.hTarget = subject
            .target
            .map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id));
        e.pParent = match subject.parent {
            Parent::None => std::ptr::null_mut(),
            Parent::Ship(id) => self.ship_pointer(id).cast(),
            other => panic!("invalid projected subject parent {other:?}"),
        };
        e.IntersectControl = subject.intersect;
        e.PrimIndex = subject.primitive;
        e.pred = subject
            .predecessor
            .map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id));
        e.succ = subject
            .successor
            .map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id));
        e
    }
    /// Return the checked legacy address; dereferencing it requires the owning game-thread boundary.
    pub fn legacy_element(&self, id: ElementId) -> *mut ELEMENT {
        self.b.elements.raw(id).cast()
    }
    #[cfg(not(test))]
    pub(super) fn primitive(&mut self, id: ElementId) -> &mut PRIMITIVE {
        let index = usize::from(self.b.elements[id].PrimIndex);
        assert!(index < 330, "invalid primitive index {index}");
        // SAFETY: the display authority remains live for the context's lifetime.
        self.project_mut(primitive_slot(index))
    }
}

impl Cx<'_> {
    pub fn element_mut(&mut self, id: ElementId) -> Edit<'_, ElementState, ELEMENT> {
        let value = self.element(id);
        Edit {
            value,
            record: &mut self.b.elements[id],
            commit: ElementState::apply,
        }
    }
}

impl Cx<'_> {
    pub fn ship_state_mut(&mut self, id: ShipId) -> Edit<'_, ShipState, STARSHIP> {
        let value = self.ship_state(id);
        Edit {
            value,
            record: self.ship_mut(id),
            commit: ShipState::apply,
        }
    }
}

impl Cx<'_> {
    pub fn race_stats_mut(&mut self, id: ShipId) -> Edit<'_, RaceStats, RACE_DESC> {
        let value = self.race_stats(id);
        Edit {
            value,
            record: self.desc_mut(id),
            commit: RaceStats::apply,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RaceDesc {
    pub(super) raw: RACE_DESC,
}
impl RaceDesc {
    pub(super) fn from_legacy(raw: RACE_DESC) -> Self {
        Self { raw }
    }
    /// Extract the legacy record; dereferencing its resource pointers remains unsafe.
    pub fn into_legacy(self) -> RACE_DESC {
        self.raw
    }
}

impl RACE_DESC {
    pub fn callbacks(&self) -> [usize; 5] {
        if let Some(race) = self.race {
            [
                race.uninit.map_or(0, |f| f as usize),
                self.race_callbacks
                    .pre
                    .resolve(race.preprocess)
                    .map_or(0, |f| f as usize),
                self.race_callbacks
                    .post
                    .resolve(race.postprocess)
                    .map_or(0, |f| f as usize),
                self.race_callbacks
                    .weapon
                    .resolve(Some(race.init_weapon))
                    .map_or(0, |f| f as usize),
                race.intelligence as usize,
            ]
        } else {
            [
                self.uninit_func.map_or(0, |f| f as usize),
                self.preprocess_func.map_or(0, |f| f as usize),
                self.postprocess_func.map_or(0, |f| f as usize),
                self.init_weapon_func.map_or(0, |f| f as usize),
                self.cyborg_control
                    .intelligence_func
                    .map_or(0, |f| f as usize),
            ]
        }
    }
}

use crate::graphics::native_abi::PRIM_LINKS;
use std::ptr;
const EMPTY_QUEUE: QUEUE = QUEUE {
    head: ptr::null_mut(),
    tail: ptr::null_mut(),
    pq_tab: ptr::null_mut(),
    free_list: ptr::null_mut(),
    object_size: 0,
    num_objects: 0,
};

#[no_mangle]
pub static mut disp_q: QUEUE = EMPTY_QUEUE;
#[no_mangle]
pub static mut race_q: [QUEUE; 2] = [EMPTY_QUEUE; 2];

#[no_mangle]
pub static mut DisplayFreeList: COUNT = 0;
#[no_mangle]
// SAFETY: Primitive fields and the PRIM_DESC union members admit all-zero bits.
pub static mut DisplayArray: [PRIMITIVE; abi::MAX_DISPLAY_PRIMS as usize] =
    unsafe { std::mem::zeroed() };
#[no_mangle]
pub static mut DisplayLinks: PRIM_LINKS = 0;

pub(super) fn ship_queue(side: usize) -> std::ptr::NonNull<QUEUE> {
    assert!(side < 2);
    std::ptr::NonNull::new((&raw mut race_q).cast::<QUEUE>().wrapping_add(side))
        .unwrap_or_else(|| panic!("{}", "static ship queues"))
}
#[cfg(not(test))]
pub(super) fn primitive_slot(index: usize) -> std::ptr::NonNull<PRIMITIVE> {
    assert!(index < 330);
    std::ptr::NonNull::new(
        (&raw mut DisplayArray)
            .cast::<PRIMITIVE>()
            .wrapping_add(index),
    )
    .unwrap_or_else(|| panic!("{}", "static primitives"))
}
impl ElementState {
    fn encode(self) -> ELEMENT {
        ELEMENT {
            pred: ptr::null_mut(),
            succ: ptr::null_mut(),
            preprocess_func: self.pre,
            postprocess_func: self.post,
            collision_func: self.collide,
            death_func: self.death,
            playerNr: self.player,
            state_flags: self.flags,
            __bindgen_anon_1: element__bindgen_ty_1 {
                life_span: self.life,
            },
            __bindgen_anon_2: element__bindgen_ty_2 {
                hit_points: self.hits,
            },
            __bindgen_anon_3: element__bindgen_ty_3 {
                mass_points: self.mass,
            },
            __bindgen_anon_4: element__bindgen_ty_4 {
                turn_wait: self.turn,
            },
            __bindgen_anon_5: element__bindgen_ty_5 {
                thrust_wait: self.thrust,
            },
            colorCycleIndex: self.color_cycle,
            velocity: self.velocity,
            IntersectControl: INTERSECT_CONTROL {
                IntersectStamp: crate::graphics::native_abi::STAMP {
                    origin: POINT { x: 0, y: 0 },
                    frame: ptr::null_mut(),
                },
                EndPoint: POINT { x: 0, y: 0 },
                last_time_val: 0,
            },
            PrimIndex: 0,
            current: STATE {
                location: self.current.location,
                image: state__bindgen_ty_1 {
                    frame: self.current.image.frame.0,
                    farray: self.current.image.frames.0,
                },
            },
            next: STATE {
                location: self.next.location,
                image: state__bindgen_ty_1 {
                    frame: self.next.image.frame.0,
                    farray: self.next.image.frames.0,
                },
            },
            pParent: ptr::null_mut(),
            hTarget: ptr::null_mut(),
        }
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn pointer_free_values_roundtrip_record_scalars_and_subject_metadata() {
        let mut queue = EMPTY_QUEUE;
        let mut groups = EMPTY_QUEUE;
        let mut activity = 0u16;
        // SAFETY: these local queue authorities and projections outlive the context.
        let mut battle = unsafe { super::super::Battle::new(&mut queue) };
        // SAFETY: the local world authorities remain live and exclusively borrowed.
        let mut world = unsafe {
            super::super::World::new(&mut activity, &mut groups, [ptr::null_mut(); 2], None)
        };
        let cx = Cx {
            b: &mut battle,
            world: &mut world,
            rng: || 0,
        };
        // SAFETY: legacy record scalar fields, nullable hooks and unions admit zero.
        let mut raw: ELEMENT = unsafe { std::mem::zeroed() };
        raw.state_flags = 513;
        raw.playerNr = 1;
        raw.colorCycleIndex = 9;
        raw.PrimIndex = 17;
        raw.set_life_span(21);
        raw.set_hit_points(7);
        raw.set_mass_points(5);
        raw.set_turn_wait(12);
        raw.set_thrust_wait(13);
        raw.current.location = POINT { x: 11, y: 12 };
        raw.next.location = POINT { x: 21, y: 22 };
        raw.IntersectControl.last_time_val = 31;
        assert!(cx.world.tracking.is_none());
        let mut subject = cx.decode_subject(&raw);
        assert_eq!(subject.life_span(), 21);
        assert_eq!(subject.hit_points(), 7);
        assert_eq!(subject.mass_points(), 5);
        assert_eq!(subject.turn_wait(), 12);
        assert_eq!(subject.thrust_wait(), 13);
        subject.set_life_span(44);
        subject.set_hit_points(8);
        subject.set_mass_points(6);
        subject.set_turn_wait(14);
        subject.set_thrust_wait(15);
        let encoded = cx.encode_subject(&subject);
        assert_eq!(encoded.PrimIndex, 17);
        assert_eq!(encoded.IntersectControl.last_time_val, 31);
        assert_eq!(encoded.current.location.x, 11);
        assert_eq!(encoded.next.location.y, 22);
        assert_eq!(encoded.state_flags, 513);
        assert_eq!(encoded.colorCycleIndex, 9);
        assert_eq!(encoded.life_span(), 44);
        assert_eq!(encoded.hit_points(), 8);
        assert_eq!(encoded.mass_points(), 6);
        assert_eq!(encoded.turn_wait(), 14);
        assert_eq!(encoded.thrust_wait(), 15);
        let mut decoded = ElementState::decode(&encoded);
        decoded.life = 55;
        {
            let mut edit = Edit {
                value: decoded,
                record: &mut raw,
                commit: ElementState::apply,
            };
            assert_eq!(edit.life, 55);
            edit.hits = 2;
        }
        assert_eq!(raw.life_span(), 55);
        assert_eq!(raw.hit_points(), 2);
    }
    #[test]
    fn ship_and_descriptor_values_preserve_resources_while_updating_stats() {
        // SAFETY: zero is valid for nullable legacy ship fields and scalars.
        let mut ship: STARSHIP = unsafe { std::mem::zeroed() };
        let mut state = ShipState::decode(&ship);
        state.facing = 7;
        state.energy_counter = 9;
        state.weapon_counter = 4;
        ShipState::apply(&mut ship, state);
        assert_eq!(ship.ShipFacing, 7);
        assert_eq!(ship.energy_counter, 9);
        assert_eq!(ship.weapon_counter, 4);
        let mut descriptor = RACE_DESC::EMPTY;
        descriptor.ship_data.ship_rsc[0] = c"retained".as_ptr();
        let mut stats = RaceStats::decode(&descriptor);
        stats.ship_info.energy_level = 10;
        stats.intelligence.weapon_range = 231;
        RaceStats::apply(&mut descriptor, stats);
        assert_eq!(descriptor.ship_info.energy_level, 10);
        assert_eq!(descriptor.cyborg_control.WeaponRange, 231);
        assert_eq!(descriptor.ship_data.ship_rsc[0], c"retained".as_ptr());
        let value = RaceDesc::from_legacy(descriptor);
        // SAFETY: this value retains only test-local static resource tokens.
        let raw = value.into_legacy();
        assert_eq!(raw.ship_info.energy_level, 10);
    }
}

#[cfg(not(test))]
impl RACE_DESC {
    pub(crate) fn cleanup(&mut self) -> abi::UNINIT_FUNC {
        if let Some(race) = self.race {
            if let Some(uninit) = race.uninit {
                uninit(&mut self.race_state);
            }
            None
        } else {
            self.uninit_func
        }
    }
}

#[cfg(not(test))]
pub(crate) type DescriptorInitializer = unsafe extern "C" fn() -> *mut RACE_DESC;

#[cfg(test)]
mod zero_descriptor_tests {
    #[test]
    fn zero_descriptor_has_no_native_race_state() {
        // SAFETY: a zeroed legacy descriptor contains nullable callbacks, pointers, and scalar state.
        let descriptor: super::RACE_DESC = unsafe { std::mem::zeroed() };
        assert!(matches!(
            descriptor.race_state,
            crate::engine::battle::RaceState::None
        ));
    }
}

impl Cx<'_> {
    /// # Safety
    /// The pointer names a live, layout-compatible legacy element on the game thread.
    pub(crate) unsafe fn legacy_parent<E>(&self, pointer: std::ptr::NonNull<E>) -> usize {
        let pointer = pointer.cast::<ELEMENT>();
        self.project(pointer).pParent as usize
    }
}

pub(super) struct CollisionOperands<E, P>(
    pub(super) *mut E,
    pub(super) *mut P,
    pub(super) *mut E,
    pub(super) *mut P,
);
pub(super) struct ElementOperand<E>(pub(super) *mut E);

#[repr(C)]
#[repr(align(16))]
#[derive(Debug, Copy, Clone)]
pub struct RaceDescriptor {
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
    pub code_ref: *mut ::std::os::raw::c_void,
    pub race: Option<&'static super::race::Race>,
    pub race_state: super::race::RaceState,
    pub race_callbacks: super::race::Callbacks,
}
