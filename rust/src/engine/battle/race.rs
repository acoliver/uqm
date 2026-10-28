//! Species behavior and value inputs for AI projections.

use super::{CollisionHook, Cx, ElementId, ShipId};

pub use super::record::RaceDesc;
pub type ShipFn = fn(&mut Cx<'_>, ShipId);
pub type WeaponFn = fn(&mut Cx<'_>, ShipId, WeaponOrigin) -> WeaponSpawn;
pub type IntelligenceFn = fn(&mut Cx<'_>, ShipId, &mut Concerns);

/// Immutable behavior shared by all ships of one converted species.
#[derive(Debug)]
pub struct Race {
    pub descriptor: fn() -> RaceDesc,
    pub preprocess: Option<ShipFn>,
    pub postprocess: Option<ShipFn>,
    pub init_weapon: WeaponFn,
    pub intelligence: IntelligenceFn,
    pub uninit: Option<fn(&mut RaceState)>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum RaceState {
    #[default]
    None = 0,
    Androsynth {
        collision: Option<CollisionHook>,
    } = 1,
}

#[derive(Clone, Copy)]
pub struct Subject {
    pub state: super::views::ElementState,
    pub target: Option<ElementId>,
    pub parent: super::Parent,
    pub(crate) intersect: super::record::INTERSECT_CONTROL,
    pub(crate) primitive: u16,
    pub(crate) predecessor: Option<ElementId>,
    pub(crate) successor: Option<ElementId>,
}
impl std::ops::Deref for Subject {
    type Target = super::views::ElementState;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}
impl std::ops::DerefMut for Subject {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}
pub type WeaponOrigin = Subject;

/// A failed allocation still occupies its original weapon slot.
pub type WeaponSpawn = Vec<Option<ElementId>>;

#[derive(Clone, Copy, Debug)]
pub struct Concern {
    pub object: Option<ElementId>,
    pub facing: u16,
    pub which_turn: u16,
    pub movement: u32,
}

/// The AI evaluates a projected subject without moving the live ship.
pub struct Concerns {
    pub objects: [Concern; 4],
    pub subject: Subject,
    pub count: u16,
}

#[derive(Clone, Copy, Debug, Default)]
#[repr(u8)]
pub enum Selection<T> {
    #[default]
    Default = 0,
    Disabled = 1,
    Override(T) = 2,
}
impl<T: Copy> Selection<T> {
    pub fn resolve(self, default: Option<T>) -> Option<T> {
        match self {
            Self::Default => default,
            Self::Disabled => None,
            Self::Override(value) => Some(value),
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Callbacks {
    pub pre: Selection<ShipFn>,
    pub post: Selection<ShipFn>,
    pub weapon: Selection<WeaponFn>,
}
