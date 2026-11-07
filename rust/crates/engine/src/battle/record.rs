//! Shared battle record layouts during the storage transition.
use crate::graphics::native_abi::{
    BYTE, COUNT, FRAME, POINT, RESOURCE, SIZE, STRING, STRING as SOUND, UWORD,
    UWORD as ELEMENT_FLAGS, UWORD as STATUS_FLAGS,
};
pub use crate::math::velocity::VelocityDesc as VELOCITY_DESC;
#[cfg(test)]
use crate::World as WorldOwner;
use std::os::raw::c_uint as SPECIES_ID;

pub use crate::graphics::native_abi::{INTERSECT_CONTROL, PRIMITIVE};

pub use abi::{
    captain_stuff, state, state__bindgen_ty_1, tfb_soundsample, ElementRecord, ShipRecord,
    TFB_SoundSample, CAPTAIN_STUFF, CHARACTERISTIC_STUFF, DATA_STUFF, EVALUATE_DESC, FLEET_STUFF,
    INTEL_STUFF, MAX_DISPLAY_PRIMS, MOVEMENT_STATE, MUSIC_REF, PRIM_DESC, SHIP_INFO, STATE,
};

#[allow(non_camel_case_types, non_snake_case)]
mod abi {
    use super::*;
    pub use crate::graphics::native_abi::PRIM_DESC;

    #[repr(C)]
    #[derive(Debug, Clone)]
    pub struct state {
        pub location: POINT,
        pub image: state__bindgen_ty_1,
    }
    #[repr(C)]
    #[derive(Debug, Clone)]
    pub struct state__bindgen_ty_1 {
        pub frame: crate::battle::frames::FrameSelection,
        pub frames: crate::battle::frames::Frames,
    }
    pub type STATE = state;
    #[repr(C)]
    #[derive(Debug, Clone)]
    pub struct ElementRecord {
        pub preprocess_func: Option<crate::battle::hooks::ElementHook>,
        pub postprocess_func: Option<crate::battle::hooks::ElementHook>,
        pub collision_func: Option<crate::battle::CollisionHook>,
        pub death_func: Option<crate::battle::ElementHook>,
        pub playerNr: SIZE,
        pub state_flags: ELEMENT_FLAGS,
        pub life: COUNT,
        pub hits: COUNT,
        pub mass: BYTE,
        pub turn: BYTE,
        pub thrust: BYTE,
        pub colorCycleIndex: BYTE,
        pub velocity: VELOCITY_DESC,
        pub IntersectControl: INTERSECT_CONTROL,
        pub PrimIndex: COUNT,
        pub current: STATE,
        pub next: STATE,
        pub parent: crate::battle::Parent,
        pub target: Option<crate::battle::ElementId>,
        pub intersect_frame: crate::battle::frames::FrameSelection,
    }

    #[repr(C)]
    #[derive(Debug, Default)]
    pub struct ShipRecord {
        pub SpeciesID: SPECIES_ID,
        pub captains_name_index: BYTE,
        pub descriptor: Option<Box<RaceDescriptor>>,
        pub crew_level: COUNT,
        pub max_crew: COUNT,
        pub ship_cost: BYTE,
        pub index: COUNT,
        pub(in crate::battle) picker_resources:
            Option<crate::battle::link_queue::ShipPickerResources>,
        pub weapon_counter: BYTE,
        pub special_counter: BYTE,
        pub energy_counter: BYTE,
        pub ship_input_state: BYTE,
        pub cur_status_flags: STATUS_FLAGS,
        pub old_status_flags: STATUS_FLAGS,
        pub hShip: Option<crate::battle::ElementId>,
        pub ship_facing: COUNT,
        pub playerNr: SIZE,
        pub control: BYTE,
    }

    impl ShipRecord {
        pub fn descriptor_ptr(&mut self) -> *mut RaceDescriptor {
            let descriptor = self.descriptor.as_deref_mut();
            descriptor.map_or(ptr::null_mut(), ptr::from_mut)
        }
    }
    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct tfb_soundsample {
        _unused: [u8; 0],
    }

    pub type TFB_SoundSample = tfb_soundsample;

    pub type MUSIC_REF = *mut *mut TFB_SoundSample;

    #[repr(C)]
    #[derive(Debug, Clone)]
    pub struct captain_stuff {
        pub captain_rsc: RESOURCE,
        pub background: FRAME,
        pub turn: FRAME,
        pub thrust: FRAME,
        pub weapon: FRAME,
        pub special: crate::battle::frames::FrameSelection,
    }

