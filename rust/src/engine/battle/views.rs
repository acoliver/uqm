//! Pointer-free Native values. Legacy record decoding stays in accessors.
use super::record::CHARACTERISTIC_STUFF;
use super::record::VELOCITY_DESC as VelocityDesc;
use super::{CollisionHook, ElementHook};
use crate::graphics::native_abi::{FRAME, POINT};

#[derive(Clone, Copy, Debug)]
pub struct Laser {
    pub origin: POINT,
    pub end: POINT,
    pub flags: u16,
    pub sender: i16,
    pub pixoffs: i16,
    pub face: u16,
    pub color: crate::graphics::native_abi::Color,
}

#[derive(Clone, Copy, Debug)]
pub struct FlagshipConfig {
    pub modules: [u8; 16],
    pub drives: [u8; 11],
    pub jets: [u8; 8],
    pub crew: u16,
    pub fuel: u32,
    pub position: (i32, i32),
    pub hyperspace: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame(pub(super) FRAME);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frames(pub(super) *mut FRAME);
impl Frame {
    pub const ABSENT: Self = Self(std::ptr::null_mut());
    pub fn is_absent(self) -> bool {
        self.0.is_null()
    }
}
impl Frame {
    pub(crate) fn raw(self) -> FRAME {
        self.0
    }
}
impl Frames {
    pub(crate) fn raw(self) -> *mut FRAME {
        self.0
    }
}
impl Frame {
    pub(crate) unsafe fn from_native(frame: FRAME) -> Self {
        Self(frame)
    }
}
impl Frames {
    pub(crate) unsafe fn from_native(frames: std::ptr::NonNull<FRAME>) -> Self {
        Self(frames.as_ptr())
    }
}

#[derive(Clone, Copy)]
pub struct Image {
    pub frame: Frame,
    pub frames: Frames,
}
#[derive(Clone, Copy)]
pub struct Visual {
    pub location: POINT,
    pub image: Image,
}
#[derive(Clone, Copy)]
pub struct ElementState {
    pub player: i16,
    pub flags: u16,
    pub life: u16,
    pub hits: u16,
    pub mass: u8,
    pub turn: u8,
    pub thrust: u8,
    pub color_cycle: u8,
    pub velocity: VelocityDesc,
    pub current: Visual,
    pub next: Visual,
    pub pre: Option<ElementHook>,
    pub post: Option<ElementHook>,
    pub collide: Option<CollisionHook>,
    pub death: Option<ElementHook>,
}
impl ElementState {
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
#[derive(Clone, Copy, Debug)]
pub struct ShipState {
    pub facing: u16,
    pub control: u8,
    pub player: i16,
    pub input: u8,
    pub status: u16,
    pub old_status: u16,
    pub weapon_counter: u8,
    pub special_counter: u8,
    pub energy_counter: u8,
    pub crew: u16,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Info {
    pub ship_flags: u16,
    pub ship_cost: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Intelligence {
    pub maneuverability: u16,
    pub weapon_range: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct RaceStats {
    pub ship_info: Info,
    pub characteristics: CHARACTERISTIC_STUFF,
    pub intelligence: Intelligence,
    pub state: super::RaceState,
}

pub struct Edit<'a, T: Copy, R> {
    pub(super) value: T,
    pub(super) record: &'a mut R,
    pub(super) commit: fn(&mut R, T),
}
impl<T: Copy, R> std::ops::Deref for Edit<'_, T, R> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}
impl<T: Copy, R> std::ops::DerefMut for Edit<'_, T, R> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
impl<T: Copy, R> Drop for Edit<'_, T, R> {
    fn drop(&mut self) {
        (self.commit)(self.record, self.value);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Missile {
    pub cx: i16,
    pub cy: i16,
    pub flags: u16,
    pub sender: i16,
    pub pixoffs: i16,
    pub speed: i16,
    pub hit_points: i16,
    pub damage: i16,
    pub face: u16,
    pub index: u16,
    pub life: u16,
    pub farray: Frames,
    pub preprocess: Option<super::ElementFn>,
    pub blast_offs: i16,
}

#[cfg(not(test))]
#[derive(Clone, Copy)]
pub enum SharedFrames {
    Asteroid,
    Explosion,
}

use super::ElementId;
#[derive(Clone, Copy)]
pub struct Subject {
    pub state: ElementState,
    pub target: Option<ElementId>,
    pub parent: super::Parent,
    pub(super) intersect: super::record::INTERSECT_CONTROL,
    pub(super) primitive: u16,
    pub(super) predecessor: Option<ElementId>,
    pub(super) successor: Option<ElementId>,
}
impl std::ops::Deref for Subject {
    type Target = ElementState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}
impl std::ops::DerefMut for Subject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

#[derive(Clone, Copy)]
pub struct PrimitiveStyle {
    pub kind: u8,
    pub color: crate::graphics::native_abi::Color,
}

#[cfg(not(test))]
impl Subject {
    pub(crate) fn intersection(&self) -> crate::graphics::native_abi::INTERSECT_CONTROL {
        self.intersect
    }
}
