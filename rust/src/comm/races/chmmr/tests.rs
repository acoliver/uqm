use super::descriptor::descriptor;
use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::DialogueHost;
use crate::engine::StoryKey;
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
            f.put(StoryKey::TALKING_PET_ON_SHIP, pet);
            f.put(StoryKey::UTWIG_BOMB_ON_SHIP, bomb);
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
            assert_eq!(f.get(StoryKey::AWARE_OF_SAMATRA), 1);
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
    assert_eq!(f.get(StoryKey::CHMMR_BOMB_STATE), 1);
    assert!(f.disabled.contains(&(NEED_ADVICE as u16)));
    f.put(StoryKey::CHMMR_EMERGING, 1);
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [WE_ARE_FREE, WHY_HAVE_YOU_FREED_US]);
    assert_eq!(f.get(StoryKey::CHMMR_UNLEASHED), 1);
    assert_eq!(f.get(StoryKey::CHMMR_EMERGING), 0);
    assert_eq!(f.get(StoryKey::CHMMR_HOME_VISITS), 0);
    assert_eq!(f.world.intros, [(3, 1680)]);
    assert_eq!(choices(&f), [SERIOUS_1, SERIOUS_2, SILLY]);
    let mut f = Api::default();
    f.put(StoryKey::CHMMR_EMERGING, 1);
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
    f.put(StoryKey::STARBASE_AVAILABLE, 1);
    f.put(StoryKey::UTWIG_BOMB_ON_SHIP, 1);
    f.put(StoryKey::CHMMR_HOME_VISITS, 3);
    f.put(StoryKey::CHMMR_STACK, 3);
    run(&mut f, Handler::ExitConversation, PROCEED);
    assert_eq!(f.spoken(), [TAKE_2_WEEKS]);
    assert_eq!(f.world.refits, 1);
    assert_eq!(f.get(StoryKey::CHMMR_BOMB_STATE), 2);
    assert_eq!(f.get(StoryKey::UTWIG_BOMB_ON_SHIP), 0);
    assert_eq!(f.get(StoryKey::CHMMR_HOME_VISITS), 0);
    assert_eq!(f.get(StoryKey::CHMMR_STACK), 0);
    assert!(f.effects.contains(&Effect::Allied(
        crate::engine::species::ship::CHMMR_SHIP as u16,
        true
    )));
}
#[test]
fn no_starbase_requests_refit_without_changing_dialogue_order() {
    let mut f = Api::default();
    run(&mut f, Handler::ExitConversation, PROCEED);
    assert_eq!(f.spoken(), [TAKE_2_WEEKS]);
    assert_eq!(f.world.refits, 1);
    assert_eq!(f.get(StoryKey::CHMMR_BOMB_STATE), 2);
}
#[test]
fn completed_bomb_states_use_post_bomb_dialogue_and_lifecycle_is_peaceful() {
    for bomb in [2, 3] {
        let mut f = Api::default();
        f.put(StoryKey::CHMMR_BOMB_STATE, bomb);
        Dialogue { api: &mut f }.initialize();
        Dialogue { api: &mut f }.intro();
        Dialogue { api: &mut f }.post_chmmr_enc();
        assert_eq!(f.spoken(), [HELLO_AFTER_BOMB_1]);
        assert_eq!(
            choices(&f),
            [WHATS_UP_AFTER_BOMB, WHAT_DO_AFTER_BOMB, BYE_AFTER_BOMB]
        );
        assert_eq!(f.get(StoryKey::CHMMR_BOMB_STATE), bomb);
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
