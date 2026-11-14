//! Earth Commander dialogue, ported from comm/comandr/comandr.c.

pub(crate) use super::script::{
    DialogueHost, ANIM_DISABLED, FUEL_TANK_SCALE, GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME, ONE_SECOND,
};
pub(crate) use crate::comm::races::descriptor::CommDescriptor;
pub(crate) use crate::engine::StoryKey;

pub(crate) const GLAD_WHEN_YOU_COME_BACK: u16 = 1;
pub(crate) const GIVE_FUEL: u16 = 2;
pub(crate) const GIVE_FUEL_AGAIN: u16 = 3;
pub(crate) const ARE_YOU_SUPPLY_SHIP: u16 = 4;
pub(crate) const DO_YOU_HAVE_RADIO_THIS_TIME: u16 = 5;
pub(crate) const HERE_IS_ANOTHER_LANDER: u16 = 6;
pub(crate) const THE_WHAT_FROM_WHERE: u16 = 7;
pub(crate) const ABOUT_TIME: u16 = 8;
pub(crate) const MESSAGE_GARBLED_1: u16 = 9;
pub(crate) const MESSAGE_GARBLED_2: u16 = 10;
pub(crate) const HERE_IS_A_NEW_LANDER: u16 = 11;
pub(crate) const THIS_MAY_SEEM_SILLY: u16 = 12;
pub(crate) const OK_THE_NAFS: u16 = 13;
pub(crate) const OK_THE_CAN: u16 = 14;
pub(crate) const OK_THE_UFW: u16 = 15;
pub(crate) const OK_THE_NAME_IS_EMPIRE0: u16 = 16;
pub(crate) const OK_THE_NAME_IS_EMPIRE1: u16 = 17;
pub(crate) const FUEL_UP0: u16 = 18;
pub(crate) const FUEL_UP1: u16 = 19;
pub(crate) const WHAT_KIND_OF_IDIOT: u16 = 20;
pub(crate) const DONT_KNOW_WHO_YOU_ARE: u16 = 21;
pub(crate) const THATS_IMPOSSIBLE: u16 = 22;
pub(crate) const ASK_AWAY: u16 = 23;
pub(crate) const RADIOS_ON_MERCURY: u16 = 24;
pub(crate) const THANKS_FOR_HELPING: u16 = 25;
pub(crate) const STARBASE_IS: u16 = 26;
pub(crate) const HAPPENED_TO_EARTH: u16 = 27;
pub(crate) const URQUAN_LEFT: u16 = 28;
pub(crate) const BASE_ON_MOON: u16 = 29;
pub(crate) const ACKNOWLEDGE_SECRET: u16 = 30;
pub(crate) const ABOUT_BASE: u16 = 31;
pub(crate) const GOOD_LUCK_WITH_BASE: u16 = 32;
pub(crate) const DEALT_WITH_BASE_YET: u16 = 33;
pub(crate) const HERE_COMES_ILWRATH: u16 = 34;
pub(crate) const VERY_IMPRESSIVE: u16 = 35;
pub(crate) const IT_WAS_ABANDONED: u16 = 36;
pub(crate) const YOU_REALLY_FOUGHT_BASE: u16 = 37;
pub(crate) const IM_GLAD_YOU_WON: u16 = 38;
pub(crate) const IM_SURE_IT_WAS_DIFFICULT: u16 = 39;
pub(crate) const THAT_WAS_PROBE: u16 = 40;
pub(crate) const DEEP_TROUBLE: u16 = 41;
pub(crate) const GOOD_NEWS: u16 = 42;
pub(crate) const SURE_HOPE: u16 = 43;
pub(crate) const ABOUT_BASE_AGAIN: u16 = 44;
pub(crate) const COOK_BUTTS: u16 = 45;
pub(crate) const OVERTHROW_ALIENS: u16 = 46;
pub(crate) const KILL_MONSTERS: u16 = 47;
pub(crate) const GOOD_LUCK_AGAIN: u16 = 48;
pub(crate) const STARBASE_WILL_BE_READY: u16 = 49;
pub(crate) const OVERTHROW_EVIL_ALIENS: u16 = 50;
pub(crate) const ANNIHILATE_THOSE_MONSTERS: u16 = 51;
pub(crate) const COOK_THEIR_BUTTS: u16 = 52;
pub(crate) const WHERE_GET_RADIOS: u16 = 53;
pub(crate) const WELL_GO_GET_THEM_NOW: u16 = 54;
pub(crate) const WE_WILL_TRANSFER_NOW: u16 = 55;
pub(crate) const WHAT_WILL_YOU_GIVE_US: u16 = 56;
pub(crate) const BEFORE_RADIOS_WE_NEED_INFO: u16 = 57;
pub(crate) const NO_BUT_WELL_HELP0: u16 = 58;
pub(crate) const NO_BUT_WELL_HELP1: u16 = 59;
pub(crate) const YES_THIS_IS_SUPPLY_SHIP: u16 = 60;
pub(crate) const WHAT_SLAVE_PLANET: u16 = 61;
pub(crate) const I_LIED: u16 = 62;
pub(crate) const PLUMB_OUT: u16 = 63;
pub(crate) const WE_ARE_VINDICATOR0: u16 = 64;
pub(crate) const WE_ARE_VINDICATOR1: u16 = 65;
pub(crate) const WE_ARE_VINDICATOR2: u16 = 66;
pub(crate) const FIRST_GIVE_INFO: u16 = 67;
pub(crate) const WHERE_CAN_I_GET_RADIOS: u16 = 69;
pub(crate) const OK_I_WILL_GET_RADIOS: u16 = 70;
pub(crate) const WHATS_THIS_STARBASE: u16 = 71;
pub(crate) const WHAT_ABOUT_EARTH: u16 = 72;
pub(crate) const WHERE_ARE_URQUAN: u16 = 73;
pub(crate) const OUR_MISSION_WAS_SECRET: u16 = 74;
pub(crate) const WE_ARE_HERE_TO_HELP: u16 = 75;
pub(crate) const TELL_ME_ABOUT_BASE: u16 = 76;
pub(crate) const WE_WILL_TAKE_CARE_OF_BASE: u16 = 77;
pub(crate) const TELL_ME_AGAIN: u16 = 78;
pub(crate) const BASE_WAS_ABANDONED: u16 = 79;
pub(crate) const WE_FOUGHT_THEM: u16 = 80;
pub(crate) const OH_YES_BIG_FIGHT: u16 = 81;
pub(crate) const I_LIED_IT_WAS_ABANDONED: u16 = 82;
pub(crate) const I_CANT_TALK_ABOUT_IT: u16 = 83;
pub(crate) const NAME_1: u16 = 84;
pub(crate) const NAME_2: u16 = 85;
pub(crate) const NAME_3: u16 = 86;
pub(crate) const NAME_40: u16 = 87;
pub(crate) const NAME_41: u16 = 88;
pub(crate) const I_LOST_MY_LANDER: u16 = 89;
pub(crate) const I_LOST_ANOTHER_LANDER: u16 = 90;
pub(crate) const NEED_FUEL_MERCURY: u16 = 91;
pub(crate) const NEED_FUEL_LUNA: u16 = 92;
pub(crate) const NEED_FUEL_AGAIN: u16 = 93;
pub(crate) const WHAT_WAS_RED_THING: u16 = 94;
pub(crate) const IT_WENT_AWAY: u16 = 95;
pub(crate) const WE_DESTROYED_IT: u16 = 96;
pub(crate) const WHAT_PROBE: u16 = 97;
pub(crate) const TAKE_CARE_OF_BASE_AGAIN: u16 = 98;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    ByeBye,
    NoRadioactives,
    AskAfterRadios,
    BaseDestroyed,
    TellMoonBase,
    TellProbe,
    RevealSelf,
    GiveRadios,
}

