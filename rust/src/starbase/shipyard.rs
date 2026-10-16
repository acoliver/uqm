//! Shipyard rules: hangar cursor, ship prices, crew arithmetic and the crew
//! pod display (`shipyard.c`, and `GetCPodCapacity` in `sis.c`).
//!
//! Everything here is pure. [`super::shipyard_screen`] carries the results
//! out through [`super::platform::Platform`] and [`super::gfx::Gfx`].

use super::gfx::{Point, Rgba};
use super::platform::{AlliedState, Escort, RaceTemplate, SisView, StarbaseError, UiStyle};
use super::sis_modules::ModuleType;

/// `RACE_SHIP_COST` (`races.h`), indexed by the `avail_race_q` position.
pub const SHIP_COST: [u16; 23] = [
    1600, 3000, 1100, 2300, 2000, 500, 1800, 1600, 1000, 2200, 1200, 2300, 3600, 1700, 1000, 2100,
    4400, 700, 3000, 600, 1300, 3000, 2300,
];

/// `CREW_EXPENSE_THRESHOLD` (`sis.h`): past this many crew bought, each
/// crew member costs two RU more.
pub const CREW_EXPENSE_THRESHOLD: i32 = 1000;
/// `CREW_POD_CAPACITY` (`sis.h`).
pub const CREW_POD_CAPACITY: u32 = 50;
/// `CREW_PER_ROW` (`sis.h`).
const CREW_PER_ROW: u16 = 5;
/// `SHIP_PIECE_OFFSET` (`sis.h`).
const SHIP_PIECE_OFFSET: i32 = 12;

/// The PC hangar layout (`shipyard.c:45-56`, `USE_3DO_HANGAR` is not
/// defined).
pub const HANGAR_SHIPS_ROW: u8 = 6;
pub const HANGAR_ROWS: u8 = 2;
pub const HANGAR_SHIPS: u8 = HANGAR_SHIPS_ROW * HANGAR_ROWS;
pub const HANGAR_X: [i16; HANGAR_SHIPS_ROW as usize] = [0, 38, 76, 131, 169, 207];
pub const HANGAR_Y: i16 = 88;
pub const HANGAR_DY: i16 = 84;
pub const SHIP_WIN_WIDTH: i16 = 34;
pub const SHIP_WIN_HEIGHT: i16 = SHIP_WIN_WIDTH + 6;
pub const SHIP_WIN_FRAMES: u16 = (SHIP_WIN_WIDTH as u16 >> 1) + 1;

/// The `PC_CREW_COLOR_TABLE` (`colors.h`).
const PC_CREW_COLORS: [Rgba; 10] = [
    Rgba::rgb15(0x0A, 0x1E, 0x09),
    Rgba::rgb15(0x00, 0x1E, 0x00),
    Rgba::rgb15(0x00, 0x1B, 0x00),
    Rgba::rgb15(0x00, 0x18, 0x00),
    Rgba::rgb15(0x00, 0x15, 0x00),
    Rgba::rgb15(0x00, 0x12, 0x00),
    Rgba::rgb15(0x00, 0x10, 0x00),
    Rgba::rgb15(0x00, 0x0D, 0x00),
    Rgba::rgb15(0x00, 0x0A, 0x00),
    Rgba::rgb15(0x00, 0x07, 0x00),
];
/// `THREEDO_CREW_COLOR` (`colors.h`).
const THREEDO_CREW_COLOR: Rgba = Rgba::rgb15(0x05, 0x10, 0x05);

/// `ShipCost[race]`: the price of the ship of an `avail_race_q` position.
pub fn ship_cost(race_id: u8) -> Result<u16, StarbaseError> {
    SHIP_COST
        .get(usize::from(race_id))
        .copied()
        .ok_or(StarbaseError::UnknownRace(race_id))
}

