use super::*;
use crate::comm::races::encounter::testing::{Effect, Fake};
use crate::comm::races::script::ScriptApi;
use std::collections::VecDeque;

struct World {
    date: Date,
    ship: Ship,
    minerals: [u16; 8],
    worth: [u8; 8],
    fleet: u16,
    speech: bool,
    random: VecDeque<u32>,
    random_calls: usize,
    skip: bool,
}
impl Default for World {
    fn default() -> Self {
        Self {
            date: Date {
                year: 2155,
                month: 2,
                day: 17,
            },
            ship: Ship {
                drives: [20; 11],
                jets: [20; 8],
                modules: [20; 16],
                ..Ship::default()
            },
            minerals: [0; 8],
            worth: [1, 2, 3, 4, 5, 6, 8, 25],
            fleet: 0,
            speech: false,
            random: VecDeque::new(),
            random_calls: 0,
            skip: false,
        }
    }
}
type Api = Fake<Handler, World>;
impl StarbaseApi for Api {
    fn date(&self) -> Date {
        self.world.date
    }
    fn ship(&self) -> Ship {
        self.world.ship
    }
    fn escorts_worth(&self) -> u16 {
        self.world.fleet
    }
    fn using_speech(&self) -> bool {
        self.world.speech
    }
    fn random(&mut self) -> u32 {
        self.world.random_calls += 1;
        self.world.random.pop_front().expect("unexpected RNG call")
    }
    fn talking_animation(&mut self, e: bool) {
        self.effects.push(Effect::Talking(e));
    }
    fn mineral_amount(&self, i: usize) -> u16 {
        self.world.minerals[i]
    }
    fn mineral_worth(&self, i: usize) -> u8 {
        self.world.worth[i]
    }
    fn remove_minerals(&mut self, i: usize, n: u16) {
        self.world.minerals[i] -= n;
        self.world.ship.mineral_mass -= n;
        self.effects.push(Effect::Remove(i, n));
    }
    fn draw_cargo(&mut self, o: u8, n: u8) {
        self.effects.push(Effect::DrawCargo(o, n));
    }
    fn show_capacity(&mut self) {
        self.effects.push(Effect::Capacity);
    }
    fn clear_display(&mut self) {
        self.effects.push(Effect::Clear);
    }
    fn now(&mut self) -> u32 {
        self.effects.push(Effect::Now);
        100
    }
    fn sleep(&mut self, t: u32) {
        self.effects.push(Effect::Sleep(t));
    }
    fn sleep_until(&mut self, t: u32) {
        self.effects.push(Effect::Until(t));
    }
    fn task_switch(&mut self) {
        self.effects.push(Effect::Switch);
    }
    fn skip_sale(&mut self) -> bool {
        self.effects.push(Effect::Skip);
        self.world.skip
    }
}
fn ready() -> Api {
    let mut f = Api::default();
    f.put("STARBASE_MONTH", 2);
    f.put("STARBASE_DAY", 17);
    f
}
fn run(f: &mut Api, s: &mut Session, h: Handler, r: u16) {
    f.effects.clear();
    Dialogue { api: f, session: s }.handle(h, r);
}
fn choices(f: &Api) -> Vec<u16> {
    f.offers().iter().map(|(p, _)| *p).collect()
}
fn state32(f: &Api, key: &str) -> u32 {
    let prefix = key.strip_suffix('0').unwrap();
    u32::from_le_bytes(std::array::from_fn(|i| f.get(&format!("{prefix}{i}"))))
}

