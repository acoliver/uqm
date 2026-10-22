use super::*;
use crate::comm::races::script::Supplies;
use std::collections::{HashMap, HashSet};
use std::ffi::CStr;

#[derive(Debug, PartialEq, Eq)]
enum Effect {
    Npc(i32),
    Set(String, u8),
    Offer(u16, Handler, Vec<i32>),
    Disable(u16),
    Lander,
    Gauges(i16, i16, i32),
    Palette(u16),
    Talk(u16),
    Animation(usize),
    Fade(u16, i16),
}

#[derive(Default)]
struct Fake {
    state: HashMap<String, u8>,
    disabled: HashSet<u16>,
    supplies: Supplies,
    effects: Vec<Effect>,
}

impl Fake {
    fn ready() -> Self {
        Self {
            supplies: Supplies {
                landers: 1,
                fuel: 200,
                radioactives: 0,
            },
            ..Self::default()
        }
    }
    fn run(&mut self, handler: Handler, r: u16) {
        self.effects.clear();
        Commander(self).handle(handler, r);
    }
    fn offers(&self) -> Vec<(u16, Handler)> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                Effect::Offer(p, h, _) => Some((*p, *h)),
                _ => None,
            })
            .collect()
    }
    fn spoken(&self) -> Vec<i32> {
        self.effects
            .iter()
            .filter_map(|e| match e {
                Effect::Npc(p) => Some(*p),
                _ => None,
            })
            .collect()
    }
}

impl ScriptApi<Handler> for Fake {
    fn get(&self, key: &str) -> u8 {
        *self.state.get(key).unwrap_or(&0)
    }
    fn set(&mut self, key: &str, value: u8) {
        assert!(
            crate::state::game_state_keys::bit_range(key).is_some(),
            "unknown key {key}"
        );
        self.state.insert(key.into(), value);
        self.effects.push(Effect::Set(key.into(), value));
    }
    fn npc(&mut self, phrase: i32) {
        self.effects.push(Effect::Npc(phrase));
    }
    fn response(&mut self, phrase: u16, handler: Handler, fragments: &[i32]) {
        self.effects
            .push(Effect::Offer(phrase, handler, fragments.to_vec()));
    }
    fn enabled(&self, phrase: u16) -> bool {
        !self.disabled.contains(&phrase)
    }
    fn disable(&mut self, phrase: u16) {
        self.disabled.insert(phrase);
        self.effects.push(Effect::Disable(phrase));
    }
    fn supplies(&self) -> Supplies {
        self.supplies
    }
    fn add_lander(&mut self) {
        self.supplies.landers += 1;
        self.effects.push(Effect::Lander);
    }
    fn gauges(&mut self, crew: i16, fuel: i16, units: i32) {
        self.supplies.fuel = self
            .supplies
            .fuel
            .checked_add_signed(i32::from(fuel))
            .unwrap();
        self.effects.push(Effect::Gauges(crew, fuel, units));
    }
    fn palette(&mut self, index: u16) {
        self.effects.push(Effect::Palette(index));
    }
    fn talk_segue(&mut self, track: u16) {
        self.effects.push(Effect::Talk(track));
    }
    fn disable_animation(&mut self, index: usize) {
        self.effects.push(Effect::Animation(index));
    }
    fn fade_palette(&mut self, index: u16, duration: i16) {
        self.effects.push(Effect::Fade(index, duration));
    }
}

