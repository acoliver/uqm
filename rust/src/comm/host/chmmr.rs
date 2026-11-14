use crate::comm::races::chmmr::{api::ChmmrApi, Handler};
use crate::engine::StoryKey;
use crate::planets::universe_units::{universe_to_logx, universe_to_logy};
use crate::starbase::sis_modules::{ModuleType, EMPTY_MODULE_SLOT, FUEL_RESERVE};

fn refit(glob: &mut crate::engine::WorldData, starbase: bool) {
    let ship = &mut glob.flagship;
    let game = &mut glob.game;
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

impl ChmmrApi for crate::comm::host::Host<'_, '_, Handler> {
    fn refit(&mut self) {
        use crate::comm::races::script::DialogueHost;
        let starbase = self.get(StoryKey::STARBASE_AVAILABLE) != 0;

        {
            refit(&mut self.cx.borrow_mut().world.data, starbase);
        }
    }
    fn intro_mode(&mut self, mode: i32, duration: i32) {
        let comm = &mut **self.comm.borrow_mut();
        self.cx.borrow_mut().with_legacy(|| {
            use crate::comm::comm_abi::set_comm_intro_mode;
            set_comm_intro_mode(comm, mode as u32, duration as u32);
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::comm::host::Host;
    use crate::comm::races::chmmr::{api::ChmmrApi, Handler};
    use crate::comm::Comm;
    use crate::engine::StoryKey;

    #[test]
    fn starbase_refit_resets_supplies_and_installs_bomb() {
        crate::comm::with_test_context(|cx| {
            cx.world.story = Some(crate::engine::world::story::StoryState::new());
            cx.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 1);
            let ship = &mut cx.world.data.flagship;
            ship.module_slots.fill(9);
            ship.drive_slots.fill(20);
            ship.jet_slots.fill(21);
            ship.fuel_on_board = 90000;
            ship.crew_enlisted = 500;
            ship.total_element_mass = 800;
            ship.total_bio_mass = 37;
            ship.element_amounts.fill(100);
            cx.world.data.game.module_cost.fill(10);
            let mut comm = Comm::new();
            Host::new(&mut comm, cx, |_: Handler| panic!("unexpected response")).refit();
            let ship = &cx.world.data.flagship;
            assert_eq!(ship.res_units, 1_000_000);
            assert_eq!(ship.num_landers, 0);
            assert_eq!(
                (
                    ship.fuel_on_board,
                    ship.crew_enlisted,
                    ship.total_element_mass
                ),
                (1000, 0, 0)
            );
            assert_eq!(ship.total_bio_mass, 37);
            assert_eq!(ship.element_amounts, [0; 8]);
            assert_eq!(
                ship.module_slots,
                [18, 19, 17, 15, 14, 15, 17, 18, 19, 16, 22, 22, 22, 22, 22, 22]
            );
            assert_eq!(ship.drive_slots, [1; 11]);
            assert_eq!(ship.jet_slots, [2; 8]);
            assert_eq!((ship.log_x, ship.log_y), (27987, 164141));
            let game = &cx.world.data.game;
            assert_eq!((game.ship_facing, game.ip_planet, game.in_orbit), (1, 3, 0));
            assert_eq!((game.ip_location.x, game.ip_location.y), (-725, 597));
            assert_eq!(
                (game.ship_stamp.origin.x, game.ship_stamp.origin.y),
                (86, 113)
            );
            assert_eq!(
                (
                    game.module_cost[0],
                    game.module_cost[4],
                    game.module_cost[5]
                ),
                (0, 0, 10)
            );
        });
    }

    #[test]
    fn unvisited_starbase_refit_retains_supplies_and_reverse_packing() {
        crate::comm::with_test_context(|cx| {
            cx.world.story = Some(crate::engine::world::story::StoryState::new());
            cx.world.set_story_state(StoryKey::STARBASE_AVAILABLE, 0);
            let ship = &mut cx.world.data.flagship;
            ship.module_slots = [3, 22, 4, 22, 5, 6, 22, 7, 22, 8, 22, 9, 22, 10, 22, 11];
            ship.fuel_on_board = 54321;
            ship.crew_enlisted = 321;
            ship.total_element_mass = 400;
            ship.element_amounts = [50; 8];
            cx.world.data.game.module_cost.fill(8);
            let mut comm = Comm::new();
            Host::new(&mut comm, cx, |_: Handler| panic!("unexpected response")).refit();
            let ship = &cx.world.data.flagship;
            assert_eq!(
                ship.module_slots,
                [18, 19, 17, 15, 14, 15, 17, 18, 19, 16, 6, 7, 8, 9, 10, 11]
            );
            assert_eq!(
                (
                    ship.fuel_on_board,
                    ship.crew_enlisted,
                    ship.total_element_mass
                ),
                (54321, 321, 400)
            );
            assert_eq!(ship.element_amounts, [50; 8]);
            assert_eq!(cx.world.data.game.module_cost[4], 8);
            assert_eq!(
                (
                    cx.world.data.game.ship_stamp.origin.x,
                    cx.world.data.game.ship_stamp.origin.y
                ),
                (121, 113)
            );
        });
    }
}
