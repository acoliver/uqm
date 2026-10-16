//! SIS module types, module slots and fuel tank geometry.
//!
//! Ports `sis.h` (module enum, slot counts, fuel constants) and the fuel
//! capacity helpers in `sis.c`: `GetModuleFuelCapacity` (`sis.c:1384`),
//! `GetFuelTankCapacity` (`sis.c:1397`), `GetFuelTankForFuelUnit`
//! (`sis.c:1417`) and the position part of `GetFTankCapacity` (`sis.c:1445`).
//!
//! Slot bytes come from the SIS state (`CSisState::module_slots`, saved
//! games), so [`ModuleSlots::from_raw`] validates them and reports a typed
//! error. Past that boundary the types cannot hold an invalid slot, so the
//! capacity code has no defensive branches.

/// `NUM_MODULE_SLOTS` (`sis.h:36`). Tied to `CSisState::module_slots` by a
/// layout test below.
pub const NUM_MODULE_SLOTS: usize = 16;
/// `NUM_MODULES` (`sis.h:89`), also the length of `GlobData.ModuleCost`.
pub const NUM_MODULES: usize = 20;
/// `EMPTY_SLOT` (`sis.h:92`): first byte past the module numbers. The C code
/// adds a kind to it to mark an empty slot: `+ 0` in a drive slot, `+ 1` in a
/// jet slot and `+ 2` in a module slot (`globdata.c:387-395`).
pub const EMPTY_SLOT: u8 = NUM_MODULES as u8;
/// `EMPTY_SLOT + 2`: the byte an empty `ModuleSlots[]` entry holds
/// (`globdata.c:395`, `outfit.c:258`).
pub const EMPTY_MODULE_SLOT: u8 = EMPTY_SLOT + 2;

/// `FUEL_TANK_SCALE` (`sis.h:40`): raw fuel points per displayed fuel unit.
pub const FUEL_TANK_SCALE: u32 = 100;
/// `FUEL_TANK_CAPACITY` (`sis.h:41`): raw capacity of one `FUEL_TANK`.
pub const FUEL_TANK_CAPACITY: u32 = 50 * FUEL_TANK_SCALE;
/// `HEFUEL_TANK_CAPACITY` (`sis.h:42`): raw capacity of one
/// `HIGHEFF_FUELSYS`.
pub const HEFUEL_TANK_CAPACITY: u32 = 100 * FUEL_TANK_SCALE;
/// `MAX_FUEL_BARS` (`sis.h:50`).
pub const MAX_FUEL_BARS: u32 = 10;
/// `FUEL_VOLUME_PER_ROW` (`sis.h:51`): raw fuel points per drawn bar.
pub const FUEL_VOLUME_PER_ROW: u32 = HEFUEL_TANK_CAPACITY / MAX_FUEL_BARS;
/// `FUEL_RESERVE` (`sis.h:52`): capacity the hull has with no tank module.
pub const FUEL_RESERVE: u32 = FUEL_VOLUME_PER_ROW;

/// `SHIP_PIECE_OFFSET` (`sis.h:110`): horizontal pitch of one module slot.
const SHIP_PIECE_OFFSET: i32 = 12;

/// Module kinds in `sis.h:62-89` order. The discriminant is the byte stored
/// in a slot.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleType {
    PlanetLander = 0,
    FusionThruster,
    TurningJets,
    CrewPod,
    StorageBay,
    FuelTank,
    HighEffFuelSys,
    DynamoUnit,
    ShivaFurnace,
    GunWeapon,
    BlasterWeapon,
    CannonWeapon,
    TrackingSystem,
    AntimissileDefense,
    BombModule0,
    BombModule1,
    BombModule2,
    BombModule3,
    BombModule4,
    BombModule5,
}