    pub type CAPTAIN_STUFF = captain_stuff;

    pub type MOVEMENT_STATE = ::std::os::raw::c_uint;

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct EVALUATE_DESC {
        pub ObjectPtr: *mut ElementRecord,
        pub facing: COUNT,
        pub which_turn: COUNT,
        pub MoveState: MOVEMENT_STATE,
    }

    #[repr(C)]
    #[derive(Debug, Copy, Clone)]
    pub struct INTEL_STUFF {
        pub ManeuverabilityIndex: COUNT,
        pub WeaponRange: COUNT,
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
    #[derive(Debug)]
    pub struct DATA_STUFF {
        pub ship_rsc: [RESOURCE; 3usize],
        pub weapon_rsc: [RESOURCE; 3usize],
        pub special_rsc: [RESOURCE; 3usize],
        pub captain_control: CAPTAIN_STUFF,
        pub victory_ditty_rsc: RESOURCE,
        pub ship_sounds_rsc: RESOURCE,
        pub ship: crate::battle::frames::FrameArray,
        pub weapon: crate::battle::frames::FrameArray,
        pub special: crate::battle::frames::FrameArray,
        pub victory_ditty: MUSIC_REF,
        pub ship_sounds: SOUND,
    }

    pub const MAX_DISPLAY_PRIMS: u32 = 330;
}

impl Default for ElementRecord {
    fn default() -> Self {
        let visual = STATE {
            location: POINT { x: 0, y: 0 },
            image: state__bindgen_ty_1 {
                frame: FrameSelection::ABSENT,
                frames: crate::battle::frames::Frames::ABSENT,
            },
        };
        Self {
            preprocess_func: None,
            postprocess_func: None,
            collision_func: None,
            death_func: None,
            playerNr: 0,
            state_flags: 0,
            life: 0,
            hits: 0,
            mass: 0,
            turn: 0,
            thrust: 0,
            colorCycleIndex: 0,
            velocity: VELOCITY_DESC::new(),
            IntersectControl: INTERSECT_CONTROL {
                IntersectStamp: crate::graphics::native_abi::STAMP {
                    origin: POINT { x: 0, y: 0 },
                    frame: ptr::null_mut(),
                },
                EndPoint: POINT { x: 0, y: 0 },
                last_time_val: 0,
            },
            PrimIndex: 0,
            current: visual.clone(),
            next: visual,
            parent: Parent::None,
            target: None,
            intersect_frame: FrameSelection::ABSENT,
        }
    }
}

impl ElementRecord {
    pub fn life_span(&self) -> u16 {
        self.life
    }
    pub fn set_life_span(&mut self, value: u16) {
        self.life = value;
    }
    pub fn hit_points(&self) -> u16 {
        self.hits
    }
    pub fn set_hit_points(&mut self, value: u16) {
        self.hits = value;
    }
    pub fn mass_points(&self) -> u8 {
        self.mass
    }
    pub fn set_mass_points(&mut self, value: u8) {
        self.mass = value;
    }
    pub fn turn_wait(&self) -> u8 {
        self.turn
    }
    pub fn set_turn_wait(&mut self, value: u8) {
        self.turn = value;
    }
    pub fn thrust_wait(&self) -> u8 {
        self.thrust
    }
    pub fn set_thrust_wait(&mut self, value: u8) {
        self.thrust = value;
    }
}

impl ElementState {
    pub(super) fn decode(e: &ElementRecord) -> Self {
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
                    frame: e.current.image.frame.clone(),
                    frames: e.current.image.frames.clone(),
                },
            },
            next: Visual {
                location: e.next.location,
                image: Image {
                    frame: e.next.image.frame.clone(),
                    frames: e.next.image.frames.clone(),
                },
            },
            pre: e.preprocess_func,
            post: e.postprocess_func,
            collide: e.collision_func,
            death: e.death_func,
        }
    }
    pub(super) fn apply(e: &mut ElementRecord, value: Self) {
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
        e.current.image.frame = value.current.image.frame;
        e.current.image.frames = value.current.image.frames;
        e.next.image.frame = value.next.image.frame;
        e.next.image.frames = value.next.image.frames;
        e.preprocess_func = value.pre;
        e.postprocess_func = value.post;
        e.collision_func = value.collide;
        e.death_func = value.death;
    }
}

