//! The flagship a new game starts with, read from the C reference source.
//!
//! The Outfit tests need a hull that matches what `InitGameStructures`
//! builds (`globdata.c:386-424`), not one typed in from memory. This module
//! reads the piece numbers from `sis.h` and the assignments from
//! `globdata.c`, both embedded at build time, so a change to either file
//! changes the fixture.

use super::platform::SisView;

const SIS_H: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../sc2/src/uqm/sis.h"));
const GLOBDATA_C: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../sc2/src/uqm/globdata.c"
));
const MODULES_MENU_ANI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../sc2/content/base/ui/modulesmenu.ani"
));
const OUTFIT_MODULES_ANI: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../sc2/content/base/ui/outfitmodules.ani"
));

/// The piece enumerators of `sis.h` with their numbers.
pub struct PieceNumbers(Vec<(String, u32)>);

fn strip_block_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("*/").expect("comment closes") + start + 2;
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

impl PieceNumbers {
    pub fn from_sis_h() -> Self {
        let start = SIS_H
            .find("PLANET_LANDER = 0")
            .expect("piece enum in sis.h");
        let end = start
            + SIS_H[start..]
                .find("NUM_MODULES /* must be last entry */")
                .expect("enum ends with NUM_MODULES");
        let body = strip_block_comments(&SIS_H[start..end]);
        let mut numbers: Vec<(String, u32)> = Vec::new();
        let mut next = 0;
        for token in body.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            let (name, value) = match token.split_once('=') {
                Some((name, value)) => {
                    let value = value.trim();
                    let number = value.parse().unwrap_or_else(|_| {
                        numbers
                            .iter()
                            .find(|(known, _)| known == value)
                            .unwrap_or_else(|| panic!("unknown enumerator {value}"))
                            .1
                    });
                    (name.trim(), number)
                }
                None => (token, next),
            };
            numbers.push((name.to_owned(), value));
            next = value + 1;
        }
        numbers.push(("NUM_MODULES".to_owned(), next));
        Self(numbers)
    }

    pub fn get(&self, name: &str) -> u32 {
        self.0
            .iter()
            .find(|(known, _)| known == name)
            .unwrap_or_else(|| panic!("sis.h has no {name}"))
            .1
    }
}

/// `#define NAME value` of `sis.h`, for a plain number.
pub fn sis_h_define(name: &str) -> u32 {
    let line = SIS_H
        .lines()
        .find(|line| line.split_whitespace().take(2).eq(["#define", name]))
        .unwrap_or_else(|| panic!("sis.h defines {name}"));
    line.split_whitespace()
        .nth(2)
        .and_then(|value| value.parse().ok())
        .unwrap_or_else(|| panic!("{name} is not a plain number"))
}

/// Frames in an `.ani` file: one line per frame.
fn ani_frames(text: &str) -> usize {
    text.lines().filter(|line| !line.trim().is_empty()).count()
}

pub fn modules_menu_frames() -> usize {
    ani_frames(MODULES_MENU_ANI)
}

pub fn outfit_modules_frames() -> usize {
    ani_frames(OUTFIT_MODULES_ANI)
}

/// `GLOBAL_SIS` and `GLOBAL` as `InitGameStructures` leaves them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGameFlagship {
    pub drive_slots: [u8; 11],
    pub jet_slots: [u8; 8],
    pub module_slots: [u8; 16],
    pub module_cost: [u8; 20],
    pub num_landers: u8,
    pub crew_enlisted: u16,
    pub fuel_on_board: u32,
    pub res_units: u32,
    pub fuel_cost: u8,
}

/// The `GLOBAL` and `GLOBAL_SIS` fields the Outfit screens read.
const TRACKED_FIELDS: [&str; 9] = [
    "DriveSlots",
    "JetSlots",
    "ModuleSlots",
    "ModuleCost",
    "NumLanders",
    "CrewEnlisted",
    "FuelOnBoard",
    "ResUnits",
    "FuelCost",
];

enum Target {
    Whole,
    At(usize),
}