impl ModuleType {
    const ALL: [ModuleType; NUM_MODULES] = [
        ModuleType::PlanetLander,
        ModuleType::FusionThruster,
        ModuleType::TurningJets,
        ModuleType::CrewPod,
        ModuleType::StorageBay,
        ModuleType::FuelTank,
        ModuleType::HighEffFuelSys,
        ModuleType::DynamoUnit,
        ModuleType::ShivaFurnace,
        ModuleType::GunWeapon,
        ModuleType::BlasterWeapon,
        ModuleType::CannonWeapon,
        ModuleType::TrackingSystem,
        ModuleType::AntimissileDefense,
        ModuleType::BombModule0,
        ModuleType::BombModule1,
        ModuleType::BombModule2,
        ModuleType::BombModule3,
        ModuleType::BombModule4,
        ModuleType::BombModule5,
    ];

    /// The module for a slot byte, or `None` for `EMPTY_SLOT` and above.
    #[must_use]
    pub fn from_u8(raw: u8) -> Option<Self> {
        Self::ALL.get(usize::from(raw)).copied()
    }

    /// Raw fuel this module adds to the tank capacity
    /// (`GetModuleFuelCapacity`, `sis.c:1384-1394`).
    #[must_use]
    pub const fn fuel_capacity(self) -> u32 {
        match self {
            ModuleType::FuelTank => FUEL_TANK_CAPACITY,
            ModuleType::HighEffFuelSys => HEFUEL_TANK_CAPACITY,
            _ => 0,
        }
    }
}

/// One entry of `SIS_STATE.ModuleSlots`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleSlot {
    Empty,
    Installed(ModuleType),
}

impl ModuleSlot {
    /// Raw fuel the slot contributes to tank capacity.
    #[must_use]
    pub const fn fuel_capacity(self) -> u32 {
        match self {
            ModuleSlot::Empty => 0,
            ModuleSlot::Installed(module) => module.fuel_capacity(),
        }
    }
}

/// Malformed raw slot data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ModuleSlotsError {
    #[error("expected {expected} module slots, got {actual}")]
    WrongLength { expected: usize, actual: usize },
    #[error(
        "module slot {slot} holds byte {value}, neither a module (below {NUM_MODULES}) \
         nor EMPTY_MODULE_SLOT ({EMPTY_MODULE_SLOT})"
    )]
    InvalidByte { slot: usize, value: u8 },
}

/// The sixteen module slots of the flagship.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleSlots([ModuleSlot; NUM_MODULE_SLOTS]);

impl ModuleSlots {
    /// A hull with every slot empty.
    #[must_use]
    pub const fn empty() -> Self {
        Self([ModuleSlot::Empty; NUM_MODULE_SLOTS])
    }

    /// Validate raw `ModuleSlots[]` bytes from the SIS state.
    pub fn from_raw(raw: &[u8]) -> Result<Self, ModuleSlotsError> {
        if raw.len() != NUM_MODULE_SLOTS {
            return Err(ModuleSlotsError::WrongLength {
                expected: NUM_MODULE_SLOTS,
                actual: raw.len(),
            });
        }
        let mut slots = [ModuleSlot::Empty; NUM_MODULE_SLOTS];
        for (slot, (&value, out)) in raw.iter().zip(slots.iter_mut()).enumerate() {
            *out = match ModuleType::from_u8(value) {
                Some(module) => ModuleSlot::Installed(module),
                None if value == EMPTY_MODULE_SLOT => ModuleSlot::Empty,
                None => return Err(ModuleSlotsError::InvalidByte { slot, value }),
            };
        }
        Ok(Self(slots))
    }

    /// The slot at `index`, or `None` past `NUM_MODULE_SLOTS`.
    #[must_use]
    pub fn get(&self, index: usize) -> Option<ModuleSlot> {
        self.0.get(index).copied()
    }

    /// `CountSISPieces` for a module-slot piece type (`sis.c:1516-1523`).
    #[must_use]
    pub fn count(&self, module: ModuleType) -> u32 {
        self.0
            .iter()
            .filter(|slot| **slot == ModuleSlot::Installed(module))
            .count() as u32
    }