/// The `avail_race_q` positions of the races the player can build, in queue
/// order (`GetAvailableRaceFromIndex`).
#[must_use]
pub fn available_races(templates: &[RaceTemplate]) -> Vec<u8> {
    templates
        .iter()
        .enumerate()
        .filter(|(_, template)| template.allied == AlliedState::GoodGuy)
        .map(|(index, _)| u8::try_from(index).expect("the fleet queue has fewer than 256 races"))
        .collect()
}

/// The upper-left corner of a hangar slot (`DMS_GetEscortShipRect`).
#[must_use]
pub fn slot_corner(slot: u8) -> Point {
    Point {
        x: HANGAR_X[usize::from(slot % HANGAR_SHIPS_ROW)],
        y: HANGAR_Y + HANGAR_DY * i16::from(slot / HANGAR_SHIPS_ROW),
    }
}

/// `MENU_STATE.CurState` of `DoModifyShips`: an escort slot, or the flagship
/// (the high nibble). The slot is kept while the flagship is selected, so
/// moving off the flagship returns to the same column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    flagship: bool,
    slot: u8,
}

impl Cursor {
    /// `MAKE_BYTE (0, 0xF)`.
    pub const START: Self = Self {
        flagship: true,
        slot: 0,
    };

    #[must_use]
    pub const fn is_flagship(self) -> bool {
        self.flagship
    }

    #[must_use]
    pub const fn slot(self) -> u8 {
        self.slot
    }

    /// `DMS_MoveCursor`. Vertical motion wraps through the two escort rows
    /// and the flagship, which counts as one more row.
    #[must_use]
    pub fn moved(self, dx: i8, dy: i8) -> Self {
        let mut row = self.slot / HANGAR_SHIPS_ROW;
        let mut col = self.slot % HANGAR_SHIPS_ROW;
        let mut flagship = self.flagship;
        if dy != 0 {
            if flagship {
                row = HANGAR_ROWS;
            }
            let rows = i16::from(HANGAR_ROWS) + 1;
            row = ((i16::from(row) + rows + i16::from(dy)) % rows) as u8;
            flagship = row == HANGAR_ROWS;
            if flagship {
                row = 0;
            }
        } else if dx != 0 && !flagship {
            let cols = i16::from(HANGAR_SHIPS_ROW);
            col = ((i16::from(col) + cols + i16::from(dx)) % cols) as u8;
        }
        Self {
            flagship,
            slot: row * HANGAR_SHIPS_ROW + col,
        }
    }
}

/// The crew counters after `CrewTransaction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrewTotals {
    /// `MAKE_WORD (CREW_PURCHASED0, CREW_PURCHASED1)`, a C `SIZE`.
    pub purchased: i16,
    /// `GLOBAL (CrewCost)`.
    pub crew_cost: u8,
    /// The price crossed the threshold: the menu price must be redrawn.
    pub cost_changed: bool,
}

/// `CrewTransaction` (`shipyard.c:316-358`) for a nonzero `delta`.
#[must_use]
pub fn crew_transaction(purchased: i16, crew_cost: u8, delta: i16) -> CrewTotals {
    let mut totals = CrewTotals {
        purchased,
        crew_cost,
        cost_changed: false,
    };
    let delta = i32::from(delta);
    // `SIZE crew_bought = (SIZE)... + crew_delta`
    let mut bought = i32::from((i32::from(purchased) + delta) as i16);
    if bought < 0 {
        bought = if delta < 0 { 0 } else { 0x7FFF };
    } else if delta > 0 {
        if bought >= CREW_EXPENSE_THRESHOLD && bought - delta < CREW_EXPENSE_THRESHOLD {
            totals.crew_cost = crew_cost.wrapping_add(2);
            totals.cost_changed = true;
        }
    } else if bought < CREW_EXPENSE_THRESHOLD && bought - delta >= CREW_EXPENSE_THRESHOLD {
        totals.crew_cost = crew_cost.wrapping_sub(2);
        totals.cost_changed = true;
    }
    totals.purchased = bought as i16;
    totals
}

