use super::Handler;
use crate::comm::races::alliance::AllianceApi;
use crate::planets::universe_units::{universe_to_logx, universe_to_logy};
use crate::starbase::sis_modules::{ModuleType, EMPTY_MODULE_SLOT, FUEL_RESERVE};

pub(super) trait ChmmrApi: AllianceApi<Handler> {
    fn refit(&mut self);
    fn intro_mode(&mut self, mode: i32, duration: i32);
}

fn refit(glob: &mut crate::ships::native::abi::GLOBDATA, starbase: bool) {
    let ship = &mut glob.SIS_state;
    let game = &mut glob.Game_state;
    ship.ResUnits = 1_000_000;
    ship.NumLanders = 0;
    game.ModuleCost[ModuleType::PlanetLander as usize] = 0;
    ship.log_x = universe_to_logx(1752);
    ship.log_y = universe_to_logy(1450);
    game.ShipFacing = 1;
    game.ip_planet = 3;
    game.in_orbit = 0;
    game.ip_location.x = -725;
    game.ip_location.y = 597;
    if starbase {
        ship.FuelOnBoard = FUEL_RESERVE;
        ship.CrewEnlisted = 0;
        ship.TotalElementMass = 0;
        game.ModuleCost[ModuleType::StorageBay as usize] = 0;
        ship.ElementAmounts.fill(0);
        ship.ModuleSlots[10..].fill(EMPTY_MODULE_SLOT);
        game.ShipStamp.origin.x = 86;
        game.ShipStamp.origin.y = 113;
    } else {
        // Pack from the high end before replacing the ten bomb slots, exactly
        // as the original refit does when the Starbase has never been visited.
        let mut i = 15usize;
        while i > 0 {
            while i > 0 && ship.ModuleSlots[i] != EMPTY_MODULE_SLOT {
                i -= 1;
            }
            if i == 0 {
                break;
            }
            let mut m = i as i32 - 1;
            while m >= 0 && ship.ModuleSlots[m as usize] == EMPTY_MODULE_SLOT {
                m -= 1;
            }
            if m < 0 {
                break;
            }
            ship.ModuleSlots[i] = ship.ModuleSlots[m as usize];
            ship.ModuleSlots[m as usize] = EMPTY_MODULE_SLOT;
            i -= 1;
        }
        game.ShipStamp.origin.x = 121;
        game.ShipStamp.origin.y = 113;
    }
    ship.DriveSlots.fill(ModuleType::FusionThruster as u8);
    ship.JetSlots.fill(ModuleType::TurningJets as u8);
    use ModuleType::{
        BombModule0, BombModule1, BombModule2, BombModule3, BombModule4, BombModule5,
    };
    ship.ModuleSlots[..10].copy_from_slice(
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
            static mut GlobData: crate::ships::native::abi::GLOBDATA;
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
    pub glob: crate::ships::native::abi::GLOBDATA,
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