    /// `GetFuelTankCapacity` (`sis.c:1397-1409`): the reserve plus every
    /// tank module. Largest possible value is 161000.
    #[must_use]
    pub fn fuel_tank_capacity(&self) -> u32 {
        FUEL_RESERVE + self.0.iter().map(|s| s.fuel_capacity()).sum::<u32>()
    }

    /// `GetFuelTankForFuelUnit` (`sis.c:1417-1437`): the slot and remaining
    /// compartment size holding raw fuel level `unit`. Slots are searched
    /// from the last slot to the first, so the highest-numbered tank fills
    /// first. `None` when the unit does not fit.
    fn locate_fuel_unit(&self, unit: u32) -> Option<(usize, u32)> {
        debug_assert!(unit >= FUEL_RESERVE);
        let mut capacity = FUEL_RESERVE;
        for slot in (0..NUM_MODULE_SLOTS).rev() {
            capacity += self.0[slot].fuel_capacity();
            if unit < capacity {
                return Some((slot, capacity - unit));
            }
        }
        None
    }
}

/// One drawn fuel bar of `RedistributeFuel` (`outfit.c:100-135`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FuelBar {
    /// Raw fuel level this bar stands for (1000, 2000, ...).
    pub level: u32,
    /// The module slot the bar is drawn in.
    pub slot: usize,
    /// Bar row inside the module, 0 at the bottom (`rowNr`, `sis.c:1476`).
    pub row: u32,
    /// Draw position (`GetFTankCapacity`, `sis.c:1478-1482`).
    pub x: i32,
    pub y: i32,
    /// `level < FuelOnBoard`: the bar shows fuel rather than an empty cell.
    pub filled: bool,
}

/// A bar level below the capacity found no tank slot. This means the slot
/// data and the capacity disagree, which `fuel_tank_capacity` rules out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("fuel level {level} has no tank slot")]
pub struct FuelLayoutError {
    pub level: u32,
}

