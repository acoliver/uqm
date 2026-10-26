//! Spahome dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

use super::script::as_comm_data;
use crate::comm::types::CommData;

mod phrases;
use phrases::*;

mod api;
use api::SpahomeApi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    Exit,
    Password,
    Council,
    Angry,
    Alliance,
    Learn,
    Quest,
    Party,
    Allies,
    Info,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: SpahomeApi> Dialogue<'_, A> {
    fn initialize(&mut self) {
        self.api.segue(if self.api.get("SPATHI_MANNER") == 3 {
            0
        } else {
            1
        });
    }
    fn post(&mut self) {
        if self.api.get_segue() == 1 {
            let manner = self.api.get("SPATHI_MANNER");
            if manner != 2 {
                self.api.set("SPATHI_MANNER", 1);
                if manner != 1 {
                    self.api.set("SPATHI_VISITS", 0);
                    self.api.set("SPATHI_HOME_VISITS", 0);
                }
            }
        }
    }
    fn offer(&mut self, p: u32, h: Handler) {
        self.api.response(p as u16, h, &[])
    }
    fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::Exit => self.exit(r),
            Handler::Password => self.password(r),
            Handler::Council => self.council(r),
            Handler::Angry => self.angry(r),
            Handler::Alliance => self.alliance(r),
            Handler::Learn => self.learn(r),
            Handler::Quest => self.quest(r),
            Handler::Party => self.party(r),
            Handler::Allies => self.allies(r),
            Handler::Info => self.info(r),
        }
    }
    fn exit(&mut self, r: i32) {
        self.api.segue(0);
        match r as u32 {
            WE_ATTACK_AGAIN => {
                self.api.npc(WE_FIGHT_AGAIN as i32);
                self.api.segue(1)
            }
            SURRENDER_OR_DIE => {
                self.api.npc(DEFEND_OURSELVES as i32);
                self.api.segue(1)
            }
            WE_ARE_VINDICATOR0 | GORT_MERENGA | GUPH_FLORP | WAGNGL_FTHAGN | PLEEESE
            | SCREW_PASSWORD => {
                self.api.npc(
                    if r as u32 == WE_ARE_VINDICATOR0 || r as u32 == SCREW_PASSWORD {
                        NO_PASSWORD as i32
                    } else {
                        WRONG_PASSWORD as i32
                    },
                );
                self.api.segue(1)
            }
            BYE_NO_ALLY_OFFER => self.api.npc(GOODBYE_NO_ALLY_OFFER as i32),
            BYE_ANGRY_SPATHI => self.api.npc(GOODBYE_ANGRY_SPATHI as i32),
            BYE_ALLY => self.api.npc(GOODBYE_ALLY as i32),
            ALREADY_GOT_THEM => {
                self.api.npc(EARLY_BIRD_CHECK as i32);
                self.api.set("SPATHI_HOME_VISITS", 0);
                self.api.set("SPATHI_VISITS", 0);
                self.api.set("SPATHI_PARTY", 1);
                self.api.set("SPATHI_MANNER", 3)
            }
            TOO_DANGEROUS => self.api.npc(WE_AGREE as i32),
            THINK_MORE => self.api.npc(COWARD as i32),
            I_ACCEPT => {
                self.api.npc(AWAIT_RETURN as i32);
                self.api.set("SPATHI_QUEST", 1);
                self.api.set("SPATHI_MANNER", 3);
                self.api.set("SPATHI_VISITS", 0)
            }
            DO_AS_WE_SAY => {
                self.api.npc(DEPART_FOR_EARTH as i32);
                self.api
                    .set_allied(crate::comm::dispatch::ship::SPATHI_SHIP as u16, true);
                self.api.event(1, 6, 0, 0, 11);
                self.api.set("SPATHI_HOME_VISITS", 0);
                self.api.set("SPATHI_VISITS", 0)
            }
            KILLED_THEM_ALL_1 | KILLED_THEM_ALL_2 => {
                self.api.npc(if r as u32 == KILLED_THEM_ALL_1 {
                    WILL_CHECK_1 as i32
                } else {
                    WILL_CHECK_2 as i32
                });
                if self.api.get("SPATHI_CREATURES_ELIMINATED") == 0 {
                    self.api.set(
                        "LIED_ABOUT_CREATURES",
                        if r as u32 == KILLED_THEM_ALL_1 { 1 } else { 2 },
                    );
                } else {
                    self.party_joined();
                }
            }
            BYE_BEFORE_PARTY => self.api.npc(GOODBYE_BEFORE_PARTY as i32),
            BYE_FROM_PARTY_1 | BYE_FROM_PARTY_2 | BYE_FROM_PARTY_3 => {
                self.api.npc(GOODBYE_FROM_PARTY as i32)
            }
            _ => {}
        }
    }
    fn party_joined(&mut self) {
        self.api.set("SPATHI_HOME_VISITS", 0);
        self.api.set("SPATHI_VISITS", 0);
        self.api.set("SPATHI_PARTY", 1);
        self.api.set("SPATHI_MANNER", 3);
    }
    fn password(&mut self, r: i32) {
        if r == 0 {
            let n = self.api.get("SPATHI_HOME_VISITS");
            if n == 1 {
                self.api.npc(WHAT_IS_PASSWORD_AGAIN as i32);
                self.api.set("SPATHI_HOME_VISITS", 1)
            } else {
                self.api.npc(WHAT_IS_PASSWORD as i32);
                self.api.set("SPATHI_HOME_VISITS", 1)
            }
        } else if r as u32 == WHAT_DO_I_GET {
            self.api.npc(YOU_GET_TO_LIVE as i32);
            self.api.disable(WHAT_DO_I_GET as u16);
        }
        if self.api.get("FOUND_PLUTO_SPATHI") != 0 || self.api.get("KNOW_SPATHI_PASSWORD") != 0 {
            self.offer(GOOD_PASSWORD, Handler::Council);
            if self.api.enabled(WHAT_DO_I_GET as u16) {
                self.offer(WHAT_DO_I_GET, Handler::Password)
            }
        } else {
            self.api.response(
                WE_ARE_VINDICATOR0 as u16,
                Handler::Exit,
                &[
                    WE_ARE_VINDICATOR0 as i32,
                    super::script::GLOBAL_PLAYER_NAME,
                    WE_ARE_VINDICATOR1 as i32,
                    super::script::GLOBAL_SHIP_NAME,
                    WE_ARE_VINDICATOR2 as i32,
                ],
            );
            for p in [
                GORT_MERENGA,
                GUPH_FLORP,
                WAGNGL_FTHAGN,
                PLEEESE,
                SCREW_PASSWORD,
            ] {
                self.offer(p, Handler::Exit)
            }
        }
    }
    fn angry(&mut self, r: i32) {
        if r == 0 {
            self.api.npc(MEAN_GUYS_RETURN as i32);
            self.offer(WE_APOLOGIZE, Handler::Angry)
        } else {
            self.api.npc(DONT_BELIEVE as i32);
            self.offer(MISUNDERSTANDING, Handler::Alliance)
        }
        self.offer(WE_ATTACK_AGAIN, Handler::Exit);
        self.offer(BYE_ANGRY_SPATHI, Handler::Exit)
    }
    fn alliance(&mut self, r: i32) {
        let (line, disable) = match r as u32 {
            MISUNDERSTANDING => {
                self.api.npc(JUST_MISUNDERSTANDING as i32);
                self.api.fade_palette(1, 210);
                self.api.set("SPATHI_MANNER", 3);
                self.api.set("SPATHI_VISITS", 0);
                (0, false)
            }
            WE_COME_IN_PEACE => (OF_COURSE, false),
            HAND_IN_FRIENDSHIP => (TOO_AFRAID, true),
            STRONGER => (YOURE_NOT, true),
            YES_WE_ARE => (NO_YOURE_NOT, true),
            SHARE_INFO => (NO_INFO, true),
            GIVE_US_RESOURCES => (NO_RESOURCES, true),
            _ => (0, false),
        };
        if line != 0 {
            self.api.npc(line as i32)
        }
        if disable {
            self.api.disable(r as u16)
        }
        for p in [HAND_IN_FRIENDSHIP, STRONGER, YES_WE_ARE] {
            if self.api.enabled(p as u16) {
                self.offer(p, Handler::Alliance);
                break;
            }
        }
        if !self.api.enabled(HAND_IN_FRIENDSHIP as u16)
            && !self.api.enabled(STRONGER as u16)
            && !self.api.enabled(YES_WE_ARE as u16)
        {
            self.offer(HOW_PROVE, Handler::Learn)
        }
        for p in [SHARE_INFO, GIVE_US_RESOURCES] {
            if self.api.enabled(p as u16) {
                self.offer(p, Handler::Alliance)
            }
        }
    }
    fn learn(&mut self, r: i32) {
        let line = match r as u32 {
            0 => {
                self.api.npc(QUEST_AGAIN as i32);
                self.api.disable(WHAT_TEST as u16);
                if self.api.get("KNOW_SPATHI_EVIL") != 0 {
                    self.api.disable(TELL_EVIL as u16);
                }
                0
            }
            HOW_PROVE => BETTER_IDEA,
            WHAT_TEST => {
                self.api.npc(WIPE_EVIL as i32);
                self.api.set("KNOW_SPATHI_QUEST", 1);
                self.api.disable(WHAT_TEST as u16);
                0
            }
            TELL_EVIL => {
                self.api.npc(BEFORE_ACCEPT as i32);
                self.api.set("KNOW_SPATHI_EVIL", 1);
                self.api.disable(TELL_EVIL as u16);
                self.api.disable(PROVE_STRENGTH as u16);
                0
            }
            PROVE_STRENGTH => {
                self.api.npc(YOUR_BEHAVIOR as i32);
                self.api.disable(PROVE_STRENGTH as u16);
                0
            }
            WHY_DONT_YOU_DO_IT => {
                self.api.npc(WE_WONT_BECAUSE as i32);
                self.api.disable(WHY_DONT_YOU_DO_IT as u16);
                0
            }
            _ => 0,
        };
        if line != 0 {
            self.api.npc(line as i32)
        }
        if self.api.enabled(WHAT_TEST as u16) {
            self.offer(WHAT_TEST, Handler::Learn)
        } else if self.api.get("SPATHI_CREATURES_ELIMINATED") != 0 {
            self.offer(ALREADY_GOT_THEM, Handler::Exit)
        } else if self.api.enabled(TELL_EVIL as u16) {
            self.offer(TOO_DANGEROUS, Handler::Exit);
            self.offer(TELL_EVIL, Handler::Learn)
        } else {
            for p in [TOO_DANGEROUS, THINK_MORE, I_ACCEPT] {
                self.offer(p, Handler::Exit)
            }
        }
        if self.api.enabled(PROVE_STRENGTH as u16) && self.api.get("KNOW_SPATHI_QUEST") == 0 {
            self.offer(PROVE_STRENGTH, Handler::Learn)
        }
        if self.api.enabled(WHY_DONT_YOU_DO_IT as u16) {
            self.offer(WHY_DONT_YOU_DO_IT, Handler::Learn)
        }
    }
    fn quest(&mut self, r: i32) {
        if r == 0 {
            self.api.npc(if self.api.get("LIED_ABOUT_CREATURES") == 0 {
                HOW_GO_EFFORTS as i32
            } else {
                YOU_LIED_1 as i32
            })
        } else {
            let (line, p) = match r as u32 {
                LITTLE_MISTAKE => (BIG_MISTAKE, LITTLE_MISTAKE),
                TALK_TEST => (TEST_AGAIN, TALK_TEST),
                ZAPPED_A_FEW => (YOU_FORTUNATE, ZAPPED_A_FEW),
                _ => (0, 0),
            };
            if line != 0 {
                self.api.npc(line as i32);
                self.api.disable(p as u16)
            }
        }
        if self.api.get("LIED_ABOUT_CREATURES") == 0 {
            self.offer(KILLED_THEM_ALL_1, Handler::Exit)
        } else {
            if self.api.enabled(LITTLE_MISTAKE as u16) {
                self.offer(LITTLE_MISTAKE, Handler::Quest)
            }
            self.offer(KILLED_THEM_ALL_2, Handler::Exit)
        }
        if self.api.get("SPATHI_CREATURES_ELIMINATED") == 0 {
            if self.api.enabled(TALK_TEST as u16) {
                self.offer(TALK_TEST, Handler::Quest)
            }
            if self.api.enabled(ZAPPED_A_FEW as u16)
                && self.api.get("SPATHI_CREATURES_EXAMINED") != 0
            {
                self.offer(ZAPPED_A_FEW, Handler::Quest)
            }
            self.offer(BYE_BEFORE_PARTY, Handler::Exit)
        }
    }
    fn council(&mut self, r: i32) {
        let line = match r as u32 {
            0 => HELLO_AGAIN,
            GOOD_PASSWORD => {
                self.api.npc(YES_GOOD_PASSWORD as i32);
                self.api.fade_palette(1, 210);
                self.api.set("KNOW_SPATHI_PASSWORD", 1);
                self.api.set("SPATHI_HOME_VISITS", 0);
                0
            }
            WE_COME_IN_PEACE => {
                self.api.npc(KILLED_SPATHI as i32);
                self.api.disable(WE_COME_IN_PEACE as u16);
                0
            }
            SPATHI_ON_PLUTO => {
                self.api.npc(WHERE_SPATHI as i32);
                self.api.disable(SPATHI_ON_PLUTO as u16);
                0
            }
            HOSTAGE => {
                self.api.npc(GUN_TO_HEAD as i32);
                self.api.set("FOUND_PLUTO_SPATHI", 3);
                self.api.disable(HOSTAGE as u16);
                0
            }
            KILLED_FWIFFO => {
                self.api.npc(POOR_FWIFFO as i32);
                self.api.set("FOUND_PLUTO_SPATHI", 3);
                self.api.disable(KILLED_FWIFFO as u16);
                0
            }
            FWIFFO_FINE => {
                self.api.npc(NOT_LIKELY as i32);
                self.api.disable(FWIFFO_FINE as u16);
                0
            }
            SURRENDER => {
                self.api.npc(NO_SURRENDER as i32);
                self.api.disable(SURRENDER as u16);
                0
            }
            _ => 0,
        };
        if line != 0 {
            self.api.npc(line as i32)
        }
        if self.api.get("SPATHI_MANNER") == 0 {
            self.offer(WE_COME_IN_PEACE, Handler::Alliance)
        } else if self.api.enabled(WE_COME_IN_PEACE as u16) {
            self.offer(WE_COME_IN_PEACE, Handler::Council)
        } else {
            self.offer(MISUNDERSTANDING, Handler::Alliance)
        }
        let found = self.api.get("FOUND_PLUTO_SPATHI");
        if found != 0 && found < 3 {
            if self.api.enabled(SPATHI_ON_PLUTO as u16) {
                self.offer(SPATHI_ON_PLUTO, Handler::Council);
            } else if self.api.have_spathi_escort() {
                if self.api.enabled(HOSTAGE as u16) {
                    self.offer(HOSTAGE, Handler::Council);
                }
            } else if self.api.enabled(KILLED_FWIFFO as u16) {
                self.offer(KILLED_FWIFFO, Handler::Council);
                if self.api.enabled(FWIFFO_FINE as u16) {
                    self.offer(FWIFFO_FINE, Handler::Council);
                }
            }
        }
        if self.api.enabled(SURRENDER as u16) {
            self.offer(SURRENDER, Handler::Council)
        } else {
            self.offer(SURRENDER_OR_DIE, Handler::Exit)
        }
        self.offer(BYE_NO_ALLY_OFFER, Handler::Exit)
    }
    fn party(&mut self, r: i32) {
        if r == 0 {
            let n = self.api.get("SPATHI_HOME_VISITS");
            if let Some(&p) = [MUST_PARTY_1, MUST_PARTY_2, MUST_PARTY_3].get(usize::from(n)) {
                self.api.npc(p as i32);
            }
            self.api.set(
                "SPATHI_HOME_VISITS",
                if n == 2 { n } else { n.wrapping_add(1) },
            );
        } else {
            let (line, p) = match r as u32 {
                DEALS_A_DEAL => (WAIT_A_WHILE, DEALS_A_DEAL),
                HOW_LONG => (TEN_YEARS, HOW_LONG),
                RENEGING => (ADULT_VIEW, RENEGING),
                RETURN_BEASTS => (WHAT_RELATIONSHIP, RETURN_BEASTS),
                MINDS_AND_MIGHT => (HUH, MINDS_AND_MIGHT),
                FELLOWSHIP => (WHAT, FELLOWSHIP),
                _ => (0, 0),
            };
            if line != 0 {
                self.api.npc(line as i32);
                self.api.disable(p as u16)
            }
        }
        let first = [DEALS_A_DEAL, HOW_LONG, RENEGING, RETURN_BEASTS]
            .into_iter()
            .find(|&p| self.api.enabled(p as u16));
        if let Some(p) = first {
            self.offer(p, Handler::Party);
        } else {
            for p in [MINDS_AND_MIGHT, FELLOWSHIP] {
                if self.api.enabled(p as u16) {
                    self.offer(p, Handler::Party);
                }
            }
            self.offer(DO_AS_WE_SAY, Handler::Exit);
            return;
        }
        let goodbye = match self.api.get("SPATHI_HOME_VISITS") {
            1 => BYE_FROM_PARTY_1,
            2 => BYE_FROM_PARTY_2,
            _ => BYE_FROM_PARTY_3,
        };
        self.offer(goodbye, Handler::Exit);
    }
    fn allies(&mut self, r: i32) {
        if r == 0 {
            let n = self.api.get("SPATHI_HOME_VISITS");
            if let Some(&p) = [HELLO_ALLIES_1, HELLO_ALLIES_2, HELLO_ALLIES_3].get(usize::from(n)) {
                self.api.npc(p as i32);
            }
            self.api.set(
                "SPATHI_HOME_VISITS",
                if n == 2 { n } else { n.wrapping_add(1) },
            );
        } else if r as u32 == WHATS_UP {
            let n = self.api.get("SPATHI_INFO");
            if let Some(&p) = [
                GENERAL_INFO_1,
                GENERAL_INFO_2,
                GENERAL_INFO_3,
                GENERAL_INFO_4,
                GENERAL_INFO_5,
                GENERAL_INFO_5,
            ]
            .get(usize::from(n))
            {
                self.api.npc(p as i32);
            }
            if n == 2 {
                self.api.set("KNOW_URQUAN_STORY", 1);
                self.api.set("KNOW_KOHR_AH_STORY", 1)
            }
            self.api.set(
                "SPATHI_INFO",
                if n == 4 || n == 5 {
                    n
                } else {
                    n.wrapping_add(1)
                },
            );
            self.api.disable(WHATS_UP as u16)
        } else if r as u32 == RESOURCES_PLEASE {
            self.api.npc(SORRY_NO_RESOURCES as i32);
            self.api.disable(RESOURCES_PLEASE as u16)
        } else if r as u32 == SOMETHING_FISHY {
            self.api.npc(NOTHING_FISHY as i32);
            self.api.set("SPATHI_INFO", 5)
        } else if r as u32 == ENOUGH_INFO {
            self.api.npc(OK_ENOUGH_INFO as i32);
        }
        if self.api.get("SPATHI_INFO") == 4 {
            self.offer(SOMETHING_FISHY, Handler::Allies);
        }
        for p in [WHATS_UP, RESOURCES_PLEASE, LIKE_SOME_INFO] {
            if self.api.enabled(p as u16) {
                self.offer(
                    p,
                    if p == LIKE_SOME_INFO {
                        Handler::Info
                    } else {
                        Handler::Allies
                    },
                )
            }
        }
        self.offer(BYE_ALLY, Handler::Exit)
    }
    fn info(&mut self, r: i32) {
        let line = match r as u32 {
            LIKE_SOME_INFO => WHAT_ABOUT,
            WHAT_ABOUT_HIERARCHY => {
                self.api.npc(ABOUT_HIERARCHY as i32);
                self.api.disable(WHAT_ABOUT_HIERARCHY as u16);
                0
            }
            WHAT_ABOUT_HISTORY => {
                self.api.npc(ABOUT_HISTORY as i32);
                self.api.disable(WHAT_ABOUT_HISTORY as u16);
                0
            }
            WHAT_ABOUT_ALLIANCE => {
                self.api.npc(ABOUT_ALLIANCE as i32);
                self.api.disable(WHAT_ABOUT_ALLIANCE as u16);
                0
            }
            WHAT_ABOUT_OTHER => {
                self.api.npc(ABOUT_OTHER as i32);
                self.api.disable(WHAT_ABOUT_OTHER as u16);
                0
            }
            WHAT_ABOUT_PRECURSORS => {
                self.api.npc(ABOUT_PRECURSORS as i32);
                self.api.disable(WHAT_ABOUT_PRECURSORS as u16);
                0
            }
            _ => 0,
        };
        if line != 0 {
            self.api.npc(line as i32)
        }
        let mut information_left = false;
        for p in [
            WHAT_ABOUT_HIERARCHY,
            WHAT_ABOUT_HISTORY,
            WHAT_ABOUT_ALLIANCE,
            WHAT_ABOUT_OTHER,
            WHAT_ABOUT_PRECURSORS,
        ] {
            if self.api.enabled(p as u16) {
                information_left = true;
                self.offer(p, Handler::Info)
            }
        }
        self.offer(ENOUGH_INFO, Handler::Allies);
        if !information_left {
            self.api.disable(LIKE_SOME_INFO as u16);
        }
    }
    fn intro(&mut self) {
        if self.api.get("SPATHI_MANNER") == 2 {
            self.api.npc(HATE_YOU_FOREVER as i32);
            self.api.segue(1)
        } else if self.api.get("SPATHI_MANNER") == 1
            && self.api.get("KNOW_SPATHI_PASSWORD") != 0
            && (self.api.get("FOUND_PLUTO_SPATHI") != 0 || self.api.get("SPATHI_HOME_VISITS") != 7)
        {
            self.angry(0)
        } else if self
            .api
            .alliance(crate::comm::dispatch::ship::SPATHI_SHIP as u16)
            == 1
        {
            self.api.palette(1);
            self.allies(0)
        } else if self.api.get("SPATHI_PARTY") != 0 {
            self.api.palette(1);
            self.party(0)
        } else if self.api.get("SPATHI_QUEST") != 0 {
            if self.api.get("LIED_ABOUT_CREATURES") < 2 {
                self.api.palette(1);
                self.quest(0)
            } else {
                self.api.npc(YOU_LIED_2 as i32);
                self.api.set("SPATHI_MANNER", 2);
                self.api.segue(1)
            }
        } else if self.api.get("KNOW_SPATHI_QUEST") != 0 {
            self.api.palette(1);
            self.learn(0)
        } else if self.api.get("KNOW_SPATHI_PASSWORD") != 0
            && (self.api.get("FOUND_PLUTO_SPATHI") != 0 || self.api.get("SPATHI_HOME_VISITS") != 7)
        {
            self.api.palette(1);
            self.council(0)
        } else {
            self.password(0)
        }
    }
}

