use super::api::UrquanApi;
use super::phrases::{general_info, visit_phrase};
use super::*;

pub(super) struct Dialogue<'a, A> {
    pub(super) api: &'a mut A,
}

impl<A: UrquanApi> Dialogue<'_, A> {
    pub(super) fn handle(&mut self, handler: Handler, response: i32) {
        match handler {
            Handler::Combat => self.combat(response),
            Handler::Normal => self.normal(response),
            Handler::Hypno => self.hypno(response),
            Handler::MentionCompulsion => self.mention_compulsion(response),
            Handler::DescribePet => self.describe_pet(response),
            Handler::DescribeCompulsion => self.describe_compulsion(response),
            Handler::Loser => self.loser(response),
        }
    }

    fn combat(&mut self, response: i32) {
        self.api.segue(1);
        match response as u32 {
            YOU_MUST_SURRENDER => self.api.npc(NOPE as i32),
            WHATS_UP_1 | WHATS_UP_2 | WHATS_UP_3 | WHATS_UP_4 => {
                let (phrase, next) = general_info(self.api.get("URQUAN_INFO"));
                if let Some(phrase) = phrase {
                    self.api.npc(phrase as i32);
                }
                self.api.set("URQUAN_INFO", next);
            }
            I_WONT_SURRENDER => self.api.npc(BAD_CHOICE as i32),
            I_WILL_SURRENDER => {
                self.api.npc(GOOD_CHOICE as i32);
                self.api.segue(2);
            }
            LIKE_TO_LEAVE => self.api.npc(INDEPENDENCE_IS_BAD as i32),
            BYE_WARS_OVER => {
                self.api.npc(GOODBYE_WARS_OVER as i32);
                self.api.segue(0);
            }
            BYE_SUB_HYPNO => self.api.npc(GOODBYE_AND_DIE_SUB_HYPNO as i32),
            BYE_INIT_HYPNO => {
                self.api.npc(GOODBYE_AND_DIE_INIT_HYPNO as i32);
                self.api.set("URQUAN_HYPNO_VISITS", 1);
            }
            TERRAN_AMPHIBIAN | TALKING_PET_ON_STEROIDS => {
                self.api.npc(BAD_NEWS as i32);
                self.api.segue(0);
                self.api.set("URQUAN_HYPNO_VISITS", 1);
            }
            FALLING_ASLEEP | SOMEONE_ELSE_CONTROLLED => {
                self.api.npc(SOUNDS_FAMILIAR as i32);
                self.api.segue(0);
                self.api.set("URQUAN_HYPNO_VISITS", 1);
            }
            _ => {}
        }
    }

    fn normal(&mut self, response: i32) {
        if response == I_SURRENDER as i32 {
            self.api.npc(DISOBEDIENCE_PUNISHED as i32);
            self.api
                .response(I_WONT_SURRENDER as u16, Handler::Combat, &[]);
            self.api
                .response(I_WILL_SURRENDER as u16, Handler::Combat, &[]);
            return;
        }
        if response == KEY_PHRASE as i32 {
            self.api.npc(URQUAN_STORY as i32);
            self.api.set("KNOW_URQUAN_STORY", 2);
        }
        self.api
            .response(YOU_MUST_SURRENDER as u16, Handler::Combat, &[]);
        if self.api.get("KNOW_URQUAN_STORY") == 1 {
            self.api.response(KEY_PHRASE as u16, Handler::Normal, &[]);
        }
        let info = self.api.get("URQUAN_INFO");
        if info < 4 {
            self.api.response(
                (WHATS_UP_1 + u32::from(info) * 2) as u16,
                Handler::Combat,
                &[],
            );
        }
        self.api.response(I_SURRENDER as u16, Handler::Normal, &[]);
        self.api
            .response(LIKE_TO_LEAVE as u16, Handler::Combat, &[]);
    }

    fn mention_compulsion(&mut self, response: i32) {
        if response == COMPULSION as i32 {
            self.api.npc(WHAT_COMPULSION as i32);
            self.api.set("MENTIONED_PET_COMPULSION", 1);
        } else if response == POSSESSED_BY_DEVIL as i32 {
            self.api.npc(STUPID_DEVIL as i32);
            self.api.disable(POSSESSED_BY_DEVIL as u16);
        } else if response == WHY_EXPLAIN as i32 {
            self.api.npc(MUST_EXPLAIN as i32);
            self.api.disable(WHY_EXPLAIN as u16);
        }
        self.api
            .response(WASCALLY_LITTLE_GUY as u16, Handler::DescribePet, &[]);
        self.api
            .response(ALIEN_MIND_CONTROL as u16, Handler::DescribeCompulsion, &[]);
        for phrase in [POSSESSED_BY_DEVIL, WHY_EXPLAIN] {
            if self.api.enabled(phrase as u16) {
                self.api
                    .response(phrase as u16, Handler::MentionCompulsion, &[]);
            }
        }
    }

    fn describe_pet(&mut self, response: i32) {
        if response == WASCALLY_LITTLE_GUY as i32 {
            self.api.npc(WHAT_IT_LOOK_LIKE as i32);
        } else if response == TURD_AND_TOAD as i32 {
            self.api.npc(WHAT_IS_TURD_AND_TOAD as i32);
            self.api.disable(TURD_AND_TOAD as u16);
        }
        self.api
            .response(TERRAN_AMPHIBIAN as u16, Handler::Combat, &[]);
        self.api
            .response(TALKING_PET_ON_STEROIDS as u16, Handler::Combat, &[]);
        if self.api.enabled(TURD_AND_TOAD as u16) {
            self.api
                .response(TURD_AND_TOAD as u16, Handler::DescribePet, &[]);
        }
    }

    fn describe_compulsion(&mut self, response: i32) {
        if response == ALIEN_MIND_CONTROL as i32 {
            self.api.npc(WHAT_FELT_LIKE as i32);
        } else if response == BEFORE_COFFEE as i32 {
            self.api.npc(EXPLAIN as i32);
            self.api.disable(BEFORE_COFFEE as u16);
        }
        self.api
            .response(FALLING_ASLEEP as u16, Handler::Combat, &[]);
        self.api
            .response(SOMEONE_ELSE_CONTROLLED as u16, Handler::Combat, &[]);
        if self.api.enabled(BEFORE_COFFEE as u16) {
            self.api
                .response(BEFORE_COFFEE as u16, Handler::DescribeCompulsion, &[]);
        }
    }

    fn hypno(&mut self, response: i32) {
        let subsequent = self.api.get("URQUAN_HYPNO_VISITS") != 0;
        if !subsequent {
            if response == 0 {
                self.api.npc(INIT_URQUAN_WAKE_UP as i32);
            } else if response == WHERE_AM_I as i32 {
                self.api.npc(YOU_ARE_HERE as i32);
                self.api.disable(WHERE_AM_I as u16);
            } else if response == WHY_DOES_MY_HEAD_HURT as i32 {
                self.api.npc(HURTS_BECAUSE as i32);
                self.api.disable(WHY_DOES_MY_HEAD_HURT as u16);
            } else if response == WHAT_ABOUT_2_WEEKS as i32 {
                self.api.npc(ABOUT_2_WEEKS as i32);
                self.api.disable(WHAT_ABOUT_2_WEEKS as u16);
            }
            for phrase in [WHERE_AM_I, WHY_DOES_MY_HEAD_HURT, WHAT_ABOUT_2_WEEKS] {
                if self.api.enabled(phrase as u16) {
                    self.api.response(phrase as u16, Handler::Hypno, &[]);
                }
            }
            self.api
                .response(COMPULSION as u16, Handler::MentionCompulsion, &[]);
            self.api
                .response(BYE_INIT_HYPNO as u16, Handler::Combat, &[]);
        } else {
            if response == 0 {
                self.api.npc(SUBSEQUENT_URQUAN_WAKE_UP as i32);
            } else if response == UH_OH as i32 {
                self.api.npc(NO_UH_OH as i32);
                self.api.disable(UH_OH as u16);
            } else if response == STOP_MEETING as i32 {
                self.api.npc(NO_STOP_MEETING as i32);
                self.api.disable(STOP_MEETING as u16);
            }
            for phrase in [UH_OH, STOP_MEETING] {
                if self.api.enabled(phrase as u16) {
                    self.api.response(phrase as u16, Handler::Hypno, &[]);
                }
            }
            if self.api.get("MENTIONED_PET_COMPULSION") == 0 {
                self.api
                    .response(COMPULSION as u16, Handler::MentionCompulsion, &[]);
            }
            self.api
                .response(BYE_SUB_HYPNO as u16, Handler::Combat, &[]);
        }
    }

    fn loser(&mut self, response: i32) {
        if response == WHY_FLEE as i32 {
            self.api.npc(FLEE_BECAUSE as i32);
            self.api.disable(WHY_FLEE as u16);
        } else if response == WHAT_HAPPENS_NOW as i32 {
            self.api.npc(HAPPENS_NOW as i32);
            self.api.disable(WHAT_HAPPENS_NOW as u16);
        } else if response == WHAT_ABOUT_YOU as i32 {
            self.api.npc(ABOUT_US as i32);
            self.api.disable(WHAT_ABOUT_YOU as u16);
        }
        for phrase in [WHY_FLEE, WHAT_HAPPENS_NOW, WHAT_ABOUT_YOU] {
            if self.api.enabled(phrase as u16) {
                self.api.response(phrase as u16, Handler::Loser, &[]);
            }
        }
        self.api
            .response(BYE_WARS_OVER as u16, Handler::Combat, &[]);
    }

    pub(super) fn initialize(&mut self) {
        let activity = self.api.activity() as u8;
        let peaceful = self.api.get("PLAYER_HYPNOTIZED") != 0
            || activity == crate::comm::dispatch::WON_LAST_BATTLE as u8
            || (activity == crate::comm::dispatch::IN_INTERPLANETARY as u8
                && self.api.group_battle("URQUAN_PROBE_GRPOFFS0"));
        self.api.segue(if peaceful { 0 } else { 1 });
    }

    pub(super) fn post(&mut self) {
        self.api.set("PLAYER_HYPNOTIZED", 0);
    }

    pub(super) fn intro(&mut self) {
        let activity = self.api.activity() as u8;
        if activity == crate::comm::dispatch::WON_LAST_BATTLE as u8 {
            self.api.npc(OUT_TAKES as i32);
            self.api.segue(0);
            return;
        }
        if activity == crate::comm::dispatch::IN_INTERPLANETARY as u8
            && self.api.group_battle("URQUAN_PROBE_GRPOFFS0")
        {
            self.api.npc(SEND_MESSAGE as i32);
            self.api.set("PROBE_MESSAGE_DELIVERED", 1);
            return;
        }
        if self.api.get("PLAYER_HYPNOTIZED") != 0 {
            self.api.intro_mode(3, 840 * 5);
            self.hypno(0);
            return;
        }
        if self.api.get("URQUAN_SENSES_EVIL") == 0 && self.api.get("TALKING_PET_ON_SHIP") != 0 {
            self.api.npc(SENSE_EVIL as i32);
            self.api.set("URQUAN_SENSES_EVIL", 1);
        }
        if activity == crate::comm::dispatch::IN_INTERPLANETARY as u8
            && self.api.group_battle("COLONY_GRPOFFS0")
        {
            self.api.npc(CAUGHT_YA as i32);
            self.api.segue(1);
            return;
        }
        if activity == crate::comm::dispatch::IN_INTERPLANETARY as u8
            && self.api.group_battle("SAMATRA_GRPOFFS0")
        {
            self.api.npc(HELLO_SAMATRA as i32);
            self.api.set("AWARE_OF_SAMATRA", 1);
            self.api.segue(1);
            return;
        }
        let visits = self.api.get("URQUAN_VISITS");
        if self.api.get("KOHR_AH_FRENZY") == 0 {
            let (phrase, next) = visit_phrase(visits, 4);
            if let Some(index) = phrase {
                self.api.npc(
                    [
                        INIT_HELLO,
                        SUBSEQUENT_HELLO_1,
                        SUBSEQUENT_HELLO_2,
                        SUBSEQUENT_HELLO_3,
                        SUBSEQUENT_HELLO_4,
                    ][usize::from(index)] as i32,
                );
            }
            self.normal(0);
            self.api.set("URQUAN_VISITS", next);
        } else {
            let (_, next) = visit_phrase(visits, 1);
            match visits {
                0 => {
                    self.api.npc(INIT_FLEE_HUMAN as i32);
                    self.loser(0);
                }
                1 => {
                    self.api.npc(SUBSEQUENT_FLEE_HUMAN as i32);
                    self.api.segue(0);
                }
                _ => {}
            }
            self.api.set("URQUAN_VISITS", next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::api::World;
    use super::*;
    use crate::comm::races::encounter::testing::{Effect, Fake};
    use crate::comm::races::script::ScriptApi;

    type Api = Fake<Handler, World>;

    fn offered(api: &Api) -> Vec<u16> {
        api.offers().into_iter().map(|(phrase, _)| phrase).collect()
    }

    #[test]
    fn normal_greeting_offers_surrender_story_and_visit_specific_information() {
        let mut api = Api::default();
        api.put("KNOW_URQUAN_STORY", 1);
        Dialogue { api: &mut api }.normal(0);
        assert_eq!(
            offered(&api),
            [
                YOU_MUST_SURRENDER as u16,
                KEY_PHRASE as u16,
                WHATS_UP_1 as u16,
                I_SURRENDER as u16,
                LIKE_TO_LEAVE as u16,
            ]
        );
    }

    #[test]
    fn general_information_advances_and_repeats_its_final_line() {
        let mut api = Api::default();
        for (expected, count) in [
            (GENERAL_INFO_1, 1),
            (GENERAL_INFO_2, 2),
            (GENERAL_INFO_3, 3),
            (GENERAL_INFO_4, 3),
        ] {
            Dialogue { api: &mut api }.combat(WHATS_UP_1 as i32);
            assert_eq!(api.spoken().last(), Some(&(expected as i32)));
            assert_eq!(api.get("URQUAN_INFO"), count);
        }
    }

    #[test]
    fn surrender_choices_set_the_matching_ending() {
        let mut api = Api::default();
        Dialogue { api: &mut api }.normal(I_SURRENDER as i32);
        assert_eq!(api.spoken(), [DISOBEDIENCE_PUNISHED as i32]);
        assert_eq!(
            offered(&api),
            [I_WONT_SURRENDER as u16, I_WILL_SURRENDER as u16]
        );
        Dialogue { api: &mut api }.combat(I_WILL_SURRENDER as i32);
        assert_eq!(api.spoken().last(), Some(&(GOOD_CHOICE as i32)));
        assert_eq!(api.segue, 2);
        assert!(api.effects.contains(&Effect::Segue(2)));
    }

    #[test]
    fn intro_sends_probe_message_before_other_greetings() {
        let mut api = Api {
            activity: crate::comm::dispatch::IN_INTERPLANETARY,
            ..Api::default()
        };
        api.world
            .group_battles
            .insert("URQUAN_PROBE_GRPOFFS0".into());
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.spoken(), [SEND_MESSAGE as i32]);
        assert_eq!(api.get("PROBE_MESSAGE_DELIVERED"), 1);
        assert!(offered(&api).is_empty());
    }

    #[test]
    fn first_normal_intro_senses_pet_and_updates_visits() {
        let mut api = Api::default();
        api.put("TALKING_PET_ON_SHIP", 1);
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.spoken(), [SENSE_EVIL as i32, INIT_HELLO as i32]);
        assert_eq!(api.get("URQUAN_SENSES_EVIL"), 1);
        assert_eq!(api.get("URQUAN_VISITS"), 1);
        assert!(offered(&api).contains(&(YOU_MUST_SURRENDER as u16)));
    }

    #[test]
    fn hypnotized_intro_fades_in_then_offers_initial_questions() {
        let mut api = Api::default();
        api.put("PLAYER_HYPNOTIZED", 1);
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.world.intro_modes, [(3, 840 * 5)]);
        assert_eq!(api.spoken(), [INIT_URQUAN_WAKE_UP as i32]);
        assert_eq!(
            offered(&api),
            [
                WHERE_AM_I as u16,
                WHY_DOES_MY_HEAD_HURT as u16,
                WHAT_ABOUT_2_WEEKS as u16,
                COMPULSION as u16,
                BYE_INIT_HYPNO as u16,
            ]
        );
    }

    #[test]
    fn compulsion_discussion_records_discovery_and_offers_followups() {
        let mut api = Api::default();
        Dialogue { api: &mut api }.mention_compulsion(COMPULSION as i32);
        assert_eq!(api.spoken(), [WHAT_COMPULSION as i32]);
        assert_eq!(api.get("MENTIONED_PET_COMPULSION"), 1);
        assert_eq!(
            offered(&api),
            [
                WASCALLY_LITTLE_GUY as u16,
                ALIEN_MIND_CONTROL as u16,
                POSSESSED_BY_DEVIL as u16,
                WHY_EXPLAIN as u16,
            ]
        );
    }

    #[test]
    fn won_battle_plays_outtakes_and_ends_without_offers() {
        let mut api = Api {
            activity: crate::comm::dispatch::WON_LAST_BATTLE,
            ..Api::default()
        };
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.spoken(), [OUT_TAKES as i32]);
        assert_eq!(api.segue, 0);
        assert!(offered(&api).is_empty());
    }

    #[test]
    fn colony_group_triggers_capture_dialogue_before_samatra_check() {
        let mut api = Api {
            activity: crate::comm::dispatch::IN_INTERPLANETARY,
            ..Api::default()
        };
        api.world.group_battles.insert("COLONY_GRPOFFS0".into());
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.spoken(), [CAUGHT_YA as i32]);
        assert_eq!(api.segue, 1);
        assert_eq!(api.get("AWARE_OF_SAMATRA"), 0);
    }

    #[test]
    fn samatra_group_marks_knowledge_and_starts_hostile_combat() {
        let mut api = Api {
            activity: crate::comm::dispatch::IN_INTERPLANETARY,
            ..Api::default()
        };
        api.world.group_battles.insert("SAMATRA_GRPOFFS0".into());
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.spoken(), [HELLO_SAMATRA as i32]);
        assert_eq!(api.get("AWARE_OF_SAMATRA"), 1);
        assert_eq!(api.segue, 1);
    }

    #[test]
    fn first_frenzy_visit_offers_enabled_loser_questions_and_advances_byte_counter() {
        let mut api = Api::default();
        api.put("KOHR_AH_FRENZY", 1);
        Dialogue { api: &mut api }.intro();
        assert_eq!(api.spoken(), [INIT_FLEE_HUMAN as i32]);
        assert_eq!(api.get("URQUAN_VISITS"), 1);
        assert_eq!(
            offered(&api),
            [
                WHY_FLEE as u16,
                WHAT_HAPPENS_NOW as u16,
                WHAT_ABOUT_YOU as u16,
                BYE_WARS_OVER as u16,
            ]
        );
    }

    #[test]
    fn initialization_sets_peace_for_probe_battle_and_hostility_otherwise() {
        let mut api = Api {
            activity: crate::comm::dispatch::IN_INTERPLANETARY,
            ..Api::default()
        };
        api.world
            .group_battles
            .insert("URQUAN_PROBE_GRPOFFS0".into());
        Dialogue { api: &mut api }.initialize();
        assert_eq!(api.segue, 0);

        let mut api = Api::default();
        Dialogue { api: &mut api }.initialize();
        assert_eq!(api.segue, 1);
    }

    #[test]
    fn post_encounter_clears_hypnotized_state() {
        let mut api = Api::default();
        api.put("PLAYER_HYPNOTIZED", 1);
        Dialogue { api: &mut api }.post();
        assert_eq!(api.get("PLAYER_HYPNOTIZED"), 0);
    }
    #[test]
    fn later_frenzy_visits_increment_without_repeating_the_second_greeting() {
        let mut api = Api::default();
        api.put("KOHR_AH_FRENZY", 1);
        api.put("URQUAN_VISITS", 2);
        Dialogue { api: &mut api }.intro();
        assert!(api.spoken().is_empty());
        assert!(offered(&api).is_empty());
        assert_eq!(api.get("URQUAN_VISITS"), 3);
    }
}