/// The bars `RedistributeFuel` draws for `fuel_on_board`, in draw order.
///
/// The loop runs from `FUEL_RESERVE` in steps of `FUEL_VOLUME_PER_ROW` while
/// below the tank capacity (`outfit.c:108-112`).
pub fn fuel_bar_layout(
    slots: &ModuleSlots,
    fuel_on_board: u32,
) -> Result<Vec<FuelBar>, FuelLayoutError> {
    let capacity = slots.fuel_tank_capacity();
    let mut bars = Vec::new();
    let mut level = FUEL_RESERVE;
    while level < capacity {
        let (slot, compartment) = slots
            .locate_fuel_unit(level)
            .ok_or(FuelLayoutError { level })?;
        let volume = slots.0[slot].fuel_capacity();
        let row = (volume - compartment) * MAX_FUEL_BARS / HEFUEL_TANK_CAPACITY;
        let y_base = if volume == FUEL_TANK_CAPACITY { 27 } else { 30 };
        bars.push(FuelBar {
            level,
            slot,
            row,
            x: 21 + slot as i32 * SHIP_PIECE_OFFSET,
            y: y_base - row as i32,
            filled: level < fuel_on_board,
        });
        level += FUEL_VOLUME_PER_ROW;
    }
    Ok(bars)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::locdata::{CGameState, CSisState};
    use std::mem::offset_of;

    fn slots_with(entries: &[(usize, ModuleType)]) -> ModuleSlots {
        let mut raw = [EMPTY_MODULE_SLOT; NUM_MODULE_SLOTS];
        for (index, module) in entries {
            raw[*index] = *module as u8;
        }
        ModuleSlots::from_raw(&raw).expect("fixture slots are valid")
    }

    /// `GLOBAL_SIS (ModuleSlots)` as `InitGameStructures` leaves it for a new
    /// game (`globdata.c:393-401`): every slot is `EMPTY_SLOT + 2`, then the
    /// starting modules are placed.
    fn new_game_module_slots() -> [u8; NUM_MODULE_SLOTS] {
        let mut raw = [EMPTY_SLOT + 2; NUM_MODULE_SLOTS];
        raw[15] = ModuleType::GunWeapon as u8;
        raw[2] = ModuleType::CrewPod as u8;
        raw[8] = ModuleType::StorageBay as u8;
        raw[1] = ModuleType::FuelTank as u8;
        raw
    }

    #[test]
    fn new_game_module_slots_from_the_c_initializer_validate() {
        let slots = ModuleSlots::from_raw(&new_game_module_slots())
            .expect("the native new-game hull is valid");
        assert_eq!(slots.get(0), Some(ModuleSlot::Empty));
        assert_eq!(
            slots.get(1),
            Some(ModuleSlot::Installed(ModuleType::FuelTank))
        );
        assert_eq!(slots.fuel_tank_capacity(), 6000);
        assert_eq!(slots.count(ModuleType::FuelTank), 1);
    }

    #[test]
    fn slot_and_module_counts_match_the_c_state_mirror() {
        assert_eq!(
            offset_of!(CSisState, drive_slots) - offset_of!(CSisState, module_slots),
            NUM_MODULE_SLOTS
        );
        assert_eq!(
            offset_of!(CGameState, element_worth) - offset_of!(CGameState, module_cost),
            NUM_MODULES
        );
    }

    #[test]
    fn module_enum_matches_sis_h_numbering() {
        assert_eq!(ModuleType::CrewPod as u8, 3);
        assert_eq!(ModuleType::FuelTank as u8, 5);
        assert_eq!(ModuleType::HighEffFuelSys as u8, 6);
        assert_eq!(ModuleType::BombModule5 as u8, EMPTY_SLOT - 1);
        for raw in 0..EMPTY_SLOT {
            assert_eq!(ModuleType::from_u8(raw).map(|m| m as u8), Some(raw));
        }
        assert_eq!(ModuleType::from_u8(EMPTY_SLOT), None);
    }

    #[test]
    fn fuel_constants_match_sis_h() {
        assert_eq!(FUEL_TANK_CAPACITY, 5000);
        assert_eq!(HEFUEL_TANK_CAPACITY, 10_000);
        assert_eq!(FUEL_VOLUME_PER_ROW, 1000);
        assert_eq!(FUEL_RESERVE, 1000);
    }

    #[test]
    fn new_game_style_hull_with_one_tank_holds_6000() {
        let slots = slots_with(&[(0, ModuleType::FuelTank)]);
        assert_eq!(slots.fuel_tank_capacity(), 6000);
    }

    #[test]
    fn bare_hull_holds_only_the_reserve() {
        assert_eq!(ModuleSlots::empty().fuel_tank_capacity(), FUEL_RESERVE);
    }

    #[test]
    fn non_fuel_modules_add_no_capacity() {
        let slots = slots_with(&[
            (0, ModuleType::CrewPod),
            (1, ModuleType::StorageBay),
            (2, ModuleType::DynamoUnit),
            (3, ModuleType::BombModule0),
        ]);
        assert_eq!(slots.fuel_tank_capacity(), FUEL_RESERVE);
    }

    #[test]
    fn mixed_tanks_add_their_own_capacity() {
        let slots = slots_with(&[
            (0, ModuleType::FuelTank),
            (4, ModuleType::HighEffFuelSys),
            (15, ModuleType::FuelTank),
        ]);
        assert_eq!(slots.fuel_tank_capacity(), 1000 + 5000 + 10_000 + 5000);
    }

    #[test]
    fn all_slots_high_efficiency_is_the_largest_capacity() {
        let entries: Vec<_> = (0..NUM_MODULE_SLOTS)
            .map(|i| (i, ModuleType::HighEffFuelSys))
            .collect();
        assert_eq!(slots_with(&entries).fuel_tank_capacity(), 161_000);
    }

    #[test]
    fn count_matches_count_sis_pieces() {
        let slots = slots_with(&[
            (1, ModuleType::FuelTank),
            (2, ModuleType::FuelTank),
            (7, ModuleType::CrewPod),
        ]);
        assert_eq!(slots.count(ModuleType::FuelTank), 2);
        assert_eq!(slots.count(ModuleType::CrewPod), 1);
        assert_eq!(slots.count(ModuleType::StorageBay), 0);
    }

    #[test]
    fn malformed_slot_data_is_an_explicit_error() {
        assert_eq!(
            ModuleSlots::from_raw(&[EMPTY_SLOT; 15]),
            Err(ModuleSlotsError::WrongLength {
                expected: 16,
                actual: 15
            })
        );
        let mut raw = [EMPTY_MODULE_SLOT; NUM_MODULE_SLOTS];
        // The drive-slot (20) and jet-slot (21) empties, the lander marker
        // (23) and out-of-range bytes never belong in `ModuleSlots[]`.
        for value in [EMPTY_SLOT, EMPTY_SLOT + 1, EMPTY_SLOT + 3, 255] {
            raw[9] = value;
            assert_eq!(
                ModuleSlots::from_raw(&raw),
                Err(ModuleSlotsError::InvalidByte { slot: 9, value })
            );
        }
    }

    #[test]
    fn empty_module_slot_is_the_byte_the_c_code_stores() {
        assert_eq!(EMPTY_MODULE_SLOT, 22);
        let mut raw = [EMPTY_MODULE_SLOT; NUM_MODULE_SLOTS];
        raw[3] = ModuleType::BombModule5 as u8;
        let slots = ModuleSlots::from_raw(&raw).unwrap();
        assert_eq!(slots.get(2), Some(ModuleSlot::Empty));
        assert_eq!(
            slots.get(3),
            Some(ModuleSlot::Installed(ModuleType::BombModule5))
        );
    }

    #[test]
    fn bar_layout_for_one_tank_stacks_upward() {
        let slots = slots_with(&[(0, ModuleType::FuelTank)]);
        let bars = fuel_bar_layout(&slots, 3500).unwrap();
        let levels: Vec<u32> = bars.iter().map(|b| b.level).collect();
        assert_eq!(levels, [1000, 2000, 3000, 4000, 5000]);
        let rows: Vec<u32> = bars.iter().map(|b| b.row).collect();
        assert_eq!(rows, [0, 1, 2, 3, 4]);
        let ys: Vec<i32> = bars.iter().map(|b| b.y).collect();
        assert_eq!(ys, [27, 26, 25, 24, 23]);
        assert!(bars.iter().all(|b| b.x == 21 && b.slot == 0));
        let filled: Vec<bool> = bars.iter().map(|b| b.filled).collect();
        assert_eq!(filled, [true, true, true, false, false]);
    }

    #[test]
    fn bars_fill_the_highest_numbered_tank_first() {
        let slots = slots_with(&[(2, ModuleType::FuelTank), (9, ModuleType::FuelTank)]);
        let bars = fuel_bar_layout(&slots, 0).unwrap();
        assert_eq!(bars.len(), 10);
        assert!(bars[..5].iter().all(|b| b.slot == 9 && b.x == 21 + 9 * 12));
        assert!(bars[5..].iter().all(|b| b.slot == 2 && b.x == 21 + 2 * 12));
        assert_eq!(bars[5].row, 0);
    }

    #[test]
    fn high_efficiency_tank_uses_the_taller_baseline() {
        let slots = slots_with(&[(3, ModuleType::HighEffFuelSys)]);
        let bars = fuel_bar_layout(&slots, 0).unwrap();
        assert_eq!(bars.len(), 10);
        assert_eq!((bars[0].row, bars[0].y), (0, 30));
        assert_eq!((bars[9].row, bars[9].y), (9, 21));
    }

    #[test]
    fn bare_hull_has_no_bars() {
        assert!(fuel_bar_layout(&ModuleSlots::empty(), 1000)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn fuel_at_exactly_a_bar_level_does_not_fill_that_bar() {
        let slots = slots_with(&[(0, ModuleType::FuelTank)]);
        let bars = fuel_bar_layout(&slots, 2000).unwrap();
        assert!(bars[0].filled);
        assert!(!bars[1].filled);
    }
}