/// The `Field[index]` or `(Field)` an assignment's left side names.
fn assigned(left: &str, pieces: &PieceNumbers) -> Option<(String, Target)> {
    let open = left.rfind('(')?;
    let tail = &left[open + 1..];
    if let Some(bracket) = tail.find('[') {
        let field = tail[..bracket].trim().to_owned();
        let index = tail[bracket + 1..tail.find(']')?].trim();
        let target = match index {
            "i" => Target::Whole,
            _ => Target::At(
                index
                    .parse::<usize>()
                    .unwrap_or_else(|_| pieces.get(index) as usize),
            ),
        };
        Some((field, target))
    } else {
        let close = tail.find(')')?;
        Some((tail[..close].trim().to_owned(), Target::Whole))
    }
}

fn value_of(right: &str, pieces: &PieceNumbers, cost_scale: u32) -> u32 {
    let right = right.trim();
    if let Some((base, add)) = right.split_once('+') {
        return value_of(base, pieces, cost_scale) + value_of(add, pieces, cost_scale);
    }
    if let Some((dividend, divisor)) = right.split_once('/') {
        assert_eq!(divisor.trim(), "MODULE_COST_SCALE");
        return value_of(dividend, pieces, cost_scale) / cost_scale;
    }
    if right == "EMPTY_SLOT" {
        return pieces.get("NUM_MODULES");
    }
    if right.contains("FUEL_TANK_SCALE") {
        let (times, _) = right.split_once('*').expect("a product");
        return times.trim().parse::<u32>().expect("a count") * sis_h_define("FUEL_TANK_SCALE");
    }
    right.parse().unwrap_or_else(|_| match right {
        "CREW_POD_CAPACITY" => sis_h_define("CREW_POD_CAPACITY"),
        _ => pieces.get(right),
    })
}

impl NewGameFlagship {
    pub fn from_c_source() -> Self {
        let pieces = PieceNumbers::from_sis_h();
        let cost_scale = sis_h_define("MODULE_COST_SCALE");
        let start = GLOBDATA_C
            .find("for (i = 0; i < NUM_DRIVE_SLOTS; ++i)")
            .expect("slot initialisation in globdata.c");
        let end = start
            + GLOBDATA_C[start..]
                .find("GLOBAL_SIS (NumLanders) = 1;")
                .expect("lander count in globdata.c")
            + "GLOBAL_SIS (NumLanders) = 1;".len();
        let mut flagship = Self {
            drive_slots: [0; 11],
            jet_slots: [0; 8],
            module_slots: [0; 16],
            module_cost: [0; 20],
            num_landers: 0,
            crew_enlisted: 0,
            fuel_on_board: 0,
            res_units: 0,
            fuel_cost: 0,
        };

        for statement in GLOBDATA_C[start..end].split(';') {
            if !statement.contains('=') || !statement.contains("GLOBAL") {
                continue;
            }
            let mut parts: Vec<&str> = statement.split('=').collect();
            let right = parts.pop().expect("an assignment has a right side");
            for left in parts {
                let Some((field, target)) = assigned(left, &pieces) else {
                    continue;
                };
                if !TRACKED_FIELDS.contains(&field.as_str()) {
                    continue;
                }
                let value = value_of(right, &pieces, cost_scale);
                flagship.assign(&field, &target, value);
            }
        }
        flagship
    }

    fn assign(&mut self, field: &str, target: &Target, value: u32) {
        fn fill<const N: usize>(slots: &mut [u8; N], target: &Target, value: u32) {
            let value = u8::try_from(value).expect("a slot byte");
            match target {
                Target::Whole => slots.fill(value),
                Target::At(index) => slots[*index] = value,
            }
        }
        match field {
            "DriveSlots" => fill(&mut self.drive_slots, target, value),
            "JetSlots" => fill(&mut self.jet_slots, target, value),
            "ModuleSlots" => fill(&mut self.module_slots, target, value),
            "ModuleCost" => fill(&mut self.module_cost, target, value),
            "NumLanders" => self.num_landers = u8::try_from(value).expect("a lander count"),
            "CrewEnlisted" => self.crew_enlisted = u16::try_from(value).expect("a crew count"),
            "FuelOnBoard" => self.fuel_on_board = value,
            "ResUnits" => self.res_units = value,
            "FuelCost" => self.fuel_cost = u8::try_from(value).expect("a fuel cost"),
            other => unreachable!("{other} is not a tracked field"),
        }
    }

