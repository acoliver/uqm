use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
type Api = Fake<Handler, api::World>;
fn run(f: &mut Api, h: Handler, r: i32) {
    f.effects.clear();
    Dialogue { api: f }.handle(h, r);
}
fn choices(f: &Api) -> Vec<i32> {
    f.offers().iter().map(|x| i32::from(x.0)).collect()
}
#[test]
fn freed_chmmr_analyze_pet_and_bomb_before_offering_refit() {
    for pet in [0, 1] {
        for bomb in [0, 1] {
            let mut f = Api::default();
            f.put("TALKING_PET_ON_SHIP", pet);
            f.put("UTWIG_BOMB_ON_SHIP", bomb);
            run(&mut f, Handler::ChmmrFree, SERIOUS_1);
            assert_eq!(
                f.spoken(),
                [
                    WILL_HELP_ANALYZE_LOGS,
                    DONT_KNOW_ABOUT_SAMATRA,
                    if pet == 1 {
                        HAVE_TALKING_PET
                    } else {
                        NEED_DISTRACTION
                    },
                    if bomb == 1 { HAVE_BOMB } else { NEED_WEAPON },
                    if pet == 1 && bomb == 1 {
                        WE_WILL_IMPROVE_BOMB
                    } else {
                        RETURN_WHEN_READY
                    }
                ]
            );
            assert_eq!(f.get("AWARE_OF_SAMATRA"), 1);
            assert_eq!(choices(&f).contains(&PROCEED), pet == 1 && bomb == 1);
            assert_eq!(f.world.refits, 0);
        }
    }
}
#[test]
fn shielded_advice_and_emergence_keep_state_and_animation_order() {
    let mut f = Api::default();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [WHY_YOU_HERE_1]);
    assert!(f.effects.contains(&Effect::Palette(1)));
    assert_eq!(
        choices(&f),
        [NEED_HELP, FIND_OUT_WHATS_UP, NEED_ADVICE, BYE_SHIELDED]
    );
    run(&mut f, Handler::ChmmrAdvice, HOW_DEFEAT_URQUAN);
    assert_eq!(f.get("CHMMR_BOMB_STATE"), 1);
    assert!(f.disabled.contains(&(NEED_ADVICE as u16)));
    f.put("CHMMR_EMERGING", 1);
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [WE_ARE_FREE, WHY_HAVE_YOU_FREED_US]);
    assert_eq!(f.get("CHMMR_UNLEASHED"), 1);
    assert_eq!(f.get("CHMMR_EMERGING"), 0);
    assert_eq!(f.get("CHMMR_HOME_VISITS"), 0);
    assert_eq!(f.world.intros, [(3, 1680)]);
    assert_eq!(choices(&f), [SERIOUS_1, SERIOUS_2, SILLY]);
    let mut f = Api::default();
    f.put("CHMMR_EMERGING", 1);
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [WE_ARE_FREE, WHO_ARE_YOU]);
    assert_eq!(choices(&f), [I_AM_CAPTAIN0, I_AM_SAVIOR, I_AM_SILLY]);
    assert!(f.effects.contains(&Effect::Offer(
        I_AM_CAPTAIN0 as u16,
        Handler::ChmmrFree,
        vec![
            I_AM_CAPTAIN0,
            super::super::script::GLOBAL_PLAYER_NAME,
            I_AM_CAPTAIN1,
            super::super::script::GLOBAL_SHIP_NAME,
            I_AM_CAPTAIN2
        ]
    )));
}
#[test]
fn proceed_refits_and_transports_to_starbase_without_marking_bomb_processed() {
    let mut f = Api::default();
    f.put("STARBASE_AVAILABLE", 1);
    f.put("UTWIG_BOMB_ON_SHIP", 1);
    f.put("CHMMR_HOME_VISITS", 3);
    f.put("CHMMR_STACK", 3);
    let ship = &mut f.world.glob.sis_state;
    ship.module_slots.fill(9);
    ship.drive_slots.fill(20);
    ship.jet_slots.fill(21);
    ship.fuel_on_board = 90000;
    ship.crew_enlisted = 500;
    ship.total_element_mass = 800;
    ship.total_bio_mass = 37;
    ship.element_amounts.fill(100);
    f.world.glob.game_state.module_cost.fill(10);
    run(&mut f, Handler::ExitConversation, PROCEED);
    assert_eq!(f.spoken(), [TAKE_2_WEEKS]);
    assert_eq!(f.world.refits, 1);
    assert_eq!(f.get("CHMMR_BOMB_STATE"), 2);
    assert_eq!(f.get("UTWIG_BOMB_ON_SHIP"), 0);
    assert_eq!(f.get("CHMMR_HOME_VISITS"), 0);
    assert_eq!(f.get("CHMMR_STACK"), 0);
    assert!(f.effects.contains(&Effect::Allied(
        crate::comm::dispatch::ship::CHMMR_SHIP as u16,
        true
    )));
    let ship = &f.world.glob.sis_state;
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
    let game = &f.world.glob.game_state;
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
}
#[test]
fn no_starbase_refit_preserves_supplies_and_packs_modules_in_original_reverse_order() {
    let mut f = Api::default();
    let ship = &mut f.world.glob.sis_state;
    ship.module_slots = [3, 22, 4, 22, 5, 6, 22, 7, 22, 8, 22, 9, 22, 10, 22, 11];
    ship.fuel_on_board = 54321;
    ship.crew_enlisted = 321;
    ship.total_element_mass = 400;
    ship.element_amounts = [50; 8];
    f.world.glob.game_state.module_cost.fill(8);
    run(&mut f, Handler::ExitConversation, PROCEED);
    let ship = &f.world.glob.sis_state;
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
    assert_eq!(f.world.glob.game_state.module_cost[4], 8);
    assert_eq!(
        (
            f.world.glob.game_state.ship_stamp.origin.x,
            f.world.glob.game_state.ship_stamp.origin.y
        ),
        (121, 113)
    );
}
#[test]
fn completed_bomb_states_use_post_bomb_dialogue_and_lifecycle_is_peaceful() {
    for bomb in [2, 3] {
        let mut f = Api::default();
        f.put("CHMMR_BOMB_STATE", bomb);
        Dialogue { api: &mut f }.initialize();
        Dialogue { api: &mut f }.intro();
        Dialogue { api: &mut f }.post_chmmr_enc();
        assert_eq!(f.spoken(), [HELLO_AFTER_BOMB_1]);
        assert_eq!(
            choices(&f),
            [WHATS_UP_AFTER_BOMB, WHAT_DO_AFTER_BOMB, BYE_AFTER_BOMB]
        );
        assert_eq!(f.get("CHMMR_BOMB_STATE"), bomb);
        assert_eq!(f.segue, 0);
    }
    let d = descriptor();
    assert_eq!(d.alien_talk_desc.base_frame_rate, 14);
    assert_eq!(
        d.alien_ambient_array[..6]
            .iter()
            .map(|a| a.anim_flags)
            .collect::<Vec<_>>(),
        [2, 2, 2, 4, 4, 4]
    );
}