// ---------------------------------------------------------------------------
// Resource keys (from resinst.h)
// ---------------------------------------------------------------------------

const RACE_PMAP_ANIM: &[u8] = b"comm.safeones.graphics\0";
const RACE_FONT: &[u8] = b"comm.spathi.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.safeones.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.spathi.music\0";
const RACE_ALT_MUSIC: &[u8] = b"comm.safeones.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.safeones.dialogue\0";

mod descriptor;
use descriptor::descriptor;

/// Spahome race dialogue implementation.
#[cfg(not(test))]
pub(crate) mod live {
    use super::super::script::live::LiveScript;
    use super::*;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    fn exit(r: u32) {
        run(|s| s.handle(Handler::Exit, r as i32));
    }
    fn password(r: u32) {
        run(|s| s.handle(Handler::Password, r as i32));
    }
    fn council(r: u32) {
        run(|s| s.handle(Handler::Council, r as i32));
    }
    fn angry(r: u32) {
        run(|s| s.handle(Handler::Angry, r as i32));
    }
    fn alliance(r: u32) {
        run(|s| s.handle(Handler::Alliance, r as i32));
    }
    fn learn(r: u32) {
        run(|s| s.handle(Handler::Learn, r as i32));
    }
    fn quest(r: u32) {
        run(|s| s.handle(Handler::Quest, r as i32));
    }
    fn party(r: u32) {
        run(|s| s.handle(Handler::Party, r as i32));
    }
    fn allies(r: u32) {
        run(|s| s.handle(Handler::Allies, r as i32));
    }
    fn info(r: u32) {
        run(|s| s.handle(Handler::Info, r as i32));
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::Exit => exit,
            Handler::Password => password,
            Handler::Council => council,
            Handler::Angry => angry,
            Handler::Alliance => alliance,
            Handler::Learn => learn,
            Handler::Quest => quest,
            Handler::Party => party,
            Handler::Allies => allies,
            Handler::Info => info,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {
        run(|s| s.post());
    }
    pub(super) fn uninit() -> u16 {
        0
    }
    #[no_mangle]
    pub unsafe extern "C" fn init_spahome_comm() -> *mut crate::comm::locdata::CLocData {
        static mut DESC: Option<crate::comm::locdata::CLocData> = None;
        extern "C" {
            static mut ScreenWidth: i32;
        }
        let d = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(|| descriptor(ScreenWidth));
        d.alien_text_baseline.x = (1 + ((ScreenWidth - 80) >> 1)) as i16;
        d.alien_text_width = (ScreenWidth - 80 - 16) as u16;
        run(|s| s.initialize());
        d
    }
}
#[cfg(test)]
#[path = "spahome/tests.rs"]
mod branch_tests;

pub struct SpahomeDialogue;

impl super::RaceDialogue for SpahomeDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_spahome_comm()) }
        }
        #[cfg(test)]
        {
            as_comm_data(&descriptor(320))
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
mod tests {
    use super::*;

    #[test]
    fn test_resource_keys_are_null_terminated() {
        assert_eq!(RACE_PMAP_ANIM.last(), Some(&0));
        assert_eq!(RACE_FONT.last(), Some(&0));
        assert_eq!(RACE_COLOR_MAP.last(), Some(&0));
        assert_eq!(RACE_MUSIC.last(), Some(&0));
    }

    #[test]
    fn test_game_state_keys_exist() {
        assert!(
            crate::state::game_state_keys::bit_range("FOUND_PLUTO_SPATHI").is_some(),
            "missing game state key: FOUND_PLUTO_SPATHI"
        );
        assert!(
            crate::state::game_state_keys::bit_range("KNOW_KOHR_AH_STORY").is_some(),
            "missing game state key: KNOW_KOHR_AH_STORY"
        );
        assert!(
            crate::state::game_state_keys::bit_range("KNOW_SPATHI_EVIL").is_some(),
            "missing game state key: KNOW_SPATHI_EVIL"
        );
        assert!(
            crate::state::game_state_keys::bit_range("KNOW_SPATHI_PASSWORD").is_some(),
            "missing game state key: KNOW_SPATHI_PASSWORD"
        );
        assert!(
            crate::state::game_state_keys::bit_range("KNOW_SPATHI_QUEST").is_some(),
            "missing game state key: KNOW_SPATHI_QUEST"
        );
    }
}