impl ShipState {
    pub(super) fn decode(s: &ShipRecord) -> Self {
        Self {
            facing: s.ship_facing,
            control: s.control,
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
    fn apply(s: &mut ShipRecord, v: Self) {
        s.ship_facing = v.facing;
        s.control = v.control;
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
    pub(super) fn decode(d: &RaceDescriptor) -> Self {
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
    pub(in crate::battle) fn apply(d: &mut RaceDescriptor, v: Self) {
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

use crate::battle::views::*;
use crate::battle::{Cx, ElementId, Parent, ShipId};

impl Cx<'_> {
    pub fn ship_pointer(&self, id: ShipId) -> *mut ShipRecord {
        std::ptr::from_ref(self.ship(id)).cast_mut()
    }
    pub fn ship(&self, id: ShipId) -> &ShipRecord {
        let pool = &self.kernel.races[usize::from(id.side)];
        let key = pool
            .pool
            .id_at(usize::from(id.slot))
            .unwrap_or_else(|| panic!("invalid ship {id:?}"));
        assert_eq!(id.owner, pool.pool.storage_identity(), "foreign ship owner");
        assert_eq!(id.gen, key.generation(), "stale ship {id:?}");
        pool.pool.get(key)
    }
    pub(super) fn ship_mut(&mut self, id: ShipId) -> &mut ShipRecord {
        let pool = &mut self.kernel.races[usize::from(id.side)];
        let key = pool
            .pool
            .id_at(usize::from(id.slot))
            .unwrap_or_else(|| panic!("invalid ship {id:?}"));
        assert_eq!(id.owner, pool.pool.storage_identity(), "foreign ship owner");
        assert_eq!(id.gen, key.generation(), "stale ship {id:?}");
        pool.pool.get_mut(key)
    }
    pub fn desc(&self, id: ShipId) -> &RaceDescriptor {
        self.ship(id)
            .descriptor
            .as_deref()
            .unwrap_or_else(|| panic!("ship {id:?} has no descriptor"))
    }
    pub(super) fn desc_mut(&mut self, id: ShipId) -> &mut RaceDescriptor {
        self.ship_mut(id)
            .descriptor
            .as_deref_mut()
            .unwrap_or_else(|| panic!("ship {id:?} has no descriptor"))
    }
    #[cfg(not(test))]
    pub fn captain_controls(&self, id: ShipId) -> &CAPTAIN_STUFF {
        &self.desc(id).ship_data.captain_control
    }
    /// Return the checked legacy address; dereferencing it requires the owning game-thread boundary.
    pub fn legacy_element(&self, id: ElementId) -> *mut ElementRecord {
        self.kernel.elements.raw(id).cast()
    }
    #[cfg(not(test))]
    pub(super) fn primitive(&mut self, id: ElementId) -> &mut crate::battle::DisplayPrimitive {
        let index = usize::from(self.kernel.elements[id].PrimIndex);
        assert!(index < 330, "invalid primitive index {index}");
        // SAFETY: the display authority remains live for the context's lifetime.
        self.kernel.display.get_mut(index as u16)
    }
}

impl Cx<'_> {
    pub fn element_mut(&mut self, id: ElementId) -> Edit<'_, ElementState, ElementRecord> {
        let value = self.element(id);
        Edit {
            value,
            record: self.kernel.elements.record_mut(id),
            commit: ElementState::apply,
        }
    }
}

impl Cx<'_> {
    pub fn ship_state_mut(&mut self, id: ShipId) -> Edit<'_, ShipState, ShipRecord> {
        let value = self.ship_state(id);
        Edit {
            value,
            record: self.ship_mut(id),
            commit: ShipState::apply,
        }
    }
}

impl Cx<'_> {
    pub fn race_stats_mut(&mut self, id: ShipId) -> Edit<'_, RaceStats, RaceDescriptor> {
        let value = self.race_stats(id);
        Edit {
            value,
            record: self.desc_mut(id),
            commit: RaceStats::apply,
        }
    }
}