/// The RU a dismissed crew member returns (`shipyard.c:468`, `:581`).
#[must_use]
pub fn dismiss_refund(crew_cost: u8, purchased: i16) -> u32 {
    let step = if i32::from(purchased) == CREW_EXPENSE_THRESHOLD {
        2
    } else {
        0
    };
    u32::from(crew_cost.wrapping_sub(step))
}

/// The crew pod capacity of the ship (`GetCrewPodCapacity`).
#[must_use]
pub fn crew_pod_capacity(module_slots: &[u8]) -> u32 {
    let pods = module_slots
        .iter()
        .filter(|piece| **piece == ModuleType::CrewPod as u8)
        .count() as u32;
    pods * CREW_POD_CAPACITY
}

/// `DMS_HireFlagShipCrew`: the RU a hire costs, or `None` when the pods are
/// full or the RU do not cover it.
#[must_use]
pub fn flagship_hire(sis: &SisView, crew_cost: u8) -> Option<u32> {
    if crew_pod_capacity(&sis.module_slots) <= u32::from(sis.crew_enlisted) {
        return None;
    }
    (sis.res_units >= u32::from(crew_cost)).then_some(u32::from(crew_cost))
}

/// `DMS_HireEscortShipCrew`: the RU the hire costs, or `None` when it is
/// refused. A hire into a ship with no crew buys the ship.
pub fn escort_hire(
    escort: &Escort,
    template_max_crew: u16,
    res_units: u32,
    crew_cost: u8,
) -> Result<Option<u32>, StarbaseError> {
    if res_units < u32::from(crew_cost)
        || escort.crew_level >= escort.max_crew
        || escort.crew_level >= template_max_crew
    {
        return Ok(None);
    }
    Ok(Some(if escort.crew_level > 0 {
        u32::from(crew_cost)
    } else {
        u32::from(ship_cost(escort.race_id)?)
    }))
}

/// What `DMS_DismissEscortShipCrew` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscortDismiss {
    /// No crew to dismiss.
    NoCrew,
    /// One crew member leaves; the RU are returned.
    Crew { refund: u32 },
    /// The last crew member leaves; the ship is sold.
    Scrap { refund: u32 },
}

pub fn escort_dismiss(
    escort: &Escort,
    crew_cost: u8,
    purchased: i16,
) -> Result<EscortDismiss, StarbaseError> {
    Ok(match escort.crew_level {
        0 => EscortDismiss::NoCrew,
        1 => EscortDismiss::Scrap {
            refund: u32::from(ship_cost(escort.race_id)?),
        },
        _ => EscortDismiss::Crew {
            refund: dismiss_refund(crew_cost, purchased),
        },
    })
}

/// Where `GetCPodCapacity` puts the crew member of this number, and the
/// colour it draws it in. `None` when the crew member does not fit.
#[must_use]
pub fn crew_dot(module_slots: &[u8], crew_number: u16, fonts: UiStyle) -> Option<(Point, Rgba)> {
    let mut capacity: u32 = 0;
    for slot in (0..module_slots.len()).rev() {
        let module_capacity = if module_slots[slot] == ModuleType::CrewPod as u8 {
            CREW_POD_CAPACITY
        } else {
            0
        };
        if u32::from(crew_number) < capacity + module_capacity {
            let seat = (u32::from(crew_number) - capacity) as u16;
            let row = seat / CREW_PER_ROW;
            let col = seat % CREW_PER_ROW;
            let color = match fonts {
                UiStyle::Pc => PC_CREW_COLORS[usize::from(row)],
                UiStyle::ThreeDo => THREEDO_CREW_COLOR,
            };
            let point = Point {
                x: (27 + slot as i32 * SHIP_PIECE_OFFSET - i32::from(col) * 2) as i16,
                y: (34 - i32::from(row) * 2) as i16,
            };
            return Some((point, color));
        }
        capacity += module_capacity;
    }
    None
}
