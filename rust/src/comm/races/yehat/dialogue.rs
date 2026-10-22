use super::api::YehatApi;
use super::phrases::{space_info, visit_phrase};
use super::*;

pub(super) struct Dialogue<'a, A> {
    pub(super) api: &'a mut A,
}

impl<A: YehatApi> Dialogue<'_, A> {
    pub(super) fn handle(&mut self, handler: Handler, response: i32) {
        match handler {
            Handler::Exit => self.exit(response),
            Handler::Royalists => self.royalists(response),
            Handler::StartRevolt => self.start_revolt(response),
            Handler::Home => self.home(response),
            Handler::Space => self.space(response),
        }
    }

    fn offer(&mut self, phrase: u32, handler: Handler) {
        self.api.response(phrase as u16, handler, &[]);
    }

    fn counter(&mut self, key: &str, phrases: &[u32], final_case: u8) {
        let visits = self.api.get(key);
        let (case, next) = visit_phrase(visits, final_case);
        if let Some(case) = case {
            self.api.npc(phrases[case as usize] as i32);
        }
        self.api.set(key, next);
    }

    pub(super) fn exit(&mut self, response: i32) {
        self.api.segue(1);
        match response as u32 {
            BYE_HOMEWORLD => self.api.npc(GOODBYE_AND_DIE_HOMEWORLD as i32),
            BYE_ROYALIST => self.api.npc(GOODBYE_AND_DIE_ROYALIST as i32),
            I_DEMAND_YOU_ALLY_HOMEWORLD0 => {
                self.api.npc(ENEMY_MUST_DIE as i32);
                self.api.set("NO_YEHAT_ALLY_HOME", 1);
            }
            BYE_SPACE => {
                if self.api.random_bit() != 0 {
                    self.api.npc(GOODBYE_AND_DIE_SPACE as i32);
                } else {
                    self.api.npc(GO_IN_PEACE as i32);
                    self.api.segue(0);
                }
            }
            NOT_HERE | NOT_SEND => {
                let visits = self.api.get("YEHAT_REBEL_VISITS");
                if visits == 0 {
                    self.api.npc(JUST_A_TRICK_1 as i32);
                } else if visits == 1 {
                    self.api.npc(JUST_A_TRICK_2 as i32);
                }
                self.api.set("YEHAT_REBEL_VISITS", 1);
            }
            OK_SEND => {
                self.api.npc(WE_REVOLT as i32);
                self.api.segue(0);
                for key in [
                    "YEHAT_CIVIL_WAR",
                    "YEHAT_VISITS",
                    "YEHAT_HOME_VISITS",
                    "YEHAT_REBEL_VISITS",
                    "YEHAT_REBEL_INFO",
                    "YEHAT_REBEL_TOLD_PKUNK",
                    "NO_YEHAT_INFO",
                ] {
                    self.api.set(key, u8::from(key == "YEHAT_CIVIL_WAR"));
                }
                self.api.schedule_rebel_event();
            }
            _ => {}
        }
    }

    pub(super) fn royalists(&mut self, response: i32) {
        match response as u32 {
            HOW_IS_REBELLION => {
                self.counter(
                    "YEHAT_ROYALIST_INFO",
                    &[ROYALIST_REBELLION_1, ROYALIST_REBELLION_2],
                    1,
                );
                self.api.disable(HOW_IS_REBELLION as u16);
            }
            WHAT_ABOUT_PKUNK_ROYALIST => {
                self.pkunk_response();
                self.api.set("YEHAT_ROYALIST_TOLD_PKUNK", 1);
            }
            SORRY_ABOUT_REVOLUTION => {
                self.api.npc(ALL_YOUR_FAULT as i32);
                self.api.set("NO_YEHAT_INFO", 1);
            }
            _ => {}
        }
        if self.api.enabled(HOW_IS_REBELLION as u16) {
            self.offer(HOW_IS_REBELLION, Handler::Royalists);
        }
        self.offer_pkunk(Handler::Royalists);
        if self.api.get("NO_YEHAT_INFO") == 0 {
            self.offer(SORRY_ABOUT_REVOLUTION, Handler::Royalists);
        }
        self.offer(BYE_ROYALIST, Handler::Exit);
    }

    pub(super) fn start_revolt(&mut self, response: i32) {
        match response as u32 {
            SHOFIXTI_ALIVE_1 => {
                self.api.npc(SEND_HIM_OVER_1 as i32);
                self.api.set("YEHAT_REBEL_TOLD_PKUNK", 1);
            }
            SHOFIXTI_ALIVE_2 => self.api.npc(SEND_HIM_OVER_2 as i32),
            _ => {}
        }
        if self.api.have_shofixti_escort() {
            self.offer(OK_SEND, Handler::Exit);
        } else {
            self.offer(NOT_HERE, Handler::Exit);
        }
        self.offer(NOT_SEND, Handler::Exit);
    }

    pub(super) fn home(&mut self, response: i32) {
        match response as u32 {
            WHATS_UP_HOMEWORLD => {
                self.counter(
                    "YEHAT_ROYALIST_INFO",
                    &[GENERAL_INFO_HOMEWORLD_1, GENERAL_INFO_HOMEWORLD_2],
                    1,
                );
                self.api.disable(WHATS_UP_HOMEWORLD as u16);
            }
            AT_LEAST_HELP_US_HOMEWORLD => {
                self.api.npc(NO_HELP_ENEMY as i32);
                self.api.set("NO_YEHAT_HELP_HOME", 1);
            }
            GIVE_INFO => {
                self.api.npc(NO_INFO_FOR_ENEMY as i32);
                self.api.set("NO_YEHAT_INFO", 1);
            }
            WHAT_ABOUT_PKUNK_ROYALIST => {
                self.pkunk_response();
                self.api.set("YEHAT_ROYALIST_TOLD_PKUNK", 1);
            }
            _ => {}
        }
        if self.api.enabled(WHATS_UP_HOMEWORLD as u16) {
            self.offer(WHATS_UP_HOMEWORLD, Handler::Home);
        }
        self.offer_pkunk(Handler::Home);
        if self.api.get("NO_YEHAT_HELP_HOME") == 0 {
            self.offer(AT_LEAST_HELP_US_HOMEWORLD, Handler::Home);
        }
        if self.api.get("NO_YEHAT_INFO") == 0 {
            self.offer(GIVE_INFO, Handler::Home);
        }
        if self.api.get("NO_YEHAT_ALLY_HOME") == 0 {
            let mut fragments = vec![
                I_DEMAND_YOU_ALLY_HOMEWORLD0 as i32,
                crate::comm::races::script::GLOBAL_PLAYER_NAME,
                I_DEMAND_YOU_ALLY_HOMEWORLD1 as i32,
            ];
            fragments.extend(self.api.alliance_fragments(NAME_1 as u16));
            fragments.extend([
                I_DEMAND_YOU_ALLY_HOMEWORLD2 as i32,
                crate::comm::races::script::GLOBAL_SHIP_NAME,
                I_DEMAND_YOU_ALLY_HOMEWORLD3 as i32,
            ]);
            self.api.response(
                I_DEMAND_YOU_ALLY_HOMEWORLD0 as u16,
                Handler::Exit,
                &fragments,
            );
        }
        self.offer(BYE_HOMEWORLD, Handler::Exit);
    }

    pub(super) fn space(&mut self, response: i32) {
        let mut last_stack = 0;
        let mut options = [None; 3];
        match response as u32 {
            WHATS_UP_SPACE_1 | WHATS_UP_SPACE_2 | WHATS_UP_SPACE_3 | WHATS_UP_SPACE_4 => {
                let (phrase, next) = space_info(self.api.get("YEHAT_REBEL_INFO"));
                if let Some(phrase) = phrase {
                    self.api.npc(phrase as i32);
                }
                self.api.set("YEHAT_REBEL_INFO", next);
                self.api.disable(WHATS_UP_SPACE_1 as u16);
            }
            I_DEMAND_YOU_ALLY_SPACE0 => {
                self.api.npc(WE_CANNOT_1 as i32);
                last_stack = 2;
                self.api.set("NO_YEHAT_ALLY_SPACE", 1);
            }
            OBLIGATION => {
                self.api.npc(WE_CANNOT_2 as i32);
                self.api.segue(0);
                self.api.set("NO_YEHAT_ALLY_SPACE", 2);
                return;
            }
            AT_LEAST_HELP_US_SPACE => {
                self.api.npc(SORRY_CANNOT as i32);
                last_stack = 1;
                self.api.set("NO_YEHAT_HELP_SPACE", 1);
            }
            DISHONOR => {
                self.api.npc(HERES_A_HINT as i32);
                self.api.set("NO_YEHAT_HELP_SPACE", 2);
            }
            WHAT_ABOUT_PKUNK_ROYALIST => {
                self.pkunk_response();
                self.api.set("YEHAT_ROYALIST_TOLD_PKUNK", 1);
            }
            _ => {}
        }

        if self.api.enabled(WHATS_UP_SPACE_1 as u16) {
            options[0] = match self.api.get("YEHAT_REBEL_INFO") {
                0 => Some(WHATS_UP_SPACE_1),
                1 => Some(WHATS_UP_SPACE_2),
                2 => Some(WHATS_UP_SPACE_3),
                3 => Some(WHATS_UP_SPACE_4),
                _ => None,
            };
        }
        options[1] = match self.api.get("NO_YEHAT_HELP_SPACE") {
            0 => Some(AT_LEAST_HELP_US_SPACE),
            1 => Some(DISHONOR),
            _ => None,
        };
        options[2] = match self.api.get("NO_YEHAT_ALLY_SPACE") {
            0 => Some(I_DEMAND_YOU_ALLY_SPACE0),
            1 => Some(OBLIGATION),
            _ => None,
        };
        self.offer_space_option(options[last_stack]);
        for (index, phrase) in options.into_iter().enumerate() {
            if index != last_stack {
                self.offer_space_option(phrase);
            }
        }
        self.offer_pkunk(Handler::Space);
        if self.api.get("SHOFIXTI_VISITS") != 0 {
            match self.api.get("YEHAT_REBEL_TOLD_PKUNK") {
                0 => self.offer(SHOFIXTI_ALIVE_1, Handler::StartRevolt),
                1 => self.offer(SHOFIXTI_ALIVE_2, Handler::StartRevolt),
                _ => {}
            }
        }
        self.offer(BYE_SPACE, Handler::Exit);
    }

    fn offer_space_option(&mut self, phrase: Option<u32>) {
        if let Some(phrase) = phrase {
            if phrase == I_DEMAND_YOU_ALLY_SPACE0 {
                let mut fragments = vec![
                    I_DEMAND_YOU_ALLY_SPACE0 as i32,
                    crate::comm::races::script::GLOBAL_PLAYER_NAME,
                    I_DEMAND_YOU_ALLY_SPACE1 as i32,
                    crate::comm::races::script::GLOBAL_SHIP_NAME,
                    I_DEMAND_YOU_ALLY_SPACE2 as i32,
                ];
                fragments.extend(self.api.alliance_fragments(NAME_1 as u16));
                fragments.push(I_DEMAND_YOU_ALLY_SPACE3 as i32);
                self.api.response(phrase as u16, Handler::Space, &fragments);
            } else {
                self.offer(phrase, Handler::Space);
            }
        }
    }

    fn offer_pkunk(&mut self, handler: Handler) {
        if self.api.get("YEHAT_ROYALIST_TOLD_PKUNK") == 0
            && self.api.get("PKUNK_VISITS") != 0
            && self.api.get("PKUNK_HOME_VISITS") != 0
        {
            self.offer(WHAT_ABOUT_PKUNK_ROYALIST, handler);
        }
    }

    fn pkunk_response(&mut self) {
        let phrase = if self.api.get("YEHAT_ABSORBED_PKUNK") != 0 {
            PKUNK_ABSORBED_ROYALIST
        } else {
            HATE_PKUNK_ROYALIST
        };
        self.api.npc(phrase as i32);
    }

    pub(super) fn initialize(&mut self) {
        let peaceful = self.api.activity() as u8 == crate::comm::dispatch::WON_LAST_BATTLE as u8;
        if !peaceful {
            self.api.segue(1);
        } else {
            self.api.segue(0);
        }
    }

    pub(super) fn intro(&mut self) {
        if self.api.activity() as u8 == crate::comm::dispatch::WON_LAST_BATTLE as u8 {
            self.api.npc(OUT_TAKES as i32);
            self.api.segue(0);
            return;
        }
        let homeworld = self.api.get("GLOBAL_FLAGS_AND_DATA") & (1 << 7) != 0;
        if self.api.get("YEHAT_CIVIL_WAR") != 0 {
            if homeworld {
                self.counter(
                    "YEHAT_HOME_VISITS",
                    &[ROYALIST_HOMEWORLD_HELLO_1, ROYALIST_HOMEWORLD_HELLO_2],
                    1,
                );
            } else {
                self.counter(
                    "YEHAT_VISITS",
                    &[ROYALIST_SPACE_HELLO_1, ROYALIST_SPACE_HELLO_2],
                    1,
                );
            }
            self.royalists(NULL_PHRASE as i32);
        } else if homeworld {
            self.counter(
                "YEHAT_HOME_VISITS",
                &[HOMEWORLD_HELLO_1, HOMEWORLD_HELLO_2],
                1,
            );
            self.home(NULL_PHRASE as i32);
        } else {
            self.counter(
                "YEHAT_VISITS",
                &[SPACE_HELLO_1, SPACE_HELLO_2, SPACE_HELLO_3, SPACE_HELLO_4],
                3,
            );
            self.space(NULL_PHRASE as i32);
        }
    }
}
