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
fn taalo_protector_blocks_hypnosis_on_first_and_repeat_visits() {
    for protector in [0, 1] {
        let mut f = Api::default();
        f.put("TAALO_PROTECTOR_ON_SHIP", protector);
        f.put("UMGAH_VISITS", 3);
        run(&mut f, Handler::CompelPlayer, BYE_AT_UMGAH);
        assert_eq!(f.get("KNOW_UMGAH_ZOMBIES"), 1);
        assert_eq!(f.get("PLAYER_HYPNOTIZED"), 1 - protector);
        assert_eq!(
            f.spoken(),
            if protector == 0 {
                vec![GOODBYE_AT_UMGAH]
            } else {
                vec![GOODBYE_AT_UMGAH, CANT_COMPEL]
            }
        );
        if protector != 0 {
            assert_eq!(f.segue, 1);
        }
        f.put("TALKING_PET_HOME_VISITS", 1);
        f.world.npc_ships = true;
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert_eq!(
            f.spoken(),
            if protector == 0 {
                vec![HYPNOTIZE_AGAIN_1]
            } else {
                vec![HYPNOTIZE_AGAIN_1, CANT_COMPEL]
            }
        );
        assert_eq!(f.segue, u32::from(protector));
    }
}
#[test]
fn acquisition_resets_umgah_and_arilou_state_for_all_acceptance_branches() {
    for r in [OK_LETS_DO_IT, WANT_KILL_1, WANT_KILL_2, WANT_KILL_3] {
        let mut f = Api::default();
        for k in [
            "UMGAH_ZOMBIE_BLOBBIES",
            "UMGAH_VISITS",
            "UMGAH_HOME_VISITS",
            "ARILOU_STACK_2",
            "SHIP_TO_COMPEL",
        ] {
            f.put(k, 1);
        }
        run(&mut f, Handler::ExitConversation, r);
        assert_eq!(
            f.spoken(),
            [if r == OK_LETS_DO_IT {
                COMING_ABOARD
            } else {
                GLAD_YOU_WONT_KILL
            }]
        );
        assert_eq!((f.get("TALKING_PET"), f.get("TALKING_PET_ON_SHIP")), (1, 1));
        for k in [
            "UMGAH_ZOMBIE_BLOBBIES",
            "UMGAH_VISITS",
            "UMGAH_HOME_VISITS",
            "ARILOU_STACK_2",
            "SHIP_TO_COMPEL",
        ] {
            assert_eq!(f.get(k), 0);
        }
        assert_eq!(f.segue, 0);
    }
}
#[test]
fn samatra_requires_completed_bomb_and_pet_compulsion_is_persistent() {
    for (bomb, on_ship, phrase) in [(0, 0, NEED_BOMB), (2, 1, SOUP_UP_BOMB)] {
        let mut f = Api::default();
        f.put("READY_TO_CONFUSE_URQUAN", 1);
        f.put("SHIP_TO_COMPEL", 1);
        f.put("CHMMR_BOMB_STATE", bomb);
        f.put("UTWIG_BOMB_ON_SHIP", on_ship);
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), [HAVENT_GOT_EVERYTHING, phrase]);
        assert_eq!(f.get("READY_TO_CONFUSE_URQUAN"), 0);
        assert_eq!(f.get("SHIP_TO_COMPEL"), 0);
        assert_eq!(f.get("AWARE_OF_SAMATRA"), 1);
        assert_eq!(f.segue, 0);
    }
    let mut f = Api::default();
    f.put("READY_TO_CONFUSE_URQUAN", 1);
    f.put("CHMMR_BOMB_STATE", 3);
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [I_SENSE_MY_SLAVES]);
    assert_eq!(choices(&f), [WHAT_NOW, IM_SCARED, COMPEL_URQUAN]);
    run(&mut f, Handler::ExitConversation, COMPEL_URQUAN);
    assert_eq!(f.get("URQUAN_MESSED_UP"), 1);
    f.put("READY_TO_CONFUSE_URQUAN", 1);
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [HELLO_AFTER_COMPEL_URQUAN]);
}
#[test]
fn dnyarri_lie_and_deal_choices_follow_learned_history_and_umgah_survival() {
    for learned in [0, 1] {
        let mut f = Api::default();
        f.put("LEARNED_TALKING_PET", learned);
        run(&mut f, Handler::PetDeal, WHAT_ARE_YOU_REALLY);
        assert!(choices(&f).contains(&if learned == 0 {
            HARD_TO_BELIEVE
        } else {
            BULLSHIT
        }));
    }
    let mut f = Api::default();
    run(&mut f, Handler::PetDeal, HARD_TO_BELIEVE);
    assert_eq!(f.get("DNYARRI_LIED"), 1);
    f.put("LEARNED_TALKING_PET", 1);
    run(&mut f, Handler::PetDevice, 0);
    assert!(choices(&f).contains(&YOU_LIED));
    run(&mut f, Handler::PetDevice, YOU_LIED);
    assert_eq!(f.get("DNYARRI_LIED"), 0);
    for alive in [false, true] {
        for visits in [0, 1] {
            for shield in [0, 1] {
                let mut f = Api::default();
                f.world.umgah_alive = alive;
                f.put("TALKING_PET_HOME_VISITS", visits);
                f.put("TAALO_PROTECTOR_ON_SHIP", shield);
                Dialogue { api: &mut f }.intro();
                assert_eq!(
                    f.spoken(),
                    [if alive {
                        LETS_MAKE_A_DEAL
                    } else {
                        UMGAH_ALL_GONE
                    }]
                );
                assert_eq!(
                    f.disabled.contains(&(HOW_TRUST as u16)),
                    !alive && (visits == 0 || shield == 0)
                );
                assert_eq!(f.world.tracking_calls, 1);
            }
        }
    }
}
#[test]
fn onboard_greeting_skips_shield_complaint_and_strobe_preserves_response_handler() {
    for shield in [0, 1] {
        let mut f = Api::default();
        f.put("TALKING_PET_ON_SHIP", 1);
        f.put("TAALO_PROTECTOR_ON_SHIP", shield);
        Dialogue { api: &mut f }.intro();
        assert_eq!(
            f.spoken(),
            [if shield == 0 {
                HELLO_AS_DEVICE_2
            } else {
                HELLO_AS_DEVICE_1
            }]
        );
        assert_eq!(f.get("TALKING_PET_VISITS"), if shield == 0 { 2 } else { 1 });
    }
    let mut f = Api::default();
    run(&mut f, Handler::MindControl, WHAT_ABOUT_POWERS);
    assert_eq!(f.spoken(), [NOT_POWERS_BUT_FLOWERS]);
    assert_eq!(f.offers(), [(YES_FLOWERS as u16, Handler::MindControl)]);
    assert!(f.effects.contains(&Effect::Talk(u16::MAX)));
    assert_eq!(f.world.strobes, 1);
    run(&mut f, Handler::MindControl, YES_FLOWERS);
    assert_eq!(f.spoken(), [GOOD_HUMAN]);
    assert_eq!(
        f.offers(),
        [(WISH_TO_GO_NOW as u16, Handler::ExitConversation)]
    );
    assert_eq!(f.world.strobes, 2);
}
#[test]
fn final_battle_outtakes_lifecycle_and_portrait() {
    for (flags, phrase) in [(0, HUMAN_PEP_TALK), (32, CYBORG_PEP_TALK)] {
        let mut f = Api {
            activity: 1,
            ..Api::default()
        };
        f.world.flags = flags;
        Dialogue { api: &mut f }.initialize();
        assert_eq!(f.segue, 1);
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), [phrase]);
    }
    let mut f = Api {
        activity: 5,
        ..Api::default()
    };
    Dialogue { api: &mut f }.intro();
    Dialogue { api: &mut f }.post_talkpet_enc();
    assert_eq!(f.spoken(), [OUT_TAKES]);
    assert_eq!(f.segue, 0);
    let d = descriptor();
    assert_eq!(d.alien_ambient_array[16].num_frames, 30);
    assert_eq!(d.alien_ambient_array[16].anim_flags, 178);
    assert_eq!(d.alien_ambient_array[16].base_frame_rate, 42);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(d.alien_frame_res) }
            .to_str()
            .unwrap(),
        "comm.talkingpet.graphics"
    );
}