    /// The Outfit screen's view of this flagship.
    pub fn sis_view(&self) -> SisView {
        SisView {
            res_units: self.res_units,
            fuel_on_board: self.fuel_on_board,
            crew_enlisted: self.crew_enlisted,
            total_element_mass: 0,
            module_slots: self.module_slots,
            drive_slots: self.drive_slots,
            jet_slots: self.jet_slots,
            num_landers: self.num_landers,
            ship_name: "Vindicator".to_owned(),
            fuel_cost: self.fuel_cost,
            module_cost: self.module_cost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::starbase::sis_modules::{ModuleSlots, ModuleType};

    #[test]
    fn piece_numbers_follow_the_sis_h_enum() {
        let pieces = PieceNumbers::from_sis_h();
        assert_eq!(pieces.get("PLANET_LANDER"), 0);
        assert_eq!(pieces.get("CREW_POD"), ModuleType::CrewPod as u32);
        assert_eq!(pieces.get("FUEL_TANK"), ModuleType::FuelTank as u32);
        assert_eq!(
            pieces.get("ANTIMISSILE_DEFENSE"),
            ModuleType::AntimissileDefense as u32
        );
        assert_eq!(pieces.get("NUM_PURCHASE_MODULES"), 14);
        assert_eq!(pieces.get("BOMB_MODULE_0"), 14);
        assert_eq!(pieces.get("BOMB_MODULE_5"), ModuleType::BombModule5 as u32);
        assert_eq!(pieces.get("NUM_MODULES"), 20);
    }

    #[test]
    fn the_new_game_hull_comes_from_the_c_initializer() {
        let flagship = NewGameFlagship::from_c_source();
        // Drive slots 5 and 6 hold FUSION_THRUSTER, the rest EMPTY_SLOT + 0.
        let mut drives = [20; 11];
        drives[5] = 1;
        drives[6] = 1;
        assert_eq!(flagship.drive_slots, drives);
        // Jet slots 0 and 6 hold TURNING_JETS, the rest EMPTY_SLOT + 1.
        let mut jets = [21; 8];
        jets[0] = 2;
        jets[6] = 2;
        assert_eq!(flagship.jet_slots, jets);
        // Every module slot starts at EMPTY_SLOT + 2 (22), then the starting
        // modules go in.
        let mut modules = [22; 16];
        modules[15] = ModuleType::GunWeapon as u8;
        modules[2] = ModuleType::CrewPod as u8;
        modules[8] = ModuleType::StorageBay as u8;
        modules[1] = ModuleType::FuelTank as u8;
        assert_eq!(flagship.module_slots, modules);
        assert_eq!(flagship.num_landers, 1);
        assert_eq!(flagship.crew_enlisted, 50);
        assert_eq!(flagship.fuel_on_board, 1000);
        assert_eq!(flagship.res_units, 0);
        assert_eq!(flagship.fuel_cost, 20);
    }

    #[test]
    fn the_new_game_prices_are_the_c_costs_in_module_cost_units() {
        let flagship = NewGameFlagship::from_c_source();
        let mut costs = [0; 20];
        costs[ModuleType::PlanetLander as usize] = 10;
        costs[ModuleType::FusionThruster as usize] = 10;
        costs[ModuleType::TurningJets as usize] = 10;
        costs[ModuleType::CrewPod as usize] = 40;
        costs[ModuleType::StorageBay as usize] = 15;
        costs[ModuleType::FuelTank as usize] = 10;
        costs[ModuleType::DynamoUnit as usize] = 40;
        costs[ModuleType::GunWeapon as usize] = 40;
        assert_eq!(flagship.module_cost, costs);
    }

    #[test]
    fn the_new_game_hull_is_valid_for_the_module_slots_model() {
        let flagship = NewGameFlagship::from_c_source();
        ModuleSlots::from_raw(&flagship.module_slots).expect("native hull validates");
    }

    #[test]
    fn module_art_has_a_frame_for_every_purchasable_module() {
        let pieces = PieceNumbers::from_sis_h();
        let purchasable = pieces.get("NUM_PURCHASE_MODULES") as usize;
        assert!(
            modules_menu_frames() >= purchasable,
            "modulesmenu.ani has {} frames",
            modules_menu_frames()
        );
        // Every drawn ship piece is a top view at its piece number or the
        // side view `NUM_MODULES - 1 + 6` frames later; the highest piece is
        // the lander berth marker, EMPTY_SLOT + 3.
        let empty_slot = pieces.get("NUM_MODULES") as usize;
        let side_view_offset = (empty_slot - 1) + 6;
        assert!(outfit_modules_frames() > empty_slot + 3 + side_view_offset);
    }
}