#[test]
fn descriptor_initialization_and_post_encounter() {
    let mut f = ready();
    let mut s = Session {
        bulletins: u32::MAX,
        ..Session::default()
    };
    let mut d = descriptor();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .initialize(&mut d);
    assert_eq!(s.bulletins, 0);
    assert_eq!(f.segue, 0);
    assert_eq!(d.num_animations, 10);
    assert_eq!(
        (
            d.alien_text_baseline.x,
            d.alien_text_baseline.y,
            d.alien_text_width
        ),
        (164, 20, 143)
    );
    assert_eq!(d.alien_ambient_array[9].start_index, 95);
    assert_eq!(d.alien_talk_desc.base_restart_rate, 98);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(d.alien_alt_song_res) },
        c"comm.starbase.music"
    );
    assert_eq!(d.alien_song_flags, 1);
    assert_eq!(
        unsafe { std::ffi::CStr::from_ptr(d.conversation_phrases_res) },
        c"comm.starbase.dialogue"
    );
    for bomb in [0, 1, 2, 3] {
        f.put("MOONBASE_ON_SHIP", 1);
        f.put("CHMMR_BOMB_STATE", bomb);
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .post_starbase_enc();
        assert_eq!(f.get("MOONBASE_ON_SHIP"), 0);
        assert_eq!(f.get("CHMMR_BOMB_STATE"), if bomb == 2 { 3 } else { bomb });
    }
}
#[test]
fn random_greetings_and_farewells_keep_name_and_rng_byte_truncation() {
    for i in 0..8 {
        for speech in [false, true] {
            let mut f = ready();
            let mut s = Session::default();
            f.world.speech = speech;
            f.world.random.push_back(0x100 + i);
            Dialogue {
                api: &mut f,
                session: &mut s,
            }
            .intro();
            let mut expected = vec![8 + 2 * i as i32];
            if !speech {
                expected.extend([300, GLOBAL_PLAYER_NAME]);
            }
            expected.push(9 + 2 * i as i32);
            assert_eq!(f.spoken(), expected);
            assert_eq!(f.world.random_calls, 1);
            assert_eq!(f.get("STARBASE_VISITED"), 1);
            assert_eq!(choices(&f), vec![237, 235]);
            f.world.random.push_back(i);
            run(&mut f, &mut s, Handler::ByeBye, GOODBYE_COMMANDER);
            let mut expected = vec![26 + 2 * i as i32];
            if !speech {
                expected.extend([300, GLOBAL_PLAYER_NAME]);
            }
            expected.push(27 + 2 * i as i32);
            assert_eq!(f.spoken(), expected);
        }
    }
}
#[test]
fn completed_bomb_farewell_commits_current_bulletins_without_rng() {
    let mut f = ready();
    let mut s = Session {
        bulletins: 0x12345678,
        ..Session::default()
    };
    f.put("STARBASE_BULLETS0", 1);
    f.put("CHMMR_BOMB_STATE", 2);
    run(&mut f, &mut s, Handler::ByeBye, GOODBYE_COMMANDER);
    assert_eq!(state32(&f, "STARBASE_BULLETS0"), 0x12345679);
    assert_eq!(f.spoken(), vec![215]);
    assert_eq!(f.world.random_calls, 0);
}
#[test]
fn moonbase_welcome_sets_supplies_and_start_date_with_speech_variants() {
    for speech in [false, true] {
        let mut f = ready();
        let mut s = Session::default();
        f.world.speech = speech;
        f.world.date = Date {
            year: 2155,
            month: 4,
            day: 3,
        };
        f.put("MOONBASE_ON_SHIP", 1);
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .intro();
        assert_eq!(
            f.spoken(),
            if speech {
                vec![84, 298, 85, 297, 86]
            } else {
                vec![84, 296, GLOBAL_SHIP_NAME, 85, GLOBAL_SHIP_NAME, 86]
            }
        );
        assert!(f.effects.contains(&Effect::Gauges(0, 0, 2500)));
        assert_eq!(f.get("STARBASE_MONTH"), 4);
        assert_eq!(f.get("STARBASE_DAY"), 3);
        assert_eq!(f.get("STARBASE_VISITED"), 1);
    }
}
#[test]
fn starbase_functions_keep_ship_name_fragments_and_each_stack() {
    let mut f = ready();
    let mut s = Session::default();
    run(&mut f, &mut s, Handler::TellStarBase, STARBASE_FUNCTIONS);
    assert_eq!(f.spoken(), vec![88]);
    assert_eq!(choices(&f), vec![246, 244, 249, 250, 242]);
    assert!(f.effects.contains(&Effect::Offer(
        246,
        Handler::TellStarBase,
        vec![246, GLOBAL_SHIP_NAME, 247]
    )));
    assert!(f.effects.contains(&Effect::Offer(
        244,
        Handler::TellStarBase,
        vec![244, GLOBAL_SHIP_NAME, 245]
    )));
    for (r, p) in [
        (TELL_ME_ABOUT_MODULES0, 93),
        (TELL_ME_ABOUT_FUEL0, 92),
        (TELL_ME_ABOUT_SHIPS, 96),
        (TELL_ME_ABOUT_RU, 97),
        (TELL_ME_ABOUT_MINERALS, 98),
        (TELL_ME_ABOUT_LIFE, 99),
    ] {
        run(&mut f, &mut s, Handler::TellStarBase, r);
        assert_eq!(f.spoken(), vec![p]);
    }
    run(&mut f, &mut s, Handler::TellStarBase, TELL_ME_ABOUT_CREW);
    assert_eq!(f.spoken(), vec![94, 296, GLOBAL_SHIP_NAME, 95]);
    assert_eq!(choices(&f), vec![242]);
    f.world.speech = true;
    run(&mut f, &mut s, Handler::TellStarBase, TELL_ME_ABOUT_CREW);
    assert_eq!(f.spoken(), vec![94, 299, 95]);
}
#[test]
fn information_menus_route_to_all_submenus_and_back() {
    let mut f = ready();
    let mut s = Session::default();
    for (r, p) in [
        (NEED_INFO, 87),
        (ENOUGH_STARBASE, 100),
        (ENOUGH_HISTORY, 114),
        (ENOUGH_MISSION, 101),
    ] {
        run(&mut f, &mut s, Handler::NeedInfo, r);
        assert_eq!(f.spoken(), vec![p]);
        assert_eq!(choices(&f), vec![238, 239, 240, 241]);
    }
    run(&mut f, &mut s, Handler::NormalStarbase, NO_NEED_INFO);
    assert_eq!(f.spoken(), vec![91]);
    for (h, start, items, exit) in [
        (
            Handler::AllianceInfo,
            WHAT_ABOUT_ALLIANCE,
            vec![SHOFIXTI, YEHAT, ARILOU, CHENJESU, MMRNMHRM, SYREEN],
            ENOUGH_ALLIANCE,
        ),
        (
            Handler::HierarchyInfo,
            WHAT_ABOUT_HIERARCHY,
            vec![URQUAN, MYCON, SPATHI, UMGAH, ANDROSYNTH, ILWRATH, VUX],
            ENOUGH_HIERARCHY,
        ),
        (
            Handler::WarInfo,
            THE_WAR,
            vec![
                WHAT_STARTED_WAR,
                WHAT_WAS_WAR_LIKE,
                WHY_LOSE_WAR,
                WHAT_AFTER_WAR,
            ],
            ENOUGH_WAR,
        ),
        (
            Handler::AncientHistory,
            ANCIENT_HISTORY,
            vec![PRECURSORS, OLD_RACES, ALIENS_ON_EARTH],
            ENOUGH_ANCIENT,
        ),
        (
            Handler::DefeatUrquan,
            HOW_DEFEAT,
            vec![HOW_FIND_URQUAN, HOW_FIGHT_URQUAN, HOW_ALLY, HOW_GET_STRONG],
            ENOUGH_DEFEAT,
        ),
    ] {
        run(&mut f, &mut s, h, start);
        assert_eq!(
            choices(&f),
            items.iter().copied().chain([exit]).collect::<Vec<_>>()
        );
        for (i, r) in items.iter().enumerate() {
            run(&mut f, &mut s, h, *r);
            assert_eq!(f.spoken().len(), 1);
            assert_eq!(
                choices(&f),
                items[i + 1..]
                    .iter()
                    .copied()
                    .chain([exit])
                    .collect::<Vec<_>>()
            );
        }
        run(&mut f, &mut s, h, start);
        assert_eq!(f.offers().len(), items.len() + 1);
    }
    run(&mut f, &mut s, Handler::AlienRaces, ALIEN_RACES);
    assert_eq!(choices(&f), vec![265, 266, 267, 268]);
    for r in [ENOUGH_ALLIANCE, ENOUGH_HIERARCHY, WHAT_ABOUT_OTHER] {
        run(&mut f, &mut s, Handler::AlienRaces, r);
    }
    assert_eq!(choices(&f), vec![268]);
    run(&mut f, &mut s, Handler::TellHistory, HISTORY);
    assert_eq!(choices(&f), vec![261, 262, 263, 264]);
    for r in [ENOUGH_ALIENS, ENOUGH_WAR, ENOUGH_ANCIENT] {
        run(&mut f, &mut s, Handler::TellHistory, r);
    }
    assert_eq!(choices(&f), vec![264]);
    run(&mut f, &mut s, Handler::TellMission, OUR_MISSION);
    assert_eq!(choices(&f), vec![253, 254, 295, 255, 243]);
    for r in [
        WHERE_GET_MINERALS,
        WHAT_ABOUT_ALIENS,
        WHAT_DO_NOW,
        WHAT_ABOUT_URQUAN,
    ] {
        run(&mut f, &mut s, Handler::TellMission, r);
    }
    assert_eq!(
        f.offers(),
        vec![(256, Handler::DefeatUrquan), (243, Handler::NeedInfo)]
    );
    run(&mut f, &mut s, Handler::TellMission, ENOUGH_DEFEAT);
    assert_eq!(choices(&f), vec![243]);
}
#[test]
fn timing_is_strict_and_uses_original_non_leap_calendar() {
    for (start_month, start_day, months, days, target) in [
        (
            2,
            17,
            0,
            7,
            Date {
                year: 2155,
                month: 2,
                day: 24,
            },
        ),
        (
            12,
            25,
            0,
            42,
            Date {
                year: 2156,
                month: 2,
                day: 5,
            },
        ),
        (
            11,
            17,
            3,
            0,
            Date {
                year: 2156,
                month: 2,
                day: 17,
            },
        ),
        (
            2,
            28,
            0,
            1,
            Date {
                year: 2155,
                month: 3,
                day: 1,
            },
        ),
    ] {
        let mut f = ready();
        let mut s = Session::default();
        f.put("STARBASE_MONTH", start_month);
        f.put("STARBASE_DAY", start_day);
        f.world.date = target;
        assert!(!Dialogue {
            api: &mut f,
            session: &mut s
        }
        .check_timing(months, days));
        f.world.date.day += 1;
        assert!(Dialogue {
            api: &mut f,
            session: &mut s
        }
        .check_timing(months, days));
    }
}
#[test]
fn every_device_discussion_and_discovery_effect() {
    let cases = [
        (
            "PORTAL_SPAWNER_ON_SHIP",
            "DISCUSSED_PORTAL_SPAWNER",
            ABOUT_PORTAL,
        ),
        (
            "TALKING_PET_ON_SHIP",
            "DISCUSSED_TALKING_PET",
            ABOUT_TALKPET,
        ),
        ("UTWIG_BOMB_ON_SHIP", "DISCUSSED_UTWIG_BOMB", ABOUT_BOMB),
        ("SUN_DEVICE_ON_SHIP", "DISCUSSED_SUN_EFFICIENCY", ABOUT_SUN),
        ("ROSY_SPHERE_ON_SHIP", "DISCUSSED_ROSY_SPHERE", ABOUT_SPHERE),
        ("AQUA_HELIX_ON_SHIP", "DISCUSSED_AQUA_HELIX", ABOUT_HELIX),
        (
            "CLEAR_SPINDLE_ON_SHIP",
            "DISCUSSED_CLEAR_SPINDLE",
            ABOUT_SPINDLE,
        ),
        ("MAIDENS_ON_SHIP", "DISCUSSED_MAIDENS", ABOUT_MAIDENS),
        (
            "UMGAH_BROADCASTERS_ON_SHIP",
            "DISCUSSED_UMGAH_HYPERWAVE",
            ABOUT_UCASTER,
        ),
        (
            "BURV_BROADCASTERS_ON_SHIP",
            "DISCUSSED_BURVIX_HYPERWAVE",
            ABOUT_BCASTER,
        ),
        (
            "TAALO_PROTECTOR_ON_SHIP",
            "DISCUSSED_TAALO_PROTECTOR",
            ABOUT_SHIELD,
        ),
        (
            "SYREEN_SHUTTLE_ON_SHIP",
            "DISCUSSED_SYREEN_SHUTTLE",
            ABOUT_SHUTTLE,
        ),
        (
            "DESTRUCT_CODE_ON_SHIP",
            "DISCUSSED_DESTRUCT_CODE",
            ABOUT_DESTRUCT,
        ),
        ("PORTAL_KEY_ON_SHIP", "DISCUSSED_URQUAN_WARP", ABOUT_WARPPOD),
        (
            "ARTIFACT_2_ON_SHIP",
            "DISCUSSED_ARTIFACT_2",
            ABOUT_ARTIFACT_2,
        ),
        (
            "ARTIFACT_3_ON_SHIP",
            "DISCUSSED_ARTIFACT_3",
            ABOUT_ARTIFACT_3,
        ),
    ];
    for (on, discussed, phrase) in cases {
        let mut f = ready();
        let mut s = Session::default();
        f.put(on, 1);
        assert!(Dialogue {
            api: &mut f,
            session: &mut s
        }
        .discuss_devices(false));
        assert!(f.spoken().is_empty());
        assert_eq!(f.get(discussed), 0);
        f.effects.clear();
        assert!(Dialogue {
            api: &mut f,
            session: &mut s
        }
        .discuss_devices(true));
        assert_eq!(
            f.spoken(),
            vec![
                i32::from(DEVICE_HEAD),
                i32::from(phrase),
                i32::from(DEVICE_TAIL)
            ]
        );
        assert_eq!(f.get(discussed), 1);
        assert!(!Dialogue {
            api: &mut f,
            session: &mut s
        }
        .discuss_devices(false));
    }
    for condition in 1..=4 {
        let mut f = ready();
        let mut s = Session::default();
        f.put("ULTRON_CONDITION", condition);
        assert!(Dialogue {
            api: &mut f,
            session: &mut s
        }
        .discuss_devices(true));
        assert_eq!(
            f.spoken()[1],
            i32::from(ABOUT_ULTRON_0) + i32::from(condition) - 1
        );
    }
}
#[test]
fn eggs_share_discussion_flags_and_vux_report_preserves_track_sequence() {
    let mut f = ready();
    let mut s = Session::default();
    for key in [
        "EGG_CASE0_ON_SHIP",
        "EGG_CASE1_ON_SHIP",
        "EGG_CASE2_ON_SHIP",
        "VUX_BEAST_ON_SHIP",
    ] {
        f.put(key, 1);
    }
    assert!(Dialogue {
        api: &mut f,
        session: &mut s
    }
    .discuss_devices(true));
    assert_eq!(
        f.spoken(),
        vec![
            i32::from(DEVICE_HEAD),
            i32::from(ABOUT_EGGCASE_0),
            i32::from(BETWEEN_DEVICES),
            i32::from(ABOUT_VUXBEAST0),
            i32::from(ABOUT_VUXBEAST1),
            i32::from(DEVICE_TAIL)
        ]
    );
    for key in [
        "DISCUSSED_EGG_CASING0",
        "DISCUSSED_EGG_CASING1",
        "DISCUSSED_EGG_CASING2",
    ] {
        assert_eq!(f.get(key), 1);
    }
    assert_eq!(
        &f.effects[f.effects.len() - 5..],
        &[
            Effect::Talk(4),
            Effect::Talking(false),
            Effect::Talk(5),
            Effect::Talking(true),
            Effect::Talk(u16::MAX)
        ]
    );
}
#[test]
fn bulletins_preserve_bit_positions_order_and_repeat_without_gifts_or_rng() {
    let mut f = ready();
    let mut s = Session::default();
    for race in [6, 19, 7, 9, 3, 1, 5] {
        f.alliances[race] = 1;
    }
    f.put("PORTAL_SPAWNER", 1);
    f.put("ZOQFOT_DISTRESS", 1);
    f.put("SPATHI_SHIELDED_SELVES", 1);
    f.put("PKUNK_MISSION", 1);
    f.put("YEHAT_ABSORBED_PKUNK", 1);
    f.put("CHMMR_BOMB_STATE", 2);
    f.world.random.push_back(0x106);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .check_bulletins(false);
    let bits = [0, 1, 2, 3, 4, 5, 6, 10, 12, 13, 14, 17, 18];
    assert_eq!(s.bulletins, bits.iter().fold(0, |m, b| m | (1 << b)));
    assert_eq!(f.world.random_calls, 1);
    assert!(f.effects.contains(&Effect::AddEscorts(0, 3)));
    let original = f.spoken();
    f.effects.clear();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .check_bulletins(true);
    assert_eq!(f.spoken(), original[1..]);
    assert_eq!(f.world.random_calls, 1);
    assert!(!f
        .effects
        .iter()
        .any(|e| matches!(e, Effect::AddEscorts(..))));
}
#[test]
fn crew_bulletins_visit_later_bits_and_retire_obsolete_thresholds() {
    for sold in [0u16, 1, 100, 101, 250, 251] {
        let mut f = ready();
        let mut s = Session::default();
        f.put("CREW_SOLD_TO_DRUUGE0", sold as u8);
        f.put("CREW_SOLD_TO_DRUUGE1", (sold >> 8) as u8);
        f.put("CREW_PURCHASED0", 232);
        f.put("CREW_PURCHASED1", 3);
        f.world.random.push_back(0);
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .check_bulletins(false);
        let spoken = f.spoken();
        assert_eq!(
            spoken.contains(&i32::from(STARBASE_BULLETIN_27)),
            sold > 0 && sold <= 100
        );
        assert_eq!(
            spoken.contains(&i32::from(STARBASE_BULLETIN_28)),
            sold > 100 && sold <= 250
        );
        assert!(spoken.contains(&i32::from(STARBASE_BULLETIN_29)));
        assert_eq!(
            spoken.contains(&i32::from(STARBASE_BULLETIN_30)),
            sold > 250
        );
    }
}
#[test]
fn timed_bulletins_and_suppression_flags() {
    let mut f = ready();
    let mut s = Session::default();
    f.world.date = Date {
        year: 2156,
        month: 1,
        day: 1,
    };
    f.world.random.push_back(0);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .check_bulletins(false);
    assert_eq!(
        s.bulletins,
        (1 << 7) | (1 << 8) | (1 << 9) | (1 << 11) | (1 << 15)
    );
    let mut f = ready();
    let mut s = Session::default();
    f.world.date = Date {
        year: 2156,
        month: 1,
        day: 1,
    };
    f.put("MET_MELNORME", 1);
    f.put("FOUND_PLUTO_SPATHI", 1);
    f.put("ZOQFOT_GRPOFFS3", 1);
    f.put("DESTRUCT_CODE_ON_SHIP", 1);
    f.put("ARILOU_MANNER", 2);
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .check_bulletins(false);
    assert_eq!(s.bulletins, 0);
    assert_eq!(
        state32(&f, "STARBASE_BULLETS0"),
        (1 << 5) | (1 << 7) | (1 << 8) | (1 << 9) | (1 << 11) | (1 << 15)
    );
}
#[test]
fn analysis_reports_minimum_maximum_fleet_bomb_and_pet_branches() {
    let mut f = ready();
    let mut s = Session::default();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .analyze_condition();
    assert_eq!(
        f.spoken(),
        [
            LETS_SEE,
            IMPROVE_1,
            NEED_THRUSTERS_1,
            NEED_TURN_1,
            NEED_GUNS_1,
            NEED_CREW_1,
            NEED_FUEL_1,
            NEED_STORAGE_1,
            NEED_LANDERS_2,
            NEED_DYNAMOS_1,
            GO_GET_MINERALS
        ]
        .map(i32::from)
    );
    f.world.ship = Ship {
        drives: [1; 11],
        jets: [2; 8],
        modules: [11, 11, 11, 12, 7, 8, 8, 13, 13, 4, 20, 20, 20, 20, 20, 20],
        crew: 150,
        fuel: 15000,
        landers: 3,
        units: 3000,
        mineral_mass: 0,
    };
    f.effects.clear();
    Dialogue {
        api: &mut f,
        session: &mut s,
    }
    .analyze_condition();
    assert_eq!(
        f.spoken(),
        [LETS_SEE, GOT_AWESOME_FLAGSHIP, GO_ALLY_WITH_ALIENS].map(i32::from)
    );
    f.alliances[0] = 1;
    f.alliances[1] = 1;
    for (fleet, bomb, pet, last) in [
        (0, 0, 0, BUY_COMBAT_SHIPS),
        (6000, 0, 0, GO_LEARN_ABOUT_URQUAN),
        (19999, 2, 0, MUST_ACQUIRE_AWESOME_FLEET),
        (20000, 2, 0, MUST_ELIMINATE_URQUAN_GUARDS),
        (20000, 2, 1, GO_DESTROY_SAMATRA),
    ] {
        f.world.fleet = fleet;
        f.put("CHMMR_BOMB_STATE", bomb);
        f.put("TALKING_PET_ON_SHIP", pet);
        f.effects.clear();
        Dialogue {
            api: &mut f,
            session: &mut s,
        }
        .analyze_condition();
        assert_eq!(f.spoken().last(), Some(&i32::from(last)));
    }
}
#[test]
fn mineral_sale_animation_and_skip_transfer_identical_quantities() {
    for skip in [false, true] {
        let mut f = ready();
        let mut s = Session::default();
        f.world.minerals = [2, 0, 1, 0, 0, 0, 0, 0];
        f.world.ship.mineral_mass = 3;
        f.world.skip = skip;
        run(&mut f, &mut s, Handler::SellMinerals, HAVE_MINERALS);
        assert_eq!(f.world.minerals, [0; 8]);
        assert_eq!(f.world.ship.mineral_mass, 0);
        let paid: i32 = f
            .effects
            .iter()
            .map(|e| {
                if let Effect::Gauges(_, _, u) = e {
                    *u
                } else {
                    0
                }
            })
            .sum();
        assert_eq!(paid, 5);
        assert_eq!(f.get("LIGHT_MINERAL_LOAD"), 1);
        assert_eq!(f.spoken(), vec![42, 300, GLOBAL_PLAYER_NAME, 43]);
        assert_eq!(
            &f.effects[..4],
            &[
                Effect::DrawCargo(255, 255),
                Effect::Sleep(420),
                Effect::Now,
                Effect::DrawCargo(0, 0)
            ]
        );
        if !skip {
            assert_eq!(
                &f.effects[4..11],
                &[
                    Effect::Skip,
                    Effect::Remove(0, 1),
                    Effect::Switch,
                    Effect::Now,
                    Effect::DrawCargo(0, 0),
                    Effect::Capacity,
                    Effect::Gauges(0, 0, 1)
                ]
            );
        } else {
            assert!(!f.effects.iter().any(|e| matches!(e, Effect::Capacity)));
        }
    }
}
#[test]
fn mineral_load_thresholds_counter_saturation_and_u16_total_wrap() {
    for (amount, worth, key, first, last) in [
        (999, 1, "LIGHT_MINERAL_LOAD", 42, 54),
        (1000, 1, "MEDIUM_MINERAL_LOAD", 56, 68),
        (2499, 1, "MEDIUM_MINERAL_LOAD", 56, 68),
        (2500, 1, "HEAVY_MINERAL_LOAD", 70, 82),
        (33000, 2, "LIGHT_MINERAL_LOAD", 42, 54),
    ] {
        for visit in 0..=6 {
            let mut f = ready();
            let mut s = Session::default();
            f.world.skip = true;
            f.world.speech = true;
            f.world.minerals[0] = amount;
            f.world.worth[0] = worth;
            f.world.ship.mineral_mass = amount;
            f.put(key, visit);
            run(&mut f, &mut s, Handler::SellMinerals, HAVE_MINERALS);
            let p = if visit == 6 {
                last
            } else {
                first + 2 * i32::from(visit)
            };
            assert_eq!(f.spoken(), vec![p, p + 1]);
            assert_eq!(f.get(key), (visit + 1).min(6));
        }
    }
}