#[derive(Debug)]
pub struct RaceDesc {
    pub(super) raw: Box<RaceDescriptor>,
}
impl RaceDesc {
    pub(super) fn empty() -> Self {
        Self::from_storage(Box::new(RaceDescriptor::EMPTY))
    }
    pub(super) fn from_storage(raw: Box<RaceDescriptor>) -> Self {
        Self { raw }
    }
    #[cfg(not(test))]
    pub(super) fn into_storage(self) -> Box<RaceDescriptor> {
        self.raw
    }
    pub fn info(&self) -> &SHIP_INFO {
        &self.raw.ship_info
    }
    pub fn race(&self) -> Option<&'static crate::battle::Race> {
        self.raw.race
    }
    pub fn set_race(&mut self, race: &'static crate::battle::Race) {
        self.raw.race = Some(race);
    }
    pub fn callbacks(&self) -> [usize; 5] {
        self.raw.callbacks()
    }
    #[cfg(not(test))]
    pub fn fleet(&self) -> &FLEET_STUFF {
        &self.raw.fleet
    }
    pub fn address(&self) -> usize {
        std::ptr::from_ref(&*self.raw) as usize
    }
    #[cfg(not(test))]
    pub fn cleanup(&mut self, world: &mut crate::battle::World) {
        self.raw.cleanup(world);
    }
    /// Extract the legacy record; dereferencing its resource pointers remains unsafe.
    pub fn into_legacy(self) -> RaceDescriptor {
        *self.raw
    }
}

impl RaceDescriptor {
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
                    .resolve(race.init_weapon)
                    .map_or(0, |f| f as usize),
                race.intelligence.map_or(0, |f| f as usize),
            ]
        } else {
            [0; 5]
        }
    }
}

use std::ptr;

impl ElementState {
    fn encode(self) -> ElementRecord {
        ElementRecord {
            preprocess_func: self.pre,
            postprocess_func: self.post,
            collision_func: self.collide,
            death_func: self.death,
            playerNr: self.player,
            state_flags: self.flags,
            life: self.life,
            hits: self.hits,
            mass: self.mass,
            turn: self.turn,
            thrust: self.thrust,
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
                    frame: self.current.image.frame,
                    frames: self.current.image.frames,
                },
            },
            next: STATE {
                location: self.next.location,
                image: state__bindgen_ty_1 {
                    frame: self.next.image.frame,
                    frames: self.next.image.frames,
                },
            },
            parent: Parent::None,
            target: None,
            intersect_frame: FrameSelection::ABSENT,
        }
    }
}

impl Subject {
    pub(super) fn decode(record: &ElementRecord) -> Self {
        Self {
            state: ElementState::decode(record),
            target: record.target,
            parent: record.parent,
            intersect: record.IntersectControl,
            primitive: record.PrimIndex,
            intersect_frame: FrameSelection::retain(record.IntersectControl.IntersectStamp.frame),
        }
    }
}