pub(crate) struct Commander<'a, A>(pub(crate) &'a mut A);

impl<A: DialogueHost<Handler>> Commander<'_, A> {
    pub(crate) fn say(&mut self, phrase: u16) {
        self.0.npc(i32::from(phrase));
    }

    pub(crate) fn offer(&mut self, phrase: u16, handler: Handler) {
        self.0.response(phrase, handler, &[]);
    }

    pub(crate) fn handle(&mut self, handler: Handler, r: u16) {
        match handler {
            Handler::ByeBye => self.bye_bye(r),
            Handler::NoRadioactives => self.no_radioactives(r),
            Handler::AskAfterRadios => self.ask_after_radios(r),
            Handler::BaseDestroyed => self.base_destroyed(r),
            Handler::TellMoonBase => self.tell_moon_base(r),
            Handler::TellProbe => self.tell_probe(),
            Handler::RevealSelf => self.reveal_self(r),
            Handler::GiveRadios => self.give_radios(r),
        }
    }

    pub(crate) fn bye_bye(&mut self, r: u16) {
        match r {
            OK_I_WILL_GET_RADIOS => self.say(THANKS_FOR_HELPING),
            WELL_GO_GET_THEM_NOW => self.say(GLAD_WHEN_YOU_COME_BACK),
            WE_WILL_TAKE_CARE_OF_BASE => {
                self.say(GOOD_LUCK_WITH_BASE);
                self.0.set(StoryKey::WILL_DESTROY_BASE, 1);
            }
            TAKE_CARE_OF_BASE_AGAIN => self.say(GOOD_LUCK_AGAIN),
            BASE_WAS_ABANDONED
            | I_LIED_IT_WAS_ABANDONED
            | OH_YES_BIG_FIGHT
            | I_CANT_TALK_ABOUT_IT => {
                self.say(match r {
                    OH_YES_BIG_FIGHT => IM_GLAD_YOU_WON,
                    I_CANT_TALK_ABOUT_IT => IM_SURE_IT_WAS_DIFFICULT,
                    _ => IT_WAS_ABANDONED,
                });
                self.say(HERE_COMES_ILWRATH);
                self.0.set(StoryKey::PROBE_ILWRATH_ENCOUNTER, 1);
            }
            COOK_THEIR_BUTTS | OVERTHROW_EVIL_ALIENS | ANNIHILATE_THOSE_MONSTERS => {
                self.0.set(StoryKey::PROBE_ILWRATH_ENCOUNTER, 0);
                self.say(match r {
                    COOK_THEIR_BUTTS => COOK_BUTTS,
                    OVERTHROW_EVIL_ALIENS => OVERTHROW_ALIENS,
                    _ => KILL_MONSTERS,
                });
                self.say(THIS_MAY_SEEM_SILLY);
                for phrase in [NAME_1, NAME_2, NAME_3] {
                    self.offer(phrase, Handler::ByeBye);
                }
                self.0.response(
                    NAME_40,
                    Handler::ByeBye,
                    &[i32::from(NAME_40), GLOBAL_PLAYER_NAME, i32::from(NAME_41)],
                );
                self.0.set(StoryKey::STARBASE_AVAILABLE, 1);
            }
            _ => {
                let name = match r {
                    NAME_1 => {
                        self.say(OK_THE_NAFS);
                        0
                    }
                    NAME_2 => {
                        self.say(OK_THE_CAN);
                        1
                    }
                    NAME_3 => {
                        self.say(OK_THE_UFW);
                        2
                    }
                    _ => {
                        self.say(OK_THE_NAME_IS_EMPIRE0);
                        self.0.npc(GLOBAL_PLAYER_NAME);
                        self.say(OK_THE_NAME_IS_EMPIRE1);
                        3
                    }
                };
                self.0.set(StoryKey::NEW_ALLIANCE_NAME, name);
                self.say(STARBASE_WILL_BE_READY);
            }
        }
    }

    // These rescue effects and their ordering are shared by three menus.
    pub(crate) fn rescue(&mut self, r: u16) -> bool {
        match r {
            I_LOST_MY_LANDER | I_LOST_ANOTHER_LANDER => {
                self.say(if r == I_LOST_MY_LANDER {
                    HERE_IS_A_NEW_LANDER
                } else {
                    HERE_IS_ANOTHER_LANDER
                });
                self.0.add_lander();
                self.0.gauges(4, 0, 0);
                if r == I_LOST_MY_LANDER {
                    self.0.set(StoryKey::LANDERS_LOST, 1);
                }
            }
            NEED_FUEL_MERCURY | NEED_FUEL_LUNA | NEED_FUEL_AGAIN => {
                self.say(if r == NEED_FUEL_AGAIN {
                    GIVE_FUEL_AGAIN
                } else {
                    GIVE_FUEL
                });
                self.0.gauges(0, (5 * FUEL_TANK_SCALE) as i16, 0);
                if r != NEED_FUEL_AGAIN {
                    self.0.set(StoryKey::GIVEN_FUEL_BEFORE, 1);
                }
            }
            _ => return false,
        }
        true
    }

    pub(crate) fn offer_rescue(&mut self, handler: Handler, first_fuel: u16) {
        let supplies = self.0.supplies();
        if supplies.landers == 0 && self.0.get(StoryKey::CHMMR_BOMB_STATE) < 2 {
            self.offer(
                if self.0.get(StoryKey::LANDERS_LOST) != 0 {
                    I_LOST_ANOTHER_LANDER
                } else {
                    I_LOST_MY_LANDER
                },
                handler,
            );
        }
        if supplies.fuel < 2 * FUEL_TANK_SCALE {
            self.offer(
                if self.0.get(StoryKey::GIVEN_FUEL_BEFORE) != 0 {
                    NEED_FUEL_AGAIN
                } else {
                    first_fuel
                },
                handler,
            );
        }
    }

    pub(crate) fn no_radioactives(&mut self, r: u16) {
        if r == YES_THIS_IS_SUPPLY_SHIP {
            self.say(ABOUT_TIME);
            if self.0.supplies().radioactives != 0 {
                self.give_radios(0);
            } else {
                self.offer(I_LIED, Handler::NoRadioactives);
                self.offer(PLUMB_OUT, Handler::NoRadioactives);
            }
            return;
        }
        match r {
            WHERE_CAN_I_GET_RADIOS => {
                self.say(RADIOS_ON_MERCURY);
                self.0.disable(WHERE_CAN_I_GET_RADIOS);
            }
            NO_BUT_WELL_HELP0 => self.say(THE_WHAT_FROM_WHERE),
            WHAT_SLAVE_PLANET | I_LIED => self.say(DONT_KNOW_WHO_YOU_ARE),
            PLUMB_OUT => self.say(WHAT_KIND_OF_IDIOT),
            _ => {
                self.rescue(r);
            }
        }
        if self.0.supplies().radioactives != 0 {
            self.give_radios(0);
        } else {
            self.offer_rescue(Handler::NoRadioactives, NEED_FUEL_MERCURY);
            self.offer(OK_I_WILL_GET_RADIOS, Handler::ByeBye);
            if self.0.enabled(WHERE_CAN_I_GET_RADIOS) {
                self.offer(WHERE_CAN_I_GET_RADIOS, Handler::NoRadioactives);
            }
        }
    }

    pub(crate) fn ask_after_radios(&mut self, r: u16) {
        if !self.rescue(r) && r == WHERE_GET_RADIOS {
            self.say(RADIOS_ON_MERCURY);
            self.0.disable(WHERE_GET_RADIOS);
        }
        self.offer_rescue(Handler::AskAfterRadios, NEED_FUEL_MERCURY);
        self.offer(WELL_GO_GET_THEM_NOW, Handler::ByeBye);
        if self.0.enabled(WHERE_GET_RADIOS) {
            self.offer(WHERE_GET_RADIOS, Handler::AskAfterRadios);
        }
    }

    pub(crate) fn base_destroyed(&mut self, r: u16) {
        if r == WE_FOUGHT_THEM {
            self.say(YOU_REALLY_FOUGHT_BASE);
            self.offer(OH_YES_BIG_FIGHT, Handler::ByeBye);
            self.offer(I_LIED_IT_WAS_ABANDONED, Handler::ByeBye);
            self.offer(I_CANT_TALK_ABOUT_IT, Handler::ByeBye);
        } else {
            self.say(if r == WE_ARE_HERE_TO_HELP {
                BASE_ON_MOON
            } else {
                DEALT_WITH_BASE_YET
            });
            self.offer(BASE_WAS_ABANDONED, Handler::ByeBye);
            self.offer(WE_FOUGHT_THEM, Handler::BaseDestroyed);
        }
    }

    pub(crate) fn tell_moon_base(&mut self, r: u16) {
        if r == 0 {
            self.say(DEALT_WITH_BASE_YET);
        } else if !self.rescue(r) {
            if r == WE_ARE_HERE_TO_HELP {
                self.say(BASE_ON_MOON);
            } else if self.0.get(StoryKey::STARBASE_YACK_STACK1) == 0 {
                self.say(ABOUT_BASE);
                self.0.set(StoryKey::STARBASE_YACK_STACK1, 1);
            } else {
                self.say(ABOUT_BASE_AGAIN);
            }
        }
        self.offer_rescue(Handler::TellMoonBase, NEED_FUEL_LUNA);
        self.offer(
            if self.0.get(StoryKey::WILL_DESTROY_BASE) == 0 {
                WE_WILL_TAKE_CARE_OF_BASE
            } else {
                TAKE_CARE_OF_BASE_AGAIN
            },
            Handler::ByeBye,
        );
        self.offer(
            if self.0.get(StoryKey::STARBASE_YACK_STACK1) == 0 {
                TELL_ME_ABOUT_BASE
            } else {
                TELL_ME_AGAIN
            },
            Handler::TellMoonBase,
        );
    }

    pub(crate) fn tell_probe(&mut self) {
        self.say(THAT_WAS_PROBE);
        self.0.disable(WHAT_WAS_RED_THING);
        for phrase in [IT_WENT_AWAY, WE_DESTROYED_IT, WHAT_PROBE] {
            self.offer(phrase, Handler::RevealSelf);
        }
    }

    pub(crate) fn reveal_self(&mut self, r: u16) {
        let (reply, info_first) = match r {
            WE_ARE_VINDICATOR0 => (Some(THATS_IMPOSSIBLE), false),
            OUR_MISSION_WAS_SECRET => (Some(ACKNOWLEDGE_SECRET), false),
            FIRST_GIVE_INFO => (Some(ASK_AWAY), true),
            WHATS_THIS_STARBASE => (Some(STARBASE_IS), true),
            WHAT_ABOUT_EARTH => (Some(HAPPENED_TO_EARTH), true),
            WHERE_ARE_URQUAN => (Some(URQUAN_LEFT), true),
            IT_WENT_AWAY => (Some(DEEP_TROUBLE), false),
            WE_DESTROYED_IT => (Some(GOOD_NEWS), false),
            WHAT_PROBE => (Some(SURE_HOPE), false),
            _ => (None, false),
        };
        if let Some(reply) = reply {
            self.say(reply);
            if matches!(
                r,
                WE_ARE_VINDICATOR0
                    | OUR_MISSION_WAS_SECRET
                    | FIRST_GIVE_INFO
                    | WHATS_THIS_STARBASE
                    | WHAT_ABOUT_EARTH
                    | WHERE_ARE_URQUAN
            ) {
                self.0.disable(r);
            }
        }
        for info in [info_first, !info_first] {
            if info {
                for phrase in [
                    FIRST_GIVE_INFO,
                    WHATS_THIS_STARBASE,
                    WHAT_ABOUT_EARTH,
                    WHERE_ARE_URQUAN,
                    WHAT_WAS_RED_THING,
                ] {
                    if self.0.enabled(phrase) {
                        self.offer(
                            phrase,
                            if phrase == WHAT_WAS_RED_THING {
                                Handler::TellProbe
                            } else {
                                Handler::RevealSelf
                            },
                        );
                        break;
                    }
                }
            } else if self.0.enabled(WE_ARE_VINDICATOR0) {
                self.0.response(
                    WE_ARE_VINDICATOR0,
                    Handler::RevealSelf,
                    &[
                        i32::from(WE_ARE_VINDICATOR0),
                        GLOBAL_PLAYER_NAME,
                        i32::from(WE_ARE_VINDICATOR1),
                        GLOBAL_SHIP_NAME,
                        i32::from(WE_ARE_VINDICATOR2),
                    ],
                );
            } else if self.0.enabled(OUR_MISSION_WAS_SECRET) {
                self.offer(OUR_MISSION_WAS_SECRET, Handler::RevealSelf);
            } else {
                self.offer(
                    WE_ARE_HERE_TO_HELP,
                    if self.0.get(StoryKey::MOONBASE_DESTROYED) == 0 {
                        Handler::TellMoonBase
                    } else {
                        Handler::BaseDestroyed
                    },
                );
            }
        }
    }

    pub(crate) fn give_radios(&mut self, r: u16) {
        if r == WE_WILL_TRANSFER_NOW {
            self.0.set(StoryKey::RADIOACTIVES_PROVIDED, 1);
            self.say(FUEL_UP0);
            self.say(FUEL_UP1);
            self.0.talk_segue(1);
            self.0.disable_animation(2);
            self.0.fade_palette(0, (ONE_SECOND / 2) as i16);
            self.0.talk_segue(u16::MAX);
            self.reveal_self(0);
        } else {
            match r {
                WHAT_WILL_YOU_GIVE_US => self.say(MESSAGE_GARBLED_1),
                BEFORE_RADIOS_WE_NEED_INFO => self.say(MESSAGE_GARBLED_2),
                _ => {}
            }
            for phrase in [
                WE_WILL_TRANSFER_NOW,
                WHAT_WILL_YOU_GIVE_US,
                BEFORE_RADIOS_WE_NEED_INFO,
            ] {
                self.offer(phrase, Handler::GiveRadios);
            }
        }
    }

    pub(crate) fn intro(&mut self) {
        if self.0.get(StoryKey::PROBE_ILWRATH_ENCOUNTER) != 0 {
            self.say(VERY_IMPRESSIVE);
            for phrase in [
                COOK_THEIR_BUTTS,
                OVERTHROW_EVIL_ALIENS,
                ANNIHILATE_THOSE_MONSTERS,
            ] {
                self.offer(phrase, Handler::ByeBye);
            }
        } else if self.0.get(StoryKey::STARBASE_VISITED) != 0 {
            if self.0.get(StoryKey::RADIOACTIVES_PROVIDED) != 0 {
                if self.0.get(StoryKey::MOONBASE_DESTROYED) == 0 {
                    self.tell_moon_base(0);
                } else {
                    self.base_destroyed(0);
                }
            } else {
                self.0.palette(1);
                self.say(DO_YOU_HAVE_RADIO_THIS_TIME);
                if self.0.supplies().radioactives != 0 {
                    self.give_radios(0);
                } else {
                    self.ask_after_radios(0);
                }
            }
        } else {
            self.0.palette(1);
            self.0.set(StoryKey::STARBASE_VISITED, 1);
            self.say(ARE_YOU_SUPPLY_SHIP);
            self.0.response(
                NO_BUT_WELL_HELP0,
                Handler::NoRadioactives,
                &[
                    i32::from(NO_BUT_WELL_HELP0),
                    GLOBAL_SHIP_NAME,
                    i32::from(NO_BUT_WELL_HELP1),
                ],
            );
            self.offer(YES_THIS_IS_SUPPLY_SHIP, Handler::NoRadioactives);
            self.offer(WHAT_SLAVE_PLANET, Handler::NoRadioactives);
        }
    }
}

pub(crate) fn configure(desc: &mut CommDescriptor, powered: bool) {
    if powered {
        desc.alien_ambient_array[2].anim_flags |= ANIM_DISABLED;
        desc.alien_song_flags &= !1;
    } else {
        desc.alien_ambient_array[2].anim_flags &= !ANIM_DISABLED;
        desc.alien_alt_song_res = Some(c"comm.commander.lowpower.music");
        desc.alien_song_flags |= 1;
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