#[test]
fn descriptor_matches_resources_layout_and_animation_timing() {
    let desc = descriptor();
    for (ptr, name) in [
        (desc.alien_frame_res, "comm.commander.graphics"),
        (desc.alien_font_res, "comm.commander.font"),
        (desc.alien_colormap_res, "comm.commander.colortable"),
        (desc.alien_song_res, "comm.commander.music"),
        (desc.conversation_phrases_res, "comm.commander.dialogue"),
    ] {
        assert_eq!(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap(), name);
    }
    assert_eq!(
        (
            desc.alien_text_baseline.x,
            desc.alien_text_baseline.y,
            desc.alien_text_width
        ),
        (164, 20, 143)
    );
    assert_eq!((desc.alien_text_align, desc.alien_text_valign), (1, 1));
    assert_eq!(
        (
            desc.alien_text_fcolor.r,
            desc.alien_text_fcolor.g,
            desc.alien_text_fcolor.b,
            desc.alien_text_fcolor.a
        ),
        (255, 255, 255, 255)
    );
    assert_eq!(
        (desc.alien_text_bcolor.r, desc.alien_text_bcolor.a),
        (0, 255)
    );
    assert_eq!(desc.num_animations, 3);
    let actual = desc.alien_ambient_array[..3]
        .iter()
        .map(|a| {
            (
                a.start_index,
                a.num_frames,
                a.anim_flags,
                a.base_frame_rate,
                a.random_frame_rate,
                a.base_restart_rate,
                a.random_restart_rate,
                a.block_mask,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        vec![
            (1, 3, 4, 56, 0, 0, 6720, 0),
            (10, 30, 2, 21, 0, 1680, 0, 0),
            (1, 3, 17, 0, 28, 0, 56, 0)
        ]
    );
    let talk = desc.alien_talk_desc;
    assert_eq!(
        (
            talk.start_index,
            talk.num_frames,
            talk.base_frame_rate,
            talk.random_frame_rate,
            talk.base_restart_rate,
            talk.random_restart_rate
        ),
        (4, 6, 84, 56, 98, 70)
    );
    assert!(desc.alien_frame.is_null());
    assert!(desc.alien_alt_song_res.is_null());
}

#[test]
fn descriptor_reentry_switches_power_and_preserves_other_flags() {
    let mut desc = descriptor();
    desc.alien_song_flags = 4;
    configure(&mut desc, true);
    assert_eq!(desc.alien_ambient_array[2].anim_flags, 17 | 128);
    assert!(desc.alien_alt_song_res.is_null());
    configure(&mut desc, false);
    assert_eq!(desc.alien_ambient_array[2].anim_flags, 17);
    assert_eq!(desc.alien_song_flags, 5);
    assert_eq!(
        unsafe { CStr::from_ptr(desc.alien_alt_song_res) },
        c"comm.commander.lowpower.music"
    );
    configure(&mut desc, true);
    assert_eq!(desc.alien_song_flags, 4);
}

#[test]
fn first_visit_constructs_ship_name_and_preserves_choice_order() {
    let mut f = Fake::ready();
    Commander(&mut f).intro();
    assert_eq!(
        f.effects,
        vec![
            Effect::Palette(1),
            Effect::Set("STARBASE_VISITED".into(), 1),
            Effect::Npc(4),
            Effect::Offer(58, Handler::NoRadioactives, vec![58, GLOBAL_SHIP_NAME, 59]),
            Effect::Offer(60, Handler::NoRadioactives, vec![]),
            Effect::Offer(61, Handler::NoRadioactives, vec![]),
        ]
    );
}

#[test]
fn returning_intro_routes_all_game_state_branches() {
    for (provided, moon, radios, spoken, offers) in [
        (0, 0, 0, vec![5], vec![54, 53]),
        (0, 0, 1, vec![5], vec![55, 56, 57]),
        (1, 0, 0, vec![33], vec![77, 76]),
        (1, 1, 0, vec![33], vec![79, 80]),
    ] {
        let mut f = Fake::ready();
        f.state.extend([
            ("STARBASE_VISITED".into(), 1),
            ("RADIOACTIVES_PROVIDED".into(), provided),
            ("MOONBASE_DESTROYED".into(), moon),
        ]);
        f.supplies.radioactives = radios;
        Commander(&mut f).intro();
        assert_eq!(f.spoken(), spoken);
        assert_eq!(
            f.offers().iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            offers
        );
        assert_eq!(f.effects.contains(&Effect::Palette(1)), provided == 0);
    }
    let mut f = Fake::ready();
    f.state.insert("PROBE_ILWRATH_ENCOUNTER".into(), 1);
    Commander(&mut f).intro();
    assert_eq!(f.spoken(), vec![35]);
    assert_eq!(
        f.offers(),
        vec![
            (52, Handler::ByeBye),
            (50, Handler::ByeBye),
            (51, Handler::ByeBye)
        ]
    );
}

#[test]
fn supply_claim_and_radioactive_possession_routes() {
    for radios in [0, 1] {
        let mut f = Fake::ready();
        f.supplies.radioactives = radios;
        f.run(Handler::NoRadioactives, YES_THIS_IS_SUPPLY_SHIP);
        assert_eq!(f.spoken(), vec![8]);
        assert_eq!(
            f.offers().iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            if radios == 0 {
                vec![62, 63]
            } else {
                vec![55, 56, 57]
            }
        );
        f.run(Handler::NoRadioactives, NO_BUT_WELL_HELP0);
        assert_eq!(f.spoken(), vec![7]);
        assert_eq!(
            f.offers().iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            if radios == 0 {
                vec![70, 69]
            } else {
                vec![55, 56, 57]
            }
        );
    }
    for (input, output) in [(WHAT_SLAVE_PLANET, 21), (I_LIED, 21), (PLUMB_OUT, 20)] {
        let mut f = Fake::ready();
        f.run(Handler::NoRadioactives, input);
        assert_eq!(f.spoken(), vec![output]);
    }
}

#[test]
fn mercury_hint_is_disabled_in_each_radio_menu() {
    for (handler, phrase, exit) in [
        (
            Handler::NoRadioactives,
            WHERE_CAN_I_GET_RADIOS,
            OK_I_WILL_GET_RADIOS,
        ),
        (
            Handler::AskAfterRadios,
            WHERE_GET_RADIOS,
            WELL_GO_GET_THEM_NOW,
        ),
    ] {
        let mut f = Fake::ready();
        f.run(handler, phrase);
        assert_eq!(f.spoken(), vec![24]);
        assert!(!f.enabled(phrase));
        assert_eq!(f.offers(), vec![(exit, Handler::ByeBye)]);
    }
}

#[test]
fn rescue_effects_and_offers_match_all_three_menus() {
    for handler in [
        Handler::NoRadioactives,
        Handler::AskAfterRadios,
        Handler::TellMoonBase,
    ] {
        for (input, output, state) in [
            (I_LOST_MY_LANDER, 11, Some("LANDERS_LOST")),
            (I_LOST_ANOTHER_LANDER, 6, None),
            (NEED_FUEL_MERCURY, 2, Some("GIVEN_FUEL_BEFORE")),
            (NEED_FUEL_LUNA, 2, Some("GIVEN_FUEL_BEFORE")),
            (NEED_FUEL_AGAIN, 3, None),
        ] {
            let mut f = Fake::ready();
            f.run(handler, input);
            assert_eq!(f.spoken(), vec![output]);
            let lander = matches!(input, I_LOST_MY_LANDER | I_LOST_ANOTHER_LANDER);
            let mut expected = vec![Effect::Npc(output)];
            if lander {
                expected.push(Effect::Lander);
            }
            expected.push(if lander {
                Effect::Gauges(4, 0, 0)
            } else {
                Effect::Gauges(0, 500, 0)
            });
            if let Some(key) = state {
                expected.push(Effect::Set(key.into(), 1));
            }
            assert_eq!(&f.effects[..expected.len()], &expected);
        }
        for bomb in [0, 1, 2, 3] {
            for fuel in [199, 200] {
                for repeat in [0, 1] {
                    let mut f = Fake::ready();
                    f.supplies = Supplies {
                        landers: 0,
                        fuel,
                        radioactives: 0,
                    };
                    f.state.extend([
                        ("CHMMR_BOMB_STATE".into(), bomb),
                        ("LANDERS_LOST".into(), repeat),
                        ("GIVEN_FUEL_BEFORE".into(), repeat),
                    ]);
                    f.run(handler, 0);
                    let offers = f.offers();
                    let mut expected = Vec::new();
                    if bomb < 2 {
                        expected.push((
                            if repeat == 0 {
                                I_LOST_MY_LANDER
                            } else {
                                I_LOST_ANOTHER_LANDER
                            },
                            handler,
                        ));
                    }
                    if fuel < 200 {
                        expected.push((
                            if repeat != 0 {
                                NEED_FUEL_AGAIN
                            } else if handler == Handler::TellMoonBase {
                                NEED_FUEL_LUNA
                            } else {
                                NEED_FUEL_MERCURY
                            },
                            handler,
                        ));
                    }
                    assert_eq!(&offers[..expected.len()], &expected);
                    assert!(!offers.iter().any(|(p, _)| matches!(
                        *p,
                        I_LOST_MY_LANDER | I_LOST_ANOTHER_LANDER
                    ) && bomb >= 2));
                    assert!(!offers.iter().any(|(p, _)| matches!(
                        *p,
                        NEED_FUEL_MERCURY | NEED_FUEL_LUNA | NEED_FUEL_AGAIN
                    ) && fuel >= 200));
                }
            }
        }
    }
}

#[test]
fn moonbase_information_stack_and_destroy_promise() {
    let mut f = Fake::ready();
    for (input, output, last) in [
        (0, 33, 76),
        (WE_ARE_HERE_TO_HELP, 29, 76),
        (TELL_ME_ABOUT_BASE, 31, 78),
        (TELL_ME_AGAIN, 44, 78),
    ] {
        f.run(Handler::TellMoonBase, input);
        assert_eq!(f.spoken(), vec![output]);
        assert_eq!(
            f.offers(),
            vec![(77, Handler::ByeBye), (last, Handler::TellMoonBase)]
        );
    }
    f.run(Handler::ByeBye, WE_WILL_TAKE_CARE_OF_BASE);
    assert_eq!(f.get("WILL_DESTROY_BASE"), 1);
    f.run(Handler::TellMoonBase, 0);
    assert_eq!(f.offers()[0], (98, Handler::ByeBye));
}

#[test]
fn destroyed_base_and_ilwrath_responses() {
    let mut f = Fake::ready();
    for (r, output) in [(0, 33), (WE_ARE_HERE_TO_HELP, 29)] {
        f.run(Handler::BaseDestroyed, r);
        assert_eq!(f.spoken(), vec![output]);
        assert_eq!(
            f.offers(),
            vec![(79, Handler::ByeBye), (80, Handler::BaseDestroyed)]
        );
    }
    f.run(Handler::BaseDestroyed, WE_FOUGHT_THEM);
    assert_eq!(f.spoken(), vec![37]);
    assert_eq!(
        f.offers(),
        vec![
            (81, Handler::ByeBye),
            (82, Handler::ByeBye),
            (83, Handler::ByeBye)
        ]
    );
    for (r, output) in [
        (BASE_WAS_ABANDONED, 36),
        (I_LIED_IT_WAS_ABANDONED, 36),
        (OH_YES_BIG_FIGHT, 38),
        (I_CANT_TALK_ABOUT_IT, 39),
    ] {
        f.run(Handler::ByeBye, r);
        assert_eq!(f.spoken(), vec![output, 34]);
        assert_eq!(f.get("PROBE_ILWRATH_ENCOUNTER"), 1);
        assert!(f.offers().is_empty());
    }
}

#[test]
fn alliance_selection_and_custom_commander_name() {
    for (r, reply) in [
        (COOK_THEIR_BUTTS, 45),
        (OVERTHROW_EVIL_ALIENS, 46),
        (ANNIHILATE_THOSE_MONSTERS, 47),
    ] {
        let mut f = Fake::ready();
        f.run(Handler::ByeBye, r);
        assert_eq!(f.spoken(), vec![reply, 12]);
        assert_eq!(f.get("PROBE_ILWRATH_ENCOUNTER"), 0);
        assert_eq!(f.get("STARBASE_AVAILABLE"), 1);
        assert_eq!(
            f.offers(),
            vec![
                (84, Handler::ByeBye),
                (85, Handler::ByeBye),
                (86, Handler::ByeBye),
                (87, Handler::ByeBye)
            ]
        );
        assert!(f.effects.contains(&Effect::Offer(
            87,
            Handler::ByeBye,
            vec![87, GLOBAL_PLAYER_NAME, 88]
        )));
    }
    for (name, value, reply) in [
        (NAME_1, 0, vec![13]),
        (NAME_2, 1, vec![14]),
        (NAME_3, 2, vec![15]),
        (NAME_40, 3, vec![16, GLOBAL_PLAYER_NAME, 17]),
    ] {
        let mut f = Fake::ready();
        f.run(Handler::ByeBye, name);
        assert_eq!(f.get("NEW_ALLIANCE_NAME"), value);
        let mut expected = reply;
        expected.push(49);
        assert_eq!(f.spoken(), expected);
    }
    for (r, reply) in [
        (OK_I_WILL_GET_RADIOS, 25),
        (WELL_GO_GET_THEM_NOW, 1),
        (TAKE_CARE_OF_BASE_AGAIN, 48),
    ] {
        let mut f = Fake::ready();
        f.run(Handler::ByeBye, r);
        assert_eq!(f.effects, vec![Effect::Npc(reply)]);
    }
}

#[test]
fn reveal_self_rotates_stacks_and_reaches_each_destination() {
    let mut f = Fake::ready();
    f.run(Handler::RevealSelf, 0);
    assert_eq!(
        f.offers(),
        vec![(64, Handler::RevealSelf), (67, Handler::RevealSelf)]
    );
    assert!(f.effects.contains(&Effect::Offer(
        64,
        Handler::RevealSelf,
        vec![64, GLOBAL_PLAYER_NAME, 65, GLOBAL_SHIP_NAME, 66]
    )));
    for (r, reply, offers) in [
        (FIRST_GIVE_INFO, 23, vec![71, 64]),
        (WHATS_THIS_STARBASE, 26, vec![72, 64]),
        (WHAT_ABOUT_EARTH, 27, vec![73, 64]),
        (WHERE_ARE_URQUAN, 28, vec![94, 64]),
        (WE_ARE_VINDICATOR0, 22, vec![74, 94]),
        (OUR_MISSION_WAS_SECRET, 30, vec![75, 94]),
    ] {
        f.run(Handler::RevealSelf, r);
        assert_eq!(f.spoken(), vec![reply]);
        assert!(!f.enabled(r));
        assert_eq!(
            f.offers().iter().map(|(p, _)| *p).collect::<Vec<_>>(),
            offers
        );
    }
    f.run(Handler::TellProbe, WHAT_WAS_RED_THING);
    assert_eq!(f.spoken(), vec![40]);
    assert_eq!(
        f.offers(),
        vec![
            (95, Handler::RevealSelf),
            (96, Handler::RevealSelf),
            (97, Handler::RevealSelf)
        ]
    );
    assert!(!f.enabled(WHAT_WAS_RED_THING));
    for moon in [0, 1] {
        f.state.insert("MOONBASE_DESTROYED".into(), moon);
        for (r, reply) in [(IT_WENT_AWAY, 41), (WE_DESTROYED_IT, 42), (WHAT_PROBE, 43)] {
            f.run(Handler::RevealSelf, r);
            assert_eq!(f.spoken(), vec![reply]);
            assert_eq!(
                f.offers(),
                vec![(
                    75,
                    if moon == 0 {
                        Handler::TellMoonBase
                    } else {
                        Handler::BaseDestroyed
                    }
                )]
            );
        }
    }
}

#[test]
fn radio_transfer_preserves_segue_palette_and_dialogue_order() {
    let mut f = Fake::ready();
    for (r, reply) in [
        (0, vec![]),
        (WHAT_WILL_YOU_GIVE_US, vec![9]),
        (BEFORE_RADIOS_WE_NEED_INFO, vec![10]),
    ] {
        f.run(Handler::GiveRadios, r);
        assert_eq!(f.spoken(), reply);
        assert_eq!(
            f.offers(),
            vec![
                (55, Handler::GiveRadios),
                (56, Handler::GiveRadios),
                (57, Handler::GiveRadios)
            ]
        );
    }
    f.run(Handler::GiveRadios, WE_WILL_TRANSFER_NOW);
    assert_eq!(
        &f.effects[..7],
        &[
            Effect::Set("RADIOACTIVES_PROVIDED".into(), 1),
            Effect::Npc(18),
            Effect::Npc(19),
            Effect::Talk(1),
            Effect::Animation(2),
            Effect::Fade(0, 420),
            Effect::Talk(u16::MAX),
        ]
    );
    assert_eq!(
        f.offers(),
        vec![(64, Handler::RevealSelf), (67, Handler::RevealSelf)]
    );
}
