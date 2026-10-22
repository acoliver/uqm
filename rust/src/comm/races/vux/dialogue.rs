use super::phrases::{hostile_info, visit_phrase};
use super::*;
use crate::comm::races::encounter::EncounterApi;

pub(super) struct Dialogue<'a, A> {
    pub(super) api: &'a mut A,
}

impl<A: EncounterApi<Handler>> Dialogue<'_, A> {
    pub(super) fn handle(&mut self, handler: Handler, response: i32) {
        match handler {
            Handler::Combat => self.combat(response),
            Handler::Menagerie => self.menagerie(response),
            Handler::Normal => self.normal(response),
        }
    }

    fn offer(&mut self, phrase: u32, handler: Handler) {
        self.api.response(phrase as u16, handler, &[]);
    }

    fn counter(&mut self, key: &str, responses: &[u32], final_case: u8) {
        let (case, next) = visit_phrase(self.api.get(key), final_case);
        if let Some(case) = case {
            self.api.npc(responses[case as usize] as i32);
        }
        self.api.set(key, next);
    }

    pub(super) fn combat(&mut self, response: i32) {
        self.api.segue(1);
        match response as u32 {
            OK_TAKE_BEAST => {
                self.api.npc(FOOL_AIEE0 as i32);
                self.api.npc(FOOL_AIEE1 as i32);
                self.api.talk_segue(1);
                self.api.fade_palette(1, 840 / 4);
                self.api.talk_segue(u16::MAX);
                self.api.set("VUX_BEAST_ON_SHIP", 0);
                self.api.set("ZEX_IS_DEAD", 1);
                self.api.segue(0);
            }
            TRY_ANY_WAY => {
                self.api.npc(NOPE as i32);
                self.api.set("VUX_STACK_1", 4);
            }
            KILL_YOU_SQUIDS_1..=KILL_YOU_SQUIDS_4 => {
                self.api.npc(WE_FIGHT as i32);
                let next = self.api.get("VUX_STACK_2").wrapping_add(1);
                if next <= 3 {
                    self.api.set("VUX_STACK_2", next);
                }
            }
            CANT_WE_BE_FRIENDS_1 | CANT_WE_BE_FRIENDS_2 | CANT_WE_BE_FRIENDS_3
            | CANT_WE_BE_FRIENDS_4 => self.counter(
                "VUX_STACK_3",
                &[
                    NEVER_UGLY_HUMANS_1,
                    NEVER_UGLY_HUMANS_2,
                    NEVER_UGLY_HUMANS_3,
                    NEVER_UGLY_HUMANS_4,
                ],
                3,
            ),
            BYE_HOSTILE_SPACE => self.counter(
                "VUX_STACK_4",
                &[
                    GOODBYE_AND_DIE_HOSTILE_SPACE_1,
                    GOODBYE_AND_DIE_HOSTILE_SPACE_2,
                    GOODBYE_AND_DIE_HOSTILE_SPACE_3,
                    GOODBYE_AND_DIE_HOSTILE_SPACE_4,
                ],
                3,
            ),
            BYE_ZEX => {
                self.api.npc(GOODBYE_ZEX as i32);
                self.api.segue(0);
            }
            _ => {
                let visits = self.api.get("VUX_STACK_1");
                let response = match visits {
                    4 => Some(NOT_ACCEPTED_1),
                    5 => Some(NOT_ACCEPTED_2),
                    6 => Some(NOT_ACCEPTED_3),
                    7 => Some(NOT_ACCEPTED_4),
                    8 => Some(NOT_ACCEPTED_5),
                    9 => Some(NOT_ACCEPTED_6),
                    10 => Some(NOT_ACCEPTED_7),
                    11 => Some(NOT_ACCEPTED_8),
                    12 => Some(NOT_ACCEPTED_9),
                    13 => Some(TRUTH),
                    _ => None,
                };
                if let Some(phrase) = response {
                    self.api.npc(phrase as i32);
                }
                self.api.set("VUX_STACK_1", visits.wrapping_add(1));
            }
        }
    }

    pub(super) fn menagerie(&mut self, response: i32) {
        let response = response as u32;
        if response == I_HAVE_BEAST || [WHY_TRUST_1, WHY_TRUST_2, WHY_TRUST_3].contains(&response) {
            match response {
                I_HAVE_BEAST => self.api.npc(GIVE_BEAST as i32),
                WHY_TRUST_1 => {
                    self.api.npc(TRUST_1 as i32);
                    self.api.disable(WHY_TRUST_1 as u16);
                }
                WHY_TRUST_2 => {
                    self.api.npc(TRUST_2 as i32);
                    self.api.disable(WHY_TRUST_2 as u16);
                }
                WHY_TRUST_3 => {
                    self.api.npc(TRUST_3 as i32);
                    self.api.disable(WHY_TRUST_3 as u16);
                }
                _ => unreachable!(),
            }
            if self.api.enabled(WHY_TRUST_1 as u16) {
                self.offer(WHY_TRUST_1, Handler::Menagerie);
            } else if self.api.enabled(WHY_TRUST_2 as u16) {
                self.offer(WHY_TRUST_2, Handler::Menagerie);
            } else if self.api.enabled(WHY_TRUST_3 as u16) {
                self.offer(WHY_TRUST_3, Handler::Menagerie);
            }
            self.offer(OK_TAKE_BEAST, Handler::Combat);
            return;
        }
        if response == KILL_YOU {
            self.api.npc(FIGHT_AGAIN as i32);
            self.api.segue(1);
            return;
        }
        if response == REGARDLESS {
            self.api.npc(THEN_FIGHT as i32);
            self.api.segue(1);
            self.api.set("ZEX_STACK_3", 2);
            self.api.set("ZEX_VISITS", 0);
            return;
        }

        let mut last_stack = 0;
        let mut phrases = [None; 3];
        if response == NULL_PHRASE {
            if self.api.get("ZEX_STACK_3") >= 2 {
                self.counter("ZEX_VISITS", &[FIGHT_OR_TRADE_1, FIGHT_OR_TRADE_2], 1);
            } else {
                self.counter(
                    "ZEX_VISITS",
                    &[ZEX_HELLO_1, ZEX_HELLO_2, ZEX_HELLO_3, ZEX_HELLO_4],
                    3,
                );
            }
        } else if response == WHAT_YOU_DO_HERE {
            self.api.npc(MY_MENAGERIE as i32);
            self.api.set("ZEX_STACK_1", 1);
        } else if response == WHAT_ABOUT_MENAGERIE {
            self.api.npc(NEED_NEW_CREATURE as i32);
            self.api.set("ZEX_STACK_1", 2);
        } else if response == WHAT_ABOUT_CREATURE {
            self.api.npc(ABOUT_CREATURE as i32);
            self.api.set("KNOW_ZEX_WANTS_MONSTER", 1);
            self.api.set("ZEX_STACK_1", 3);
            self.api.disable(WHAT_ABOUT_CREATURE as u16);
        } else if response == ABOUT_CREATURE_AGAIN {
            self.api.npc(CREATURE_AGAIN as i32);
            self.api.disable(ABOUT_CREATURE_AGAIN as u16);
        } else if response == WHY_DONT_YOU_ATTACK {
            self.api.npc(LIKE_YOU as i32);
            last_stack = 1;
            self.api.set("ZEX_STACK_2", 1);
        } else if response == WHY_LIKE_ME {
            self.api.npc(LIKE_BECAUSE as i32);
            last_stack = 1;
            self.api.set("ZEX_STACK_2", 2);
        } else if response == ARE_YOU_A_PERVERT {
            self.api.npc(CALL_ME_WHAT_YOU_WISH as i32);
            self.api.set("ZEX_STACK_2", 3);
        } else if response == TAKE_BY_FORCE {
            self.api.npc(PRECURSOR_DEVICE as i32);
            last_stack = 2;
            self.api.set("ZEX_STACK_3", 1);
        } else if response == YOU_LIED {
            self.api.npc(YUP_LIED as i32);
            last_stack = 2;
            self.api.set("ZEX_STACK_3", 3);
        }

        if self.api.get("KNOW_ZEX_WANTS_MONSTER") != 0 && self.api.get("VUX_BEAST_ON_SHIP") != 0 {
            phrases[0] = Some(I_HAVE_BEAST);
        } else {
            phrases[0] = match self.api.get("ZEX_STACK_1") {
                0 => Some(WHAT_YOU_DO_HERE),
                1 => Some(WHAT_ABOUT_MENAGERIE),
                2 => Some(WHAT_ABOUT_CREATURE),
                3 if self.api.enabled(ABOUT_CREATURE_AGAIN as u16) => Some(ABOUT_CREATURE_AGAIN),
                _ => None,
            };
        }
        phrases[1] = match self.api.get("ZEX_STACK_2") {
            0 => Some(WHY_DONT_YOU_ATTACK),
            1 => Some(WHY_LIKE_ME),
            2 => Some(ARE_YOU_A_PERVERT),
            _ => None,
        };
        phrases[2] = match self.api.get("ZEX_STACK_3") {
            0 => Some(TAKE_BY_FORCE),
            1 => Some(REGARDLESS),
            2 => Some(YOU_LIED),
            3 => Some(KILL_YOU),
            _ => None,
        };
        if let Some(phrase) = phrases[last_stack] {
            self.offer(phrase, Handler::Menagerie);
        }
        for (index, phrase) in phrases.into_iter().enumerate() {
            if index != last_stack {
                if let Some(phrase) = phrase {
                    self.offer(phrase, Handler::Menagerie);
                }
            }
        }
        self.offer(BYE_ZEX, Handler::Combat);
    }

    pub(super) fn normal(&mut self, response: i32) {
        match response as u32 {
            WHY_SO_MEAN => {
                self.api.npc(URQUAN_SLAVES as i32);
                self.api.set("VUX_STACK_1", 1);
            }
            DEEPER_REASON => {
                self.api.npc(OLD_INSULT as i32);
                self.api.set("VUX_STACK_1", 2);
            }
            IF_WE_APOLOGIZE => {
                self.api.npc(PROBABLY_NOT as i32);
                self.api.set("VUX_STACK_1", 3);
            }
            WHATS_UP_HOSTILE => {
                let (phrase, next) = hostile_info(self.api.get("VUX_INFO"));
                if let Some(phrase) = phrase {
                    self.api.npc(phrase as i32);
                }
                self.api.set("VUX_INFO", next);
                self.api.disable(WHATS_UP_HOSTILE as u16);
            }
            _ => {}
        }

        let topic = match self.api.get("VUX_STACK_1") {
            0 => Some(WHY_SO_MEAN),
            1 => Some(DEEPER_REASON),
            2 => Some(IF_WE_APOLOGIZE),
            3 => Some(TRY_ANY_WAY),
            stack @ 4..=13 => Some(
                [
                    APOLOGY_1, APOLOGY_2, APOLOGY_3, APOLOGY_4, APOLOGY_5, APOLOGY_6, APOLOGY_7,
                    APOLOGY_8, APOLOGY_9, APOLOGY_10,
                ][usize::from(stack - 4)],
            ),
            _ => None,
        };
        if let Some(topic) = topic {
            self.offer(
                topic,
                if topic == TRY_ANY_WAY || (APOLOGY_1..=APOLOGY_10).contains(&topic) {
                    Handler::Combat
                } else {
                    Handler::Normal
                },
            );
        }
        let threats = [
            KILL_YOU_SQUIDS_1,
            KILL_YOU_SQUIDS_2,
            KILL_YOU_SQUIDS_3,
            KILL_YOU_SQUIDS_4,
        ];
        if let Some(phrase) = threats.get(self.api.get("VUX_STACK_2") as usize) {
            self.offer(*phrase, Handler::Combat);
        }
        if self.api.enabled(WHATS_UP_HOSTILE as u16) {
            self.offer(WHATS_UP_HOSTILE, Handler::Normal);
        }
        if self.api.get("VUX_STACK_1") > 13 {
            let friends = [
                CANT_WE_BE_FRIENDS_1,
                CANT_WE_BE_FRIENDS_2,
                CANT_WE_BE_FRIENDS_3,
                CANT_WE_BE_FRIENDS_4,
            ];
            if let Some(phrase) = friends.get(self.api.get("VUX_STACK_3") as usize) {
                self.offer(*phrase, Handler::Combat);
            }
        }
        self.offer(BYE_HOSTILE_SPACE, Handler::Combat);
    }

    pub(super) fn initialize(&mut self) {
        let peaceful = (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 6)) != 0
            || self.api.activity() as u8 == crate::comm::dispatch::WON_LAST_BATTLE as u8;
        self.api.segue(if peaceful { 0 } else { 1 });
    }

    pub(super) fn intro(&mut self) {
        if self.api.activity() as u8 == crate::comm::dispatch::WON_LAST_BATTLE as u8 {
            self.api.npc(OUT_TAKES as i32);
            self.api.segue(0);
            return;
        }
        if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 6)) != 0 {
            self.menagerie(NULL_PHRASE as i32);
            return;
        }
        if (self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7)) != 0 {
            self.counter(
                "VUX_HOME_VISITS",
                &[
                    HOMEWORLD_HELLO_1,
                    HOMEWORLD_HELLO_2,
                    HOMEWORLD_HELLO_3,
                    HOMEWORLD_HELLO_4,
                ],
                3,
            );
        } else {
            self.counter(
                "VUX_VISITS",
                &[SPACE_HELLO_1, SPACE_HELLO_2, SPACE_HELLO_3, SPACE_HELLO_4],
                3,
            );
        }
        self.normal(NULL_PHRASE as i32);
    }
}
