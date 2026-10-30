//! Species behavior and value inputs for AI projections.

use super::{CollisionHook, Cx, ElementHook, ElementId, ShipId};

pub use super::record::RaceDesc;
pub type ShipFn = fn(&mut Cx<'_>, ShipId);
pub type WeaponFn = fn(&mut Cx<'_>, ShipId, WeaponOrigin) -> WeaponSpawn;
pub type IntelligenceFn = fn(&mut Cx<'_>, ShipId, &mut Concerns);

/// Immutable behavior shared by all ships of one converted species.
#[derive(Debug)]
pub struct Race {
    pub descriptor: fn(&mut super::World) -> RaceDesc,
    pub preprocess: Option<ShipFn>,
    pub postprocess: Option<ShipFn>,
    pub init_weapon: Option<WeaponFn>,
    pub intelligence: Option<IntelligenceFn>,
    pub uninit: Option<fn(&mut super::World, &mut super::views::RaceStats)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryKey {
    ShofixtiRecruited,
    ProbeIlwrathEncounter,
    ArilouSpaceSide,
    ChmmrBombState,
    BombCarrier,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default)]
pub enum RaceState {
    #[default]
    None = 0,
    Androsynth {
        collision: Option<CollisionHook>,
    } = 1,
    Mmrnmhrm {
        alternate: super::record::CHARACTERISTIC_STUFF,
    } = 2,
    Pkunk {
        phoenix: Option<ElementId>,
        saved_pre: Option<ElementHook>,
        saved_post: Option<ElementHook>,
        saved_death: Option<ElementHook>,
    } = 3,
    Umgah {
        previous_facing: Option<u16>,
    } = 4,
    Sis(SisState) = 5,
    SaMatra(SaMatraState) = 6,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SisState {
    pub trackers: u16,
    pub blasters: u16,
    pub missiles: [Option<FlagshipWeapon>; 6],
}

/// Module-derived launch parameters exist before the flagship graphics are loaded.
#[derive(Clone, Copy, Debug, Default)]
pub struct FlagshipWeapon {
    pub flags: u16,
    pub blast_offset: i16,
    pub speed: i16,
    pub preprocess: Option<super::ElementFn>,
    pub hit_points: i16,
    pub damage: i16,
    pub life: u16,
    pub index: u16,
    pub pixel_offset: i16,
    pub facing: u16,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SaMatraState {
    pub generation: u64,
    pub destroyed: u8,
    pub gate_entered: bool,
    pub generators: [Option<ElementId>; 8],
}

pub use super::views::Subject;
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
    pub objects: [Concern; 10],
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
