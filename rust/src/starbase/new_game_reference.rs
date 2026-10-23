//! Starting flagship fixture from the live Rust equipment and price initializer.

use super::platform::SisView;

const SIS_H: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../sc2/src/uqm/sis.h"));
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

pub fn new_game_crew_cost() -> u8 {
    let mut sis = unsafe { std::mem::zeroed() };
    let mut state = unsafe { std::mem::zeroed() };
    crate::navigation::initial_state::prices(&mut sis, &mut state);
    state.CrewCost
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

impl NewGameFlagship {
    pub fn from_initializer() -> Self {
        let mut sis = unsafe { std::mem::zeroed() };
        let mut state = unsafe { std::mem::zeroed() };
        crate::navigation::initial_state::equipment(&mut sis);
        crate::navigation::initial_state::prices(&mut sis, &mut state);
        Self {
            drive_slots: sis.DriveSlots,
            jet_slots: sis.JetSlots,
            module_slots: sis.ModuleSlots,
            module_cost: state.ModuleCost,
            num_landers: sis.NumLanders,
            crew_enlisted: sis.CrewEnlisted,
            fuel_on_board: sis.FuelOnBoard,
            res_units: sis.ResUnits,
            fuel_cost: state.FuelCost,
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
    fn the_new_game_hull_comes_from_the_live_initializer() {
        let flagship = NewGameFlagship::from_initializer();
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
    fn the_new_game_prices_preserve_the_original_costs_in_module_cost_units() {
        let flagship = NewGameFlagship::from_initializer();
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
        let flagship = NewGameFlagship::from_initializer();
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