impl From<Subject> for ElementRecord {
    fn from(subject: Subject) -> Self {
        let mut record = subject.state.encode();
        record.target = subject.target;
        record.parent = subject.parent;
        record.IntersectControl = subject.intersect;
        record.intersect_frame = subject.intersect_frame;
        record.PrimIndex = subject.primitive;
        record
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    #[test]
    fn ship_projection_retains_its_address_and_rejects_reused_generations() {
        use crate::battle::link_queue::{LinkQueue, Starship};
        let mut battle = crate::battle::Battle::new();
        let mut kernel = crate::battle::record::Kernel::new();
        kernel.elements.reset_capacity(1);
        kernel.races[0].reset_capacity(1);
        let (id, pointer) = {
            let mut ships = LinkQueue::<Starship>::new(&mut kernel.races[0]);
            let link = ships.alloc().unwrap();
            (ships.ship_id(link), ships.raw(link).cast::<ShipRecord>())
        };
        // SAFETY: this test retains both local world authorities for the context's lifetime.
        let mut world = { WorldOwner::new() };
        let cx = Cx {
            b: &mut battle,
            world: &mut world,
            kernel: &mut kernel,
            rng: || 0,
        };
        assert_eq!(cx.ship_pointer(id), pointer);
        let replacement = {
            let mut ships = LinkQueue::<Starship>::new(&mut cx.kernel.races[0]);
            let link = ships.link(id);
            ships.free(link);
            let next = ships.alloc().unwrap();
            assert_eq!(ships.raw(next).cast::<ShipRecord>(), pointer);
            ships.ship_id(next)
        };
        assert_ne!(replacement.gen, id.gen);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cx.ship_pointer(id))).is_err()
        );
        assert_eq!(cx.ship_pointer(replacement), pointer);
    }
    #[test]
    fn ip_group_parent_keeps_identity_across_slot_reuse() {
        let mut world = WorldOwner::new();
        let original = world.ip_groups().alloc().unwrap();
        let parent = Parent::IpGroup(original);
        world.ip_groups().free(original);
        let replacement = world.ip_groups().alloc().unwrap();
        assert_eq!(world.ip_groups_ref().slot(replacement), 0);
        assert_ne!(
            Parent::IpGroup(replacement),
            parent,
            "a reused slot has a new parent identity"
        );
    }

    #[test]
    #[serial_test::serial]
    fn pointer_free_values_roundtrip_record_scalars_and_subject_metadata() {
        let mut raw = ElementRecord {
            state_flags: 513,
            playerNr: 1,
            colorCycleIndex: 9,
            PrimIndex: 17,
            ..ElementRecord::default()
        };
        raw.set_life_span(21);
        raw.set_hit_points(7);
        raw.set_mass_points(5);
        raw.set_turn_wait(12);
        raw.set_thrust_wait(13);
        raw.current.location = POINT { x: 11, y: 12 };
        raw.next.location = POINT { x: 21, y: 22 };
        raw.IntersectControl.last_time_val = 31;
        let mut subject = Subject::decode(&raw);
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
        let mut world = WorldOwner::new();
        let group = world.ip_groups().alloc().unwrap();
        subject.parent = Parent::IpGroup(group);
        let encoded = ElementRecord::from(subject);
        assert_eq!(Subject::decode(&encoded).parent, Parent::IpGroup(group));
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
        assert_eq!(
            (raw.life, raw.hits, raw.mass, raw.turn, raw.thrust),
            (55, 2, 6, 14, 15)
        );
    }
    #[test]
    fn ship_and_descriptor_values_preserve_resources_while_updating_stats() {
        // SAFETY: zero is valid for nullable legacy ship fields and scalars.
        let mut ship: ShipRecord = unsafe { std::mem::zeroed() };
        let mut state = ShipState::decode(&ship);
        state.facing = 7;
        state.energy_counter = 9;
        state.weapon_counter = 4;
        ShipState::apply(&mut ship, state);
        assert_eq!(ship.ship_facing, 7);
        assert_eq!(ship.energy_counter, 9);
        assert_eq!(ship.weapon_counter, 4);
        let mut descriptor = RaceDescriptor::EMPTY;
        descriptor.ship_data.ship_rsc[0] = c"retained".as_ptr();
        let mut stats = RaceStats::decode(&descriptor);
        stats.ship_info.energy_level = 10;
        stats.intelligence.weapon_range = 231;
        RaceStats::apply(&mut descriptor, stats);
        assert_eq!(descriptor.ship_info.energy_level, 10);
        assert_eq!(descriptor.cyborg_control.WeaponRange, 231);
        assert_eq!(descriptor.ship_data.ship_rsc[0], c"retained".as_ptr());
        let value = RaceDesc::from_storage(Box::new(descriptor));
        // SAFETY: this value retains only test-local static resource tokens.
        let raw = value.into_legacy();
        assert_eq!(raw.ship_info.energy_level, 10);
    }
}

#[cfg(not(test))]
impl RaceDescriptor {
    pub fn cleanup(&mut self, world: &mut crate::battle::World) {
        if let Some(uninit) = self.race.and_then(|race| race.uninit) {
            let mut stats = RaceStats::decode(self);
            uninit(world, &mut stats);
            RaceStats::apply(self, stats);
        }
    }
}

#[cfg(test)]
mod zero_descriptor_tests {
    #[test]
    fn zero_descriptor_has_no_native_race_state() {
        // SAFETY: a zeroed legacy descriptor contains nullable callbacks, pointers, and scalar state.
        let descriptor: super::RaceDescriptor = unsafe { std::mem::zeroed() };
        assert!(matches!(
            descriptor.race_state,
            crate::battle::RaceState::None
        ));
    }
}

#[repr(C)]
#[repr(align(16))]
#[derive(Debug)]
pub struct RaceDescriptor {
    pub assets: Option<Box<super::ship_assets::ShipAssets>>,
    pub loaded_victory: Option<crate::sound::resource_types::LoadedMusic>,
    pub loaded_sounds: Option<crate::sound::owned_bank::LoadedSoundBank>,
    pub resources: super::template::RaceResources,
    pub ship_info: SHIP_INFO,
    pub fleet: FLEET_STUFF,
    pub __bindgen_padding_0: [u16; 5usize],
    pub characteristics: CHARACTERISTIC_STUFF,
    pub ship_data: DATA_STUFF,
    pub cyborg_control: INTEL_STUFF,
    pub race: Option<&'static crate::battle::race::Race>,
    pub race_state: crate::battle::race::RaceState,
    pub race_callbacks: crate::battle::race::Callbacks,
}

