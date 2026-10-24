use super::Handler;
use crate::comm::locdata::CGlobData;
use crate::comm::races::alliance::AllianceApi;
use crate::planets::universe_units::{universe_to_logx, universe_to_logy};
use crate::starbase::sis_modules::{ModuleType, EMPTY_MODULE_SLOT, FUEL_RESERVE};

pub(super) trait ChmmrApi: AllianceApi<Handler> {
    fn refit(&mut self);
    fn intro_mode(&mut self, mode: i32, duration: i32);
}

fn refit(glob: &mut CGlobData, starbase: bool) {
    let ship = &mut glob.sis_state;
    let game = &mut glob.game_state;
    ship.res_units = 1_000_000;
    ship.num_landers = 0;
    game.module_cost[ModuleType::PlanetLander as usize] = 0;
    ship.log_x = universe_to_logx(1752);
    ship.log_y = universe_to_logy(1450);
    game.ship_facing = 1;
    game.ip_planet = 3;
    game.in_orbit = 0;
    game.ip_location.x = -725;
    game.ip_location.y = 597;
    if starbase {
        ship.fuel_on_board = FUEL_RESERVE;
        ship.crew_enlisted = 0;
        ship.total_element_mass = 0;
        game.module_cost[ModuleType::StorageBay as usize] = 0;
        ship.element_amounts.fill(0);
        ship.module_slots[10..].fill(EMPTY_MODULE_SLOT);
        game.ship_stamp.origin.x = 86;
        game.ship_stamp.origin.y = 113;
    } else {
        // Pack from the high end before replacing the ten bomb slots, exactly
        // as the original refit does when the Starbase has never been visited.
        let mut i = 15usize;
        while i > 0 {
            while i > 0 && ship.module_slots[i] != EMPTY_MODULE_SLOT {
                i -= 1;
            }
            if i == 0 {
                break;
            }
            let mut m = i as i32 - 1;
            while m >= 0 && ship.module_slots[m as usize] == EMPTY_MODULE_SLOT {
                m -= 1;
            }
            if m < 0 {
                break;
            }
            ship.module_slots[i] = ship.module_slots[m as usize];
            ship.module_slots[m as usize] = EMPTY_MODULE_SLOT;
            i -= 1;
        }
        game.ship_stamp.origin.x = 121;
        game.ship_stamp.origin.y = 113;
    }
    ship.drive_slots.fill(ModuleType::FusionThruster as u8);
    ship.jet_slots.fill(ModuleType::TurningJets as u8);
    use ModuleType::{
        BombModule0, BombModule1, BombModule2, BombModule3, BombModule4, BombModule5,
    };
    ship.module_slots[..10].copy_from_slice(
        &[
            BombModule4,
            BombModule5,
            BombModule3,
            BombModule1,
            BombModule0,
            BombModule1,
            BombModule3,
            BombModule4,
            BombModule5,
            BombModule2,
        ]
        .map(|module| module as u8),
    );
}
#[cfg(not(test))]
impl ChmmrApi for super::super::script::live::LiveScript<Handler> {
    fn refit(&mut self) {
        use crate::comm::races::script::ScriptApi;
        let starbase = self.get("STARBASE_AVAILABLE") != 0;
        extern "C" {
            static mut GlobData: CGlobData;
        }
        unsafe {
            refit(&mut *std::ptr::addr_of_mut!(GlobData), starbase);
        }
    }
    fn intro_mode(&mut self, mode: i32, duration: i32) {
        use crate::comm::comm_abi::SetCommIntroMode;
        SetCommIntroMode(mode as u32, duration as u32);
    }
}
#[cfg(test)]
pub(super) struct World {
    pub glob: CGlobData,
    pub refits: usize,
    pub intros: Vec<(i32, i32)>,
}
#[cfg(test)]
impl Default for World {
    fn default() -> Self {
        Self {
            glob: unsafe { std::mem::zeroed() },
            refits: 0,
            intros: Vec::new(),
        }
    }
}
#[cfg(test)]
impl ChmmrApi for super::super::encounter::testing::Fake<Handler, World> {
    fn refit(&mut self) {
        use crate::comm::races::script::ScriptApi;
        let starbase = self.get("STARBASE_AVAILABLE") != 0;
        self.world.refits += 1;
        refit(&mut self.world.glob, starbase);
    }
    fn intro_mode(&mut self, mode: i32, duration: i32) {
        self.world.intros.push((mode, duration));
    }
}
