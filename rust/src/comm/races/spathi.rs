//! Spathi dialogue, ported from the original C script.

use super::encounter::EncounterApi;
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
const SPATHI_SHIP: u16 = 6;
const GOOD_GUY: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    ExitConversation,
    SpathiOnPluto,
    SpathiMustGrovel,
    SpathiAllies,
    SpathiFriendly,
    SpathiBefriend,
    SpathiAntagonize,
    SpathiNeutral,
}
#[derive(Default)]
struct Session {
    refusals: u8,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
    session: &'a mut Session,
}
impl<A: EncounterApi<Handler>> Dialogue<'_, A> {
    fn handle(&mut self, handler: Handler, r: u16) {
        match handler {
            Handler::ExitConversation => self.exit_conversation(r),
            Handler::SpathiOnPluto => self.spathi_on_pluto(r),
            Handler::SpathiMustGrovel => self.spathi_must_grovel(r),
            Handler::SpathiAllies => self.spathi_allies(r),
            Handler::SpathiFriendly => self.spathi_friendly(r),
            Handler::SpathiBefriend => self.spathi_befriend(r),
            Handler::SpathiAntagonize => self.spathi_antagonize(r),
            Handler::SpathiNeutral => self.spathi_neutral(r),
        }
    }
    fn initialize(&mut self, desc: &mut CLocData) {
        if self.api.get("FOUND_PLUTO_SPATHI") == 1 {
            desc.alien_alt_song_res = c"comm.fwiffo.music".as_ptr();
            desc.alien_song_flags |= 1;
        } else {
            desc.alien_song_flags &= !1;
        }
        let peaceful = self.api.get("FOUND_PLUTO_SPATHI") == 1
            || self.api.get("SPATHI_MANNER") == 3
            || self.api.activity() as u8 == 5;
        self.api.segue(u32::from(!peaceful));
    }
    fn exit_conversation(&mut self, r: u16) {
        self.api.segue(0);
        if r == BYE_ALLY_SPACE {
            self.api.npc(i32::from(GOODBYE_ALLY_SPACE));
        } else if r == BYE_FRIENDLY_SPACE {
            self.api.npc(i32::from(GOODBYE_FRIENDLY_SPACE));
        } else if r == PART_IN_PEACE {
            self.api.npc(i32::from(KEEP_IT_SECRET));
        } else if r == WE_SORRY_SPACE {
            self.api.npc(i32::from(APOLOGIZE_AT_HOMEWORLD));
        } else if r == GIVE_INFO_SPACE {
            self.api.npc(i32::from(HERES_SOME_INFO));
        } else if r == BYE_ANGRY_SPACE {
            self.api.npc(i32::from(GOODBYE_ANGRY_SPACE));
        } else if r == ATTACK_YOU_NOW {
            self.api.npc(i32::from(YIPES));
            self.api.segue(1);
        } else if r == WE_FIGHT_AGAIN_SPACE {
            self.api.npc(i32::from(OK_FIGHT_AGAIN_SPACE));
            self.api.segue(1);
        } else if (((((r == DIE_SLUGBOY) || (r == WE_FIGHT_1)) || (r == WE_FIGHT_2))
            || (r == PAY_FOR_CRIMES))
            || (r == TELL_ME_COORDINATES))
            || (r == CHANGED_MIND)
        {
            if r == TELL_ME_COORDINATES {
                self.api.npc(i32::from(FAKE_COORDINATES));
            }
            self.api.npc(i32::from(OK_WE_FIGHT_AT_PLUTO));
            self.api.segue(1);
        } else if r == JOIN_US {
            if self.api.escort_feasibility(SPATHI_SHIP) == 0 {
                self.api.npc(i32::from(TOO_SCARY));
            } else {
                self.api.npc(i32::from(WILL_JOIN));
                self.api.talk_segue(!0_u16);
                self.api.add_escorts(SPATHI_SHIP, 1);
                self.api.fwiffo_crew();
            }
        }
    }
    fn spathi_on_pluto(&mut self, r: u16) {
        if r == HI_THERE {
            self.api.npc(i32::from(ARE_YOU_SURE));
        } else if r == DONT_KILL {
            self.api.npc(i32::from(OK_WONT));
        } else if r == YOURE_FORGIVEN {
            self.api.npc(i32::from(THANKS_FOR_FORGIVENESS));
        } else if r == YOU_WONT_DIE_YET {
            self.api.npc(i32::from(ETERNAL_GRATITUDE));
        } else if r == YOU_MAY_LIVE {
            self.api.npc(i32::from(GEE_THANKS));
        } else if r == YOUVE_GOT_ME_ALL_WRONG {
            self.api.npc(i32::from(SORRY_NO_COORDS));
        } else if r == WHAT_DOING_ON_PLUTO_1 {
            self.api.npc(i32::from(ABOUT_20_YEARS_AGO));
            self.api.disable(WHAT_DOING_ON_PLUTO_1);
        } else if r == WHAT_DOING_ON_PLUTO_2 {
            self.api.npc(i32::from(WHEN_URQUAN_ARRIVED));
            self.api.disable(WHAT_DOING_ON_PLUTO_2);
        } else if r == WHAT_DOING_ON_PLUTO_3 {
            self.api.npc(i32::from(STATIONED_ON_EARTH_MOON));
            self.api.disable(WHAT_DOING_ON_PLUTO_3);
        } else if r == WHAT_ABOUT_ILWRATH {
            self.api.npc(i32::from(ABOUT_ILWRATH));
            self.api.disable(WHAT_ABOUT_ILWRATH);
        } else if r == WHEN_ILWRATH {
            self.api.npc(i32::from(THEN_ILWRATH));
            self.api.disable(WHEN_ILWRATH);
        } else if r == WHAT_ABOUT_MOONBASE {
            self.api.npc(i32::from(SET_UP_BASE));
            self.api.disable(WHAT_ABOUT_MOONBASE);
        } else if r == WHAT_ABOUT_OTHER_SPATHI {
            self.api.npc(i32::from(SPATHI_ARE));
            self.api.disable(WHAT_ABOUT_OTHER_SPATHI);
        } else if r == HOW_MANY_CREW {
            self.api.npc(i32::from(THOUSANDS));
            self.api.disable(HOW_MANY_CREW);
        } else if r == REALLY_THOUSANDS {
            self.api.npc(i32::from(JUST_ME));
            self.api.disable(REALLY_THOUSANDS);
        } else if r == FULL_OF_MONSTERS {
            self.api.npc(i32::from(HOW_TRUE));
            self.api.disable(FULL_OF_MONSTERS);
        } else if r == WHAT_ENEMY {
            self.api.npc(i32::from(ENEMY_IS));
            self.api.disable(WHAT_ENEMY);
        } else if r == WHY_YOU_HERE {
            self.api.npc(i32::from(DREW_SHORT_STRAW));
            self.api.disable(WHY_YOU_HERE);
        } else if r == WHERE_ARE_URQUAN {
            self.api.npc(i32::from(URQUAN_LEFT));
            self.api.disable(WHERE_ARE_URQUAN);
        } else if r == WHAT_ABOUT_OTHER_RACES {
            self.api.npc(i32::from(ABOUT_OTHER_RACES));
            self.api.disable(WHAT_ABOUT_OTHER_RACES);
        } else if r == WHAT_BLAZE_OF_GLORY {
            self.api.npc(i32::from(BLAZE_IS));
            self.api.disable(WHAT_BLAZE_OF_GLORY);
        } else if r == WHAT_ABOUT_YOURSELF {
            self.api.npc(i32::from(ABOUT_MYSELF));
            self.api.disable(WHAT_ABOUT_YOURSELF);
        } else if r == JOIN_US {
            if self.session.refusals == 0 {
                self.api.npc(i32::from(WONT_JOIN_1));
                self.session.refusals += 1;
            } else if self.session.refusals == 1 {
                self.api.npc(i32::from(WONT_JOIN_2));
                self.session.refusals += 1;
            } else {
                self.api.npc(i32::from(WONT_JOIN_3));
            }
        }
        if self.api.enabled(WHAT_DOING_ON_PLUTO_1) {
            self.api
                .response(WHAT_DOING_ON_PLUTO_1, Handler::SpathiOnPluto, &[]);
        } else if self.api.enabled(WHAT_DOING_ON_PLUTO_2) {
            self.api
                .response(WHAT_DOING_ON_PLUTO_2, Handler::SpathiOnPluto, &[]);
        } else if self.api.enabled(WHAT_DOING_ON_PLUTO_3) {
            self.api
                .response(WHAT_DOING_ON_PLUTO_3, Handler::SpathiOnPluto, &[]);
        } else {
            if self.api.enabled(WHAT_ABOUT_ILWRATH) {
                self.api
                    .response(WHAT_ABOUT_ILWRATH, Handler::SpathiOnPluto, &[]);
            } else if self.api.enabled(WHEN_ILWRATH) {
                self.api.response(WHEN_ILWRATH, Handler::SpathiOnPluto, &[]);
            }
            if self.api.enabled(WHAT_ABOUT_MOONBASE) {
                self.api
                    .response(WHAT_ABOUT_MOONBASE, Handler::SpathiOnPluto, &[]);
            } else if self.api.enabled(WHAT_ABOUT_OTHER_SPATHI) {
                self.api
                    .response(WHAT_ABOUT_OTHER_SPATHI, Handler::SpathiOnPluto, &[]);
            } else {
                if self.api.enabled(HOW_MANY_CREW) {
                    self.api
                        .response(HOW_MANY_CREW, Handler::SpathiOnPluto, &[]);
                } else if self.api.enabled(REALLY_THOUSANDS) {
                    self.api
                        .response(REALLY_THOUSANDS, Handler::SpathiOnPluto, &[]);
                } else if self.api.enabled(FULL_OF_MONSTERS) {
                    self.api
                        .response(FULL_OF_MONSTERS, Handler::SpathiOnPluto, &[]);
                }
                if self.api.enabled(WHAT_ENEMY) {
                    self.api.response(WHAT_ENEMY, Handler::SpathiOnPluto, &[]);
                } else if self.api.enabled(WHY_YOU_HERE) {
                    self.api.response(WHY_YOU_HERE, Handler::SpathiOnPluto, &[]);
                }
            }
        }
        if self.api.enabled(WHERE_ARE_URQUAN) {
            self.api
                .response(WHERE_ARE_URQUAN, Handler::SpathiOnPluto, &[]);
        } else if self.api.enabled(WHAT_ABOUT_OTHER_RACES) {
            self.api
                .response(WHAT_ABOUT_OTHER_RACES, Handler::SpathiOnPluto, &[]);
        } else if self.api.enabled(WHAT_BLAZE_OF_GLORY) {
            self.api
                .response(WHAT_BLAZE_OF_GLORY, Handler::SpathiOnPluto, &[]);
        } else if self.api.enabled(WHAT_ABOUT_YOURSELF) {
            self.api
                .response(WHAT_ABOUT_YOURSELF, Handler::SpathiOnPluto, &[]);
        }
        if !(self.api.enabled(FULL_OF_MONSTERS)) {
            self.api.response(JOIN_US, Handler::ExitConversation, &[]);
        } else {
            self.api.response(JOIN_US, Handler::SpathiOnPluto, &[]);
        }
        self.api
            .response(CHANGED_MIND, Handler::ExitConversation, &[]);
    }
    fn spathi_must_grovel(&mut self, r: u16) {
        if r == IDENTIFY {
            self.api.npc(i32::from(I_FWIFFO));
            self.api
                .response(DO_CULTURAL, Handler::SpathiMustGrovel, &[]);
            self.api
                .response(YOURE_FORGIVEN, Handler::SpathiOnPluto, &[]);
            self.api
                .response(DIE_SLUGBOY, Handler::ExitConversation, &[]);
        } else if r == DO_CULTURAL {
            self.api.npc(i32::from(WEZZY_WEZZAH));
            self.api
                .response(BEGIN_RITUAL, Handler::SpathiMustGrovel, &[]);
            self.api
                .response(YOU_WONT_DIE_YET, Handler::SpathiOnPluto, &[]);
            self.api
                .response(WE_FIGHT_2, Handler::ExitConversation, &[]);
        } else if r == BEGIN_RITUAL {
            self.api.npc(i32::from(MUST_DO_RITUAL_AT_HOME));
            self.api.response(YOU_MAY_LIVE, Handler::SpathiOnPluto, &[]);
            self.api
                .response(PAY_FOR_CRIMES, Handler::ExitConversation, &[]);
            self.api
                .response(WHAT_ARE_COORDINATES, Handler::SpathiMustGrovel, &[]);
        } else {
            self.api.npc(i32::from(COORDINATES_ARE));
            self.api
                .response(YOUVE_GOT_ME_ALL_WRONG, Handler::SpathiOnPluto, &[]);
            self.api
                .response(TELL_ME_COORDINATES, Handler::ExitConversation, &[]);
        }
    }
    fn spathi_allies(&mut self, r: u16) {
        if r == 0 {
            let mut num_visits: u8;
            num_visits = self.api.get("SPATHI_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INIT_ALLIED_HELLO_SPACE));
                }
                1 => {
                    self.api.npc(i32::from(SUBSEQUENT_ALLIED_HELLO_SPACE));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("SPATHI_VISITS", num_visits);
        } else if r == WHATS_UP_SPACE_2 {
            self.api.npc(i32::from(GENERAL_INFO_SPACE_2));
            self.api.disable(WHATS_UP_SPACE_2);
        } else if r == GIVE_US_INFO_FROM_SPACE {
            self.api.npc(i32::from(GET_INFO_FROM_SPATHIWA));
            self.api.disable(GIVE_US_INFO_FROM_SPACE);
        } else if r == GIVE_US_RESOURCES_SPACE {
            self.api.npc(i32::from(GET_RESOURCES_FROM_SPATHIWA));
            self.api.disable(GIVE_US_RESOURCES_SPACE);
        } else if r == WHAT_DO_FOR_FUN {
            self.api.npc(i32::from(DO_THIS_FOR_FUN));
            self.api.disable(WHAT_DO_FOR_FUN);
        }
        if self.api.enabled(WHATS_UP_SPACE_2) {
            self.api
                .response(WHATS_UP_SPACE_2, Handler::SpathiAllies, &[]);
        }
        if self.api.enabled(GIVE_US_INFO_FROM_SPACE) {
            self.api
                .response(GIVE_US_INFO_FROM_SPACE, Handler::SpathiAllies, &[]);
        }
        if self.api.enabled(GIVE_US_RESOURCES_SPACE) {
            self.api
                .response(GIVE_US_RESOURCES_SPACE, Handler::SpathiAllies, &[]);
        }
        if self.api.enabled(WHAT_DO_FOR_FUN) {
            self.api
                .response(WHAT_DO_FOR_FUN, Handler::SpathiAllies, &[]);
        }
        self.api
            .response(BYE_ALLY_SPACE, Handler::ExitConversation, &[]);
    }
    fn spathi_friendly(&mut self, r: u16) {
        if r == 0 {
            let mut num_visits: u8;
            num_visits = self.api.get("SPATHI_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INIT_FRIENDLY_HELLO_SPACE));
                }
                1 => {
                    self.api.npc(i32::from(SUBSEQUENT_FRIENDLY_HELLO_SPACE));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("SPATHI_VISITS", num_visits);
        } else if r == SINCE_FRIENDLY_GIVE_STUFF {
            self.api.npc(i32::from(GIVE_ADVICE));
            self.api.disable(SINCE_FRIENDLY_GIVE_STUFF);
        } else if r == WHATS_UP_SPACE_1 {
            self.api.npc(i32::from(GENERAL_INFO_SPACE_1));
            self.api.disable(WHATS_UP_SPACE_1);
        }
        if self.api.enabled(WHATS_UP_SPACE_1) {
            self.api
                .response(WHATS_UP_SPACE_1, Handler::SpathiFriendly, &[]);
        }
        if self.api.enabled(SINCE_FRIENDLY_GIVE_STUFF) {
            self.api
                .response(SINCE_FRIENDLY_GIVE_STUFF, Handler::SpathiFriendly, &[]);
        }
        self.api
            .response(BYE_FRIENDLY_SPACE, Handler::ExitConversation, &[]);
    }
    fn spathi_befriend(&mut self, r: u16) {
        let mut info_left: bool;
        let mut last_stack: u8;
        let mut p_str: [u16; 2] = [0; 2];
        info_left = false;
        last_stack = 0;
        p_str[1] = 0;
        p_str[0] = p_str[1];
        if r == COME_IN_PEACE {
            self.api.npc(i32::from(AGAINST_NATURE));
        } else if r == LOOKING_FOR_A_FEW_GOOD_SQUIDS {
            self.api.npc(i32::from(URQUAN_SLAVES));
            self.api.disable(LOOKING_FOR_A_FEW_GOOD_SQUIDS);
        } else if r == WHY_SLAVES {
            self.api.npc(i32::from(UMGAH_TRICK));
            self.api.disable(WHY_SLAVES);
        } else if r == TELL_US_ABOUT_YOU {
            self.api.npc(i32::from(ABOUT_US));
            self.api.disable(TELL_US_ABOUT_YOU);
            last_stack = 1;
        } else if r == WHAT_YOU_REALLY_WANT {
            self.api.npc(i32::from(WANT_THIS));
            self.api.disable(WHAT_YOU_REALLY_WANT);
        } else if r == HOW_ABOUT_ALLIANCE {
            self.api.npc(i32::from(SURE));
            self.api.disable(HOW_ABOUT_ALLIANCE);
        }
        if self.api.enabled(LOOKING_FOR_A_FEW_GOOD_SQUIDS) {
            p_str[0] = LOOKING_FOR_A_FEW_GOOD_SQUIDS;
        } else if self.api.enabled(WHY_SLAVES) {
            p_str[0] = WHY_SLAVES;
        }
        if self.api.enabled(TELL_US_ABOUT_YOU) {
            p_str[1] = TELL_US_ABOUT_YOU;
        } else if self.api.enabled(WHAT_YOU_REALLY_WANT) {
            p_str[1] = WHAT_YOU_REALLY_WANT;
        }
        if p_str[last_stack as usize] != 0 {
            info_left = true;
            self.api
                .response(p_str[last_stack as usize], Handler::SpathiBefriend, &[]);
        }
        last_stack ^= 1;
        if p_str[last_stack as usize] != 0 {
            info_left = true;
            self.api
                .response(p_str[last_stack as usize], Handler::SpathiBefriend, &[]);
        }
        if self.api.enabled(HOW_ABOUT_ALLIANCE) {
            info_left = true;
            self.api
                .response(HOW_ABOUT_ALLIANCE, Handler::SpathiBefriend, &[]);
        }
        if !(info_left) {
            self.api.set("SPATHI_STACK1", 1_u8);
            self.spathi_neutral(r);
        }
    }
    fn spathi_antagonize(&mut self, r: u16) {
        if r == PREPARE_TO_DIE {
            self.api.npc(i32::from(ALWAYS_PREPARED));
            self.api.set("SPATHI_STACK2", 1_u8);
        } else if r == HEARD_YOURE_COWARDS {
            self.api.npc(i32::from(DARN_TOOTIN));
            self.api.disable(HEARD_YOURE_COWARDS);
        } else if r == WANNA_FIGHT {
            self.api.npc(i32::from(YES_WE_DO));
            self.api.disable(WANNA_FIGHT);
        } else if r == SO_LETS_FIGHT {
            self.api.npc(i32::from(OK_LETS_FIGHT));
            self.api.disable(SO_LETS_FIGHT);
        } else if r == SO_LETS_FIGHT_ALREADY {
            self.api.npc(i32::from(DONT_REALLY_WANT_TO_FIGHT));
            self.api.disable(SO_LETS_FIGHT_ALREADY);
        }
        if self.api.enabled(WANNA_FIGHT) {
            self.api
                .response(WANNA_FIGHT, Handler::SpathiAntagonize, &[]);
        } else if self.api.enabled(SO_LETS_FIGHT) {
            self.api
                .response(SO_LETS_FIGHT, Handler::SpathiAntagonize, &[]);
        } else if self.api.enabled(SO_LETS_FIGHT_ALREADY) {
            self.api
                .response(SO_LETS_FIGHT_ALREADY, Handler::SpathiAntagonize, &[]);
        }
        if self.api.enabled(HEARD_YOURE_COWARDS) {
            self.api
                .response(HEARD_YOURE_COWARDS, Handler::SpathiAntagonize, &[]);
        }
        self.api
            .response(ATTACK_YOU_NOW, Handler::ExitConversation, &[]);
    }
    fn spathi_neutral(&mut self, r: u16) {
        if r == 0 {
            let mut num_visits: u8;
            num_visits = self.api.get("SPATHI_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INIT_NEUTRAL_HELLO_SPACE));
                }
                1 => {
                    self.api.npc(i32::from(SUBSEQUENT_NEUTRAL_HELLO_SPACE));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("SPATHI_VISITS", num_visits);
        } else if r == LOOK_WEIRD {
            self.api.npc(i32::from(YOU_LOOK_WEIRD));
            self.api.set("SPATHI_STACK0", 1_u8);
        } else if r == NO_LOOK_REALLY_WEIRD {
            self.api.npc(i32::from(NO_YOU_LOOK_REALLY_WEIRD));
            self.api.set("SPATHI_STACK0", 2_u8);
        }
        match self.api.get("SPATHI_STACK0") {
            0 => {
                self.api.response(LOOK_WEIRD, Handler::SpathiNeutral, &[]);
            }
            1 => {
                self.api
                    .response(NO_LOOK_REALLY_WEIRD, Handler::SpathiNeutral, &[]);
            }
            _ => {}
        }
        if self.api.get("SPATHI_STACK1") == 0 {
            self.api
                .response(COME_IN_PEACE, Handler::SpathiBefriend, &[]);
        }
        if self.api.get("SPATHI_STACK2") == 0 {
            self.api
                .response(PREPARE_TO_DIE, Handler::SpathiAntagonize, &[]);
        } else {
            self.api
                .response(ATTACK_YOU_NOW, Handler::ExitConversation, &[]);
        }
        self.api
            .response(PART_IN_PEACE, Handler::ExitConversation, &[]);
    }
    fn intro(&mut self) {
        let manner: u8 = self.api.get("SPATHI_MANNER");
        if self.api.get("FOUND_PLUTO_SPATHI") == 1 {
            self.session.refusals = 0;
            self.api.npc(i32::from(SORRY_ABOUT_THAT));
            if self.api.get("SPATHI_HOME_VISITS") == 7 {
                self.api.set("KNOW_SPATHI_PASSWORD", 0_u8);
                self.api.set("SPATHI_HOME_VISITS", 0_u8);
            }
            self.api.response(IDENTIFY, Handler::SpathiMustGrovel, &[]);
            self.api.response(HI_THERE, Handler::SpathiOnPluto, &[]);
            self.api.response(DONT_KILL, Handler::SpathiOnPluto, &[]);
            self.api
                .response(WE_FIGHT_1, Handler::ExitConversation, &[]);
        } else if manner == 2 {
            self.api.npc(i32::from(HATE_YOU_FOREVER_SPACE));
            self.api.segue(1);
        } else if manner == 1 {
            let mut num_visits: u8;
            num_visits = self.api.get("SPATHI_VISITS");
            let res = {
                let old = num_visits;
                num_visits += 1;
                old
            };
            match res {
                0 => {
                    self.api.npc(i32::from(INIT_ANGRY_HELLO_SPACE));
                }
                1 => {
                    self.api.npc(i32::from(SUBSEQUENT_ANGRY_HELLO_SPACE));
                    num_visits -= 1;
                }
                _ => {}
            }
            self.api.set("SPATHI_VISITS", num_visits);
            self.api
                .response(GIVE_INFO_SPACE, Handler::ExitConversation, &[]);
            self.api
                .response(WE_SORRY_SPACE, Handler::ExitConversation, &[]);
            self.api
                .response(WE_FIGHT_AGAIN_SPACE, Handler::ExitConversation, &[]);
            self.api
                .response(BYE_ANGRY_SPACE, Handler::ExitConversation, &[]);
        } else if self.api.alliance(SPATHI_SHIP) == GOOD_GUY {
            self.spathi_allies(0_u16);
        } else if self.api.get("SPATHI_QUEST") != 0 {
            self.spathi_friendly(0_u16);
        } else {
            self.spathi_neutral(0_u16);
        }
    }
    fn post_spathi_enc(&mut self) {
        if self.api.get("FOUND_PLUTO_SPATHI") == 1 {
            self.api.set("FOUND_PLUTO_SPATHI", 2);
        } else if self.api.get_segue() == 1 {
            let manner = self.api.get("SPATHI_MANNER");
            if manner != 2 {
                self.api.set("SPATHI_MANNER", 1);
                if manner != 1 {
                    self.api.set("SPATHI_VISITS", 0);
                    if self.api.get("SPATHI_HOME_VISITS") != 7 {
                        self.api.set("SPATHI_HOME_VISITS", 0);
                    }
                }
            }
        }
    }
}
fn descriptor() -> CLocData {
    let mut desc = CLocData {
        alien_frame_res: c"comm.spathi.graphics".as_ptr(),
        alien_font_res: c"comm.spathi.font".as_ptr(),
        alien_colormap_res: c"comm.spathi.colortable".as_ptr(),
        alien_song_res: c"comm.spathi.music".as_ptr(),
        conversation_phrases_res: c"comm.spathi.dialogue".as_ptr(),
        alien_text_fcolor: CColor {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        },
        alien_text_bcolor: CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        },
        alien_text_baseline: CPoint { x: 164, y: 0 },
        alien_text_width: 288,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 8,
        ..CLocData::default()
    };
    desc.alien_ambient_array[0] = animation(1, 6, 2, (42, 0), (0, 840));
    desc.alien_ambient_array[0].block_mask = 2;
    desc.alien_ambient_array[1] = animation(7, 9, 2, (42, 0), (42, 0));
    desc.alien_ambient_array[1].block_mask = 1;
    desc.alien_ambient_array[2] = animation(16, 4, 2, (84, 56), (84, 56));
    desc.alien_ambient_array[2].block_mask = 16;
    desc.alien_ambient_array[3] = animation(20, 4, 4, (56, 56), (56, 56));
    desc.alien_ambient_array[3].block_mask = 32;
    desc.alien_ambient_array[4] = animation(24, 5, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[4].block_mask = 4;
    desc.alien_ambient_array[5] = animation(34, 4, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[5].block_mask = 8;
    desc.alien_ambient_array[6] = animation(38, 3, 4, (56, 0), (840, 2520));
    desc.alien_ambient_array[7] = animation(41, 3, 4, (56, 0), (840, 2520));
    desc.alien_talk_desc = animation(0, 0, 0, (0, 0), (0, 0));
    #[cfg(not(test))]
    {
        desc.init_encounter_func = Some(live::intro);
        desc.post_encounter_func = Some(live::post);
        desc.uninit_encounter_func = Some(live::uninit);
    }
    desc
}
#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        static mut SESSION: Option<Session> = None;
        // Only the game thread dispatches these callbacks.
        let session =
            unsafe { (&mut *std::ptr::addr_of_mut!(SESSION)).get_or_insert_with(Session::default) };
        f(&mut Dialogue {
            api: &mut api,
            session,
        });
    }
    fn exit_conversation(r: u32) {
        run(|s| {
            s.handle(
                Handler::ExitConversation,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn spathi_on_pluto(r: u32) {
        run(|s| {
            s.handle(
                Handler::SpathiOnPluto,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn spathi_must_grovel(r: u32) {
        run(|s| {
            s.handle(
                Handler::SpathiMustGrovel,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn spathi_allies(r: u32) {
        run(|s| {
            s.handle(
                Handler::SpathiAllies,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn spathi_friendly(r: u32) {
        run(|s| {
            s.handle(
                Handler::SpathiFriendly,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn spathi_befriend(r: u32) {
        run(|s| {
            s.handle(
                Handler::SpathiBefriend,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn spathi_antagonize(r: u32) {
        run(|s| {
            s.handle(
                Handler::SpathiAntagonize,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn spathi_neutral(r: u32) {
        run(|s| {
            s.handle(
                Handler::SpathiNeutral,
                u16::try_from(r).expect("response reference exceeds COUNT"),
            )
        });
    }
    fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
        match handler {
            Handler::ExitConversation => exit_conversation,
            Handler::SpathiOnPluto => spathi_on_pluto,
            Handler::SpathiMustGrovel => spathi_must_grovel,
            Handler::SpathiAllies => spathi_allies,
            Handler::SpathiFriendly => spathi_friendly,
            Handler::SpathiBefriend => spathi_befriend,
            Handler::SpathiAntagonize => spathi_antagonize,
            Handler::SpathiNeutral => spathi_neutral,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post_spathi_enc());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter descriptor and initial battle segue.
    ///
    /// # Safety
    /// Game thread only. The caller must copy the descriptor before another encounter.
    #[no_mangle]
    pub unsafe extern "C" fn init_spathi_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize(desc));
        desc
    }
}
pub struct SpathiDialogue;
impl super::RaceDialogue for SpathiDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_spathi_comm()) }
        }
        #[cfg(test)]
        {
            as_comm_data(&descriptor())
        }
    }
    fn intro(&self) {
        #[cfg(not(test))]
        live::intro();
    }
    fn post_encounter(&self) {
        #[cfg(not(test))]
        live::post();
    }
    fn uninit(&self) -> u32 {
        0
    }
}
#[cfg(test)]
mod tests;