pub use crate::kernel::Kernel;

#[cfg(test)]
mod kernel_tests {
    use super::*;
    #[test]
    fn loading_during_a_callback_retains_its_ship_through_battle_cleanup() {
        let mut kernel = Kernel::new();
        kernel.races[0].reset_capacity(1);
        let ship = kernel.races[0].pool.alloc(ShipRecord::default()).unwrap();
        kernel.races[0].pool.get_mut(ship).special_counter = 7;
        kernel.races[0].pool.put(ship);
        kernel.begin_battle();
        kernel.request_ship_reset();
        assert!(kernel.races[0].pool.head().is_none());
        assert_eq!(kernel.races[0].pool.get(ship).special_counter, 7);
        kernel.races[0].pool.get_mut(ship).special_counter = 10;
        kernel.finish_battle();
        assert_eq!(kernel.races[0].pool.get(ship).special_counter, 10);
        kernel.complete_ship_reset();
        assert!(!kernel.races[0].pool.contains(ship));
        let replacement = kernel.races[0].pool.alloc(ShipRecord::default()).unwrap();
        assert_eq!(replacement.index(), ship.index());
        assert_ne!(replacement.generation(), ship.generation());
        assert_eq!(kernel.races[0].pool.get(replacement).special_counter, 0);
    }
    #[test]
    fn separate_lifecycles_reset_state_and_keep_parallax_backing_stable_when_moved() {
        let mut kernel = Kernel::new();
        let address = kernel.star_planes[0].points.as_ptr();
        kernel.instant_victory = 1;
        kernel.status_offsets = [7, 11];
        let moved = Box::new(kernel);
        assert_eq!(moved.star_planes[0].points.as_ptr(), address);
        assert_eq!(
            moved.star_planes.each_ref().map(|p| p.points.len()),
            [30, 60, 90]
        );
        assert_eq!(moved.status_offsets, [7, 11]);
        let fresh = Kernel::new();
        assert_eq!(fresh.instant_victory, 0);
        assert!(fresh.battle_music.is_none());
        assert_eq!(fresh.ditty_playing, 0);
        assert_eq!(fresh.space_origin.x, 0);
        assert_eq!(fresh.space_origin.y, 0);
        assert_eq!(fresh.status_offsets, [0; 2]);
        assert!(fresh.player_input.iter().all(Option::is_none));
        assert!(fresh.menu_sounds.is_null());
        assert!(fresh.game_sounds.is_none());
        assert!(fresh.winner.is_none());
        assert_eq!(fresh.sounds.count, 0);
        assert_eq!(fresh.channels.borrow().lru, [0, 1, 2, 3]);
    }
}

#[cfg(not(test))]
impl RaceDesc {
    pub fn recover_crew(&mut self, crew: u16) -> u16 {
        let info = &mut self.raw.ship_info;
        recover_ship_crew(&mut info.crew_level, info.max_crew, crew)
    }
}

#[cfg(any(test, feature = "test-support"))]
impl RaceDesc {
    pub fn fixture(raw: Box<RaceDescriptor>) -> Self {
        Self::from_storage(raw)
    }
}

pub fn recover_ship_crew(remaining: &mut u16, maximum: u16, crew: u16) -> u16 {
    let mut value = *remaining;
    if value != 0 {
        value = if i32::from(crew) >= i32::from(maximum) - i32::from(value) {
            maximum
        } else {
            value.wrapping_add(crew)
        };
    }
    *remaining = value;
    value
}

impl Kernel {
    #[cfg(not(test))]
    pub fn reset_race_capacities(&mut self, capacity: usize) {
        for pool in &mut self.races {
            pool.reset_capacity(capacity);
        }
    }
}

impl Kernel {
    pub fn shared_frame_source(
        &self,
        role: super::views::SharedFrames,
    ) -> &[crate::graphics::native_abi::FRAME; 3] {
        match role {
            super::views::SharedFrames::Asteroid => &self.asteroid,
            super::views::SharedFrames::Explosion => &self.explosion,
            _ => panic!("not a battle shared frame role {role:?}"),
        }
    }
}

impl Kernel {
    #[cfg(not(test))]
    pub fn element_sound_source(&self, token: usize) -> Option<crate::battle::ElementId> {
        self.elements.sound_source(token)
    }
}
