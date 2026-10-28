//! Species behavior and value inputs for AI projections.

use super::{record::RACE_DESC, CollisionHook, Cx, ElementId, ShipId};
use crate::graphics::native_abi::POINT;

pub type RaceDesc = RACE_DESC;
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
    pub uninit: Option<fn(&mut RaceDesc)>,
}

#[derive(Clone, Copy, Debug, Default)]
pub enum RaceState {
    #[default]
    None,
    Vux {
        entry_done: bool,
    },
    Androsynth {
        collision: Option<CollisionHook>,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct WeaponOrigin {
    pub location: POINT,
    pub player: i16,
}

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
    pub subject: super::record::ELEMENT,
    pub count: u16,
}

impl RaceDesc {
    pub fn callbacks(&self) -> [usize; 5] {
        if let Some(race) = self.race {
            [
                race.uninit.map_or(0, |f| f as usize),
                if matches!(self.race_state, RaceState::Vux { entry_done: true }) {
                    0
                } else {
                    race.preprocess.map_or(0, |f| f as usize)
                },
                race.postprocess.map_or(0, |f| f as usize),
                race.init_weapon as usize,
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

impl Cx<'_> {
    pub fn weapon_origin(&self, id: ElementId) -> WeaponOrigin {
        WeaponOrigin {
            location: self.b.elements[id].next.location,
            player: self.b.elements[id].playerNr,
        }
    }
}
