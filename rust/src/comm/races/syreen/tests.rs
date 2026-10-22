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
fn talana_requires_shattered_world_knowledge_and_egg_proof_before_shuttle() {
    let mut f = Api::default();
    f.put("KNOW_ABOUT_SHATTERED", 2);
    f.put("KNOW_SYREEN_WORLD_SHATTERED", 1);
    run(&mut f, Handler::NormalSyreen, 0);
    assert_eq!(choices(&f)[0], KNOW_ABOUT_DEEP_CHILDREN);
    run(&mut f, Handler::NormalSyreen, KNOW_ABOUT_DEEP_CHILDREN);
    assert_eq!(choices(&f)[0], MYCONS_INVOLVED);
    run(&mut f, Handler::NormalSyreen, MYCONS_INVOLVED);
    assert_eq!(f.get("KNOW_ABOUT_SHATTERED"), 3);
    assert_eq!(choices(&f)[0], HAVE_NO_PROOF);
    run(&mut f, Handler::NormalSyreen, HAVE_NO_PROOF);
    assert_eq!(f.get("SYREEN_WANT_PROOF"), 1);
    assert!(!choices(&f).contains(&HAVE_NO_PROOF));
    for key in [
        "EGG_CASE0_ON_SHIP",
        "EGG_CASE1_ON_SHIP",
        "EGG_CASE2_ON_SHIP",
    ] {
        f.put(key, 1);
        f.disabled.clear();
        run(&mut f, Handler::NormalSyreen, 0);
        assert_eq!(choices(&f)[0], HAVE_PROOF);
        run(&mut f, Handler::NormalSyreen, HAVE_PROOF);
        assert_eq!(choices(&f)[0], LOOK_AT_EGG_SACKS);
        f.put(key, 0);
    }
    run(&mut f, Handler::NormalSyreen, LOOK_AT_EGG_SACKS);
    assert_eq!(f.spoken(), [HORRIBLE_TRUTH]);
    assert_eq!(f.get("SYREEN_KNOW_ABOUT_MYCON"), 1);
    assert_eq!(f.get("SYREEN_HOME_VISITS"), 0);
    assert_eq!(
        choices(&f),
        [WHATS_NEXT_STEP, WHAT_DO_I_GET_FOR_THIS, NOT_SURE]
    );
}
#[test]
fn shuttle_vault_and_ambush_handoffs_keep_flags_and_choices() {
    for (known, r, phrase) in [(0, WHERE_IS_IT, DONT_KNOW_WHERE), (1, BEEN_THERE, GREAT)] {
        let mut f = Api::default();
        f.put("KNOW_SYREEN_VAULT", known);
        run(&mut f, Handler::SyreenShuttle, WHATS_NEXT_STEP);
        assert_eq!(choices(&f)[0], r);
        run(&mut f, Handler::SyreenShuttle, r);
        assert_eq!(f.spoken(), [phrase, GIVE_SHUTTLE]);
        assert_eq!(
            (f.get("SYREEN_SHUTTLE"), f.get("SYREEN_SHUTTLE_ON_SHIP")),
            (1, 1)
        );
        assert_eq!(
            choices(&f),
            [
                IM_ON_MY_WAY,
                DOING_THIS_FOR_YOU,
                IF_I_DIE,
                WHAT_DO_I_GET_FOR_THIS,
                NOT_SURE
            ]
        );
        f.put("SYREEN_KNOW_ABOUT_MYCON", 1);
        f.put("SHIP_VAULT_UNLOCKED", 1);
        f.put("SYREEN_HOME_VISITS", 1);
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), [OK_FOUND_VAULT]);
        assert_eq!(f.get("MYCON_AMBUSH"), 1);
        assert_eq!(f.get("SYREEN_HOME_VISITS"), 0);
        assert_eq!(choices(&f), [WHATS_MY_REWARD, BYE_AFTER_VAULT]);
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), [READY_FOR_AMBUSH]);
        assert_eq!(f.get("SYREEN_HOME_VISITS"), 0);
        f.put("MYCON_KNOW_AMBUSH", 1);
        f.effects.clear();
        Dialogue { api: &mut f }.intro();
        assert_eq!(f.spoken(), [HELLO_AFTER_AMBUSH_1]);
        assert!(f.effects.contains(&Effect::Allied(
            crate::comm::dispatch::ship::SYREEN_SHIP as u16,
            true
        )));
    }
}
#[test]
fn stack_priority_and_constructed_intro_and_speech_are_preserved() {
    let mut f = Api::default();
    Dialogue { api: &mut f }.intro();
    assert_eq!(
        choices(&f),
        [
            WE_ARE_VICE_SQUAD,
            WE_ARE_THE_ONE_FOR_YOU_BABY,
            WE_ARE_VINDICATOR0,
            WE_ARE_IMPRESSED
        ]
    );
    assert!(f.effects.contains(&Effect::Offer(
        WE_ARE_VINDICATOR0 as u16,
        Handler::InitialSyreen,
        vec![
            WE_ARE_VINDICATOR0,
            super::super::script::GLOBAL_PLAYER_NAME,
            WE_ARE_VINDICATOR1,
            super::super::script::GLOBAL_SHIP_NAME,
            WE_ARE_VINDICATOR2
        ]
    )));
    for speech in [false, true] {
        f.world.speech = speech;
        run(&mut f, Handler::InitialSyreen, WE_ARE_VINDICATOR0);
        assert_eq!(
            f.spoken(),
            if speech {
                vec![WELCOME_VINDICATOR0, HOW_CAN_YOU_BE_HERE]
            } else {
                vec![
                    WELCOME_VINDICATOR0,
                    super::super::script::GLOBAL_PLAYER_NAME,
                    WELCOME_VINDICATOR1,
                    super::super::script::GLOBAL_SHIP_NAME,
                    WELCOME_VINDICATOR2,
                    HOW_CAN_YOU_BE_HERE,
                ]
            }
        );
    }
    run(&mut f, Handler::NormalSyreen, WHAT_ABOUT_HISTORY);
    assert_eq!(choices(&f)[0], WHAT_ABOUT_HOMEWORLD);
    run(&mut f, Handler::NormalSyreen, WHAT_DOING_HERE);
    assert_eq!(choices(&f)[0], WHAT_ABOUT_WAR);
    run(&mut f, Handler::NormalSyreen, WHAT_ABOUT_OUTFIT);
    assert_eq!(choices(&f)[0], WHERE_MATES);
}
#[test]
fn palette_talk_order_and_sex_flags_and_outtakes() {
    let mut f = Api::default();
    run(&mut f, Handler::Foreplay, WHATS_MY_REWARD);
    assert_eq!(
        &f.effects[..6],
        &[
            Effect::Npc(HERES_REWARD),
            Effect::Npc(MORE_COMFORTABLE),
            Effect::Talk(1),
            Effect::FadePalette(1, 840),
            Effect::Talk(u16::MAX),
            Effect::Set("PLAYER_HAVING_SEX".into(), 1)
        ]
    );
    run(&mut f, Handler::FriendlyExit, HANDS_OFF);
    assert_eq!(f.spoken(), [OK_WONT_USE_HANDS, LATER, SEX_GOODBYE]);
    assert_eq!(
        (f.get("PLAYER_HAD_SEX"), f.get("PLAYER_HAVING_SEX")),
        (1, 0)
    );
    assert!(f.effects.contains(&Effect::FadePalette(0, 420)));
    f.activity = 5;
    f.effects.clear();
    Dialogue { api: &mut f }.intro();
    assert_eq!(f.spoken(), [OUT_TAKES]);
    assert!(choices(&f).is_empty());
    assert_eq!(f.segue, 0);
}

#[test]
fn lifecycle_and_descriptor() {
    let mut f = Api::default();
    let mut d = Dialogue { api: &mut f };
    d.initialize();
    d.post_syreen_enc();
    assert_eq!(descriptor().alien_text_width, 224);
}

#[test]
fn animation_flags_match_the_portrait() {
    let d = descriptor();
    assert_eq!(
        d.alien_ambient_array[..15]
            .iter()
            .map(|a| a.anim_flags)
            .collect::<Vec<_>>(),
        [1, 1, 1, 1, 1, 1, 1, 1, 1, 4, 2, 1, 4, 12, 12]
    );
    assert_eq!(d.alien_ambient_array[13].block_mask, (1 << 7) | (1 << 14));
}
