//! Pointer-free Native values. Legacy record decoding stays in accessors.
use super::record::CHARACTERISTIC_STUFF;
use super::record::VELOCITY_DESC as VelocityDesc;
use super::{CollisionHook, ElementHook};
use crate::graphics::native_abi::{FRAME, POINT};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frame(pub(crate) FRAME);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Frames(pub(crate) *mut FRAME);
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
    pub player: i16,
    pub input: u8,
    pub status: u16,
    pub old_status: u16,
    pub weapon_counter: u8,
    pub special_counter: u8,
    pub energy_counter: u8,
    pub crew: u16,
}
#[derive(Clone, Copy, Debug)]
pub struct Info {
    pub ship_flags: u16,
    pub ship_cost: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
}
#[derive(Clone, Copy, Debug)]
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
    pub(crate) value: T,
    pub(crate) record: &'a mut R,
    pub(crate) commit: fn(&mut R, T),
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
