//! Slyhome dialogue state machine — ported from C.
//!
//! @plan PLAN-20260724-MAINLOOP-AND-COMM.P13-15

use super::script::as_comm_data;
use crate::comm::types::CommData;

mod phrases;
use phrases::*;

mod api;
use api::SlyhomeApi;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    Exit,
    Home,
    Human,
    Alien,
    Probe,
    Function,
    Bug,
    Fix,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}

impl<A: SlyhomeApi> Dialogue<'_, A> {
    fn initialize(&mut self) {
        self.api.segue(0);
    }
    fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::Exit => self.exit(r),
            Handler::Home => self.home(r),
            Handler::Human => self.human(r),
            Handler::Alien => self.alien(r),
            Handler::Probe => self.probe(r),
            Handler::Function => self.function(r),
            Handler::Bug => self.bug(r),
            Handler::Fix => self.fix(r),
        }
    }
    fn offer(&mut self, p: u32, h: Handler) {
        self.api.response(p as u16, h, &[]);
    }
    fn home(&mut self, r: i32) {
        match r as u32 {
            WE_ARE_US0 => {
                self.api.npc(TERRIBLY_EXCITING as i32);
                self.api.set("SLYLANDRO_STACK1", 1);
                self.api.disable(WE_ARE_US0 as u16);
            }
            WHAT_OTHER_VISITORS => {
                self.api.npc(VISITORS as i32);
                self.api.set("PLAYER_KNOWS_PROBE", 1);
                self.api.set("SLYLANDRO_STACK1", 2);
            }
            ANY_OTHER_VISITORS => {
                self.api.npc(LONG_AGO as i32);
                self.api.set("SLYLANDRO_STACK1", 3);
            }
            WHAT_ABOUT_SENTIENT_MILIEU => {
                self.api.npc(MET_TAALO_THEY_ARE_FROM as i32);
                self.api.set("SLYLANDRO_STACK1", 4);
            }
            WHO_ELSE => {
                self.api.npc(PRECURSORS as i32);
                self.api.set("SLYLANDRO_STACK1", 5);
            }
            PRECURSORS_YOW => {
                self.api.npc(ABOUT_PRECURSORS as i32);
                self.api.set("SLYLANDRO_STACK1", 6);
            }
            MUST_KNOW_MORE => {
                self.api.npc(ALL_WE_KNOW as i32);
                self.api.set("SLYLANDRO_STACK1", 7);
            }
            WHO_ARE_YOU => {
                self.api.npc(WE_ARE_SLY as i32);
                self.api.set("SLYLANDRO_STACK2", 1);
            }
            WHERE_ARE_YOU => {
                self.api.npc(DOWN_HERE as i32);
                self.api.set("SLYLANDRO_STACK3", 1);
            }
            THATS_IMPOSSIBLE_1 => {
                self.api.npc(NO_ITS_NOT_1 as i32);
                self.api.set("SLYLANDRO_STACK3", 2);
            }
            THATS_IMPOSSIBLE_2 => {
                self.api.npc(NO_ITS_NOT_2 as i32);
                self.api.set("SLYLANDRO_STACK3", 3);
            }
            LIKE_MORE_ABOUT_YOU => return self.alien(r),
            ENOUGH_ABOUT_ME => self.api.npc(OK_ENOUGH_YOU as i32),
            ENOUGH_INFO => self.api.npc(OK_ENOUGH_INFO as i32),
            ENOUGH_PROBE => self.api.npc(OK_ENOUGH_PROBE as i32),
            MEGA_SELF_DESTRUCT => {
                self.api.npc(WHY_YES_THERE_IS as i32);
                self.api.set("SLYLANDRO_KNOW_BROKEN", 1);
                self.api.set("DESTRUCT_CODE_ON_SHIP", 1);
                let n = self.api.get("SLYLANDRO_MULTIPLIER").wrapping_add(1);
                self.api.set("SLYLANDRO_MULTIPLIER", n);
                self.api.event(1, 0, 0, 0, 17);
            }
            _ => {}
        }
        let first = [
            WE_ARE_US0,
            WHAT_OTHER_VISITORS,
            ANY_OTHER_VISITORS,
            WHAT_ABOUT_SENTIENT_MILIEU,
            WHO_ELSE,
            PRECURSORS_YOW,
            MUST_KNOW_MORE,
        ]
        .get(self.api.get("SLYLANDRO_STACK1") as usize)
        .copied()
        .unwrap_or(0);
        let second = match self.api.get("SLYLANDRO_STACK2") {
            0 => WHO_ARE_YOU,
            1 if self.api.enabled(LIKE_MORE_ABOUT_YOU as u16) => LIKE_MORE_ABOUT_YOU,
            _ => 0,
        };
        let third = [WHERE_ARE_YOU, THATS_IMPOSSIBLE_1, THATS_IMPOSSIBLE_2]
            .get(self.api.get("SLYLANDRO_STACK3") as usize)
            .copied()
            .unwrap_or(0);
        let last = match r as u32 {
            WHO_ARE_YOU => 1,
            WHERE_ARE_YOU | THATS_IMPOSSIBLE_1 | THATS_IMPOSSIBLE_2 => 2,
            _ => 0,
        };
        let choices = [first, second, third];
        for i in std::iter::once(last).chain((0..3).filter(|&i| i != last)) {
            let p = choices[i];
            if p == WE_ARE_US0 {
                self.api.response(
                    p as u16,
                    Handler::Home,
                    &[
                        WE_ARE_US0 as i32,
                        super::script::GLOBAL_PLAYER_NAME,
                        WE_ARE_US1 as i32,
                        super::script::GLOBAL_SHIP_NAME,
                        WE_ARE_US2 as i32,
                    ],
                );
            } else if p != 0 {
                self.offer(p, Handler::Home);
            }
        }
        if self.api.get("SLYLANDRO_STACK1") != 0 {
            match self.api.get("SLYLANDRO_STACK4") {
                0 => self.offer(HAPPY_TO_TELL_MORE, Handler::Human),
                1 => self.offer(WOULD_YOU_LIKE_TO_KNOW_MORE, Handler::Human),
                _ => {}
            }
        }
        if self.api.get("PLAYER_KNOWS_PROBE") != 0 && self.api.get("SLYLANDRO_KNOW_BROKEN") == 0 {
            match self.api.get("SLYLANDRO_STACK5") {
                0 => self.offer(WHAT_ARE_PROBES, Handler::Probe),
                1 if self.api.enabled(KNOW_MORE_PROBE as u16) => {
                    self.offer(KNOW_MORE_PROBE, Handler::Probe)
                }
                _ => {}
            }
        }
        self.offer(BYE, Handler::Exit);
    }
    fn human(&mut self, r: i32) {
        let (line, key, value) = match r as u32 {
            HAPPY_TO_TELL_MORE => (TELL_MORE, "SLYLANDRO_STACK4", 1),
            WOULD_YOU_LIKE_TO_KNOW_MORE => (YES_TELL_MORE, "", 0),
            WE_COME_FROM_EARTH => (OK_EARTH, "SLYLANDRO_KNOW_EARTH", 1),
            WE_EXPLORE => (OK_EXPLORE, "SLYLANDRO_KNOW_EXPLORE", 1),
            WE_FIGHT_URQUAN => (URQUAN_NICE_GUYS, "SLYLANDRO_KNOW_URQUAN", 1),
            NOT_SAME_URQUAN => (PERSONALITY_CHANGE, "SLYLANDRO_KNOW_URQUAN", 2),
            WE_GATHER => (MAYBE_INTERESTED, "SLYLANDRO_KNOW_GATHER", 1),
            _ => (0, "", 0),
        };
        if line != 0 {
            self.api.npc(line as i32);
        }
        if !key.is_empty() {
            self.api.set(key, value);
        }
        if self.api.get("SLYLANDRO_KNOW_URQUAN") == 1 {
            self.offer(NOT_SAME_URQUAN, Handler::Human);
        }
        let mut information_left = self.api.get("SLYLANDRO_KNOW_URQUAN") == 1;
        for (key, p) in [
            ("SLYLANDRO_KNOW_EARTH", WE_COME_FROM_EARTH),
            ("SLYLANDRO_KNOW_EXPLORE", WE_EXPLORE),
            ("SLYLANDRO_KNOW_URQUAN", WE_FIGHT_URQUAN),
            ("SLYLANDRO_KNOW_GATHER", WE_GATHER),
        ] {
            if self.api.get(key) == 0 {
                self.offer(p, Handler::Human);
                information_left = true;
            }
        }
        self.offer(ENOUGH_ABOUT_ME, Handler::Home);
        if !information_left {
            self.api.set("SLYLANDRO_STACK4", 2);
        }
    }
    fn alien(&mut self, r: i32) {
        let (line, disable) = match r as u32 {
            LIKE_MORE_ABOUT_YOU => (SURE_KNOW_WHAT, false),
            WHAT_ABOUT_HOME => (ABOUT_HOME, true),
            WHAT_ABOUT_CULTURE => (ABOUT_CULTURE, true),
            WHAT_ABOUT_HISTORY => (ABOUT_HISTORY, true),
            WHAT_ABOUT_BIOLOGY => (ABOUT_BIOLOGY, true),
            _ => (0, false),
        };
        if line != 0 {
            self.api.npc(line as i32);
        }
        if disable {
            self.api.disable(r as u16);
        }
        let mut information_left = false;
        for p in [
            WHAT_ABOUT_HOME,
            WHAT_ABOUT_CULTURE,
            WHAT_ABOUT_HISTORY,
            WHAT_ABOUT_BIOLOGY,
        ] {
            if self.api.enabled(p as u16) {
                information_left = true;
                self.offer(p, Handler::Alien);
            }
        }
        self.offer(ENOUGH_INFO, Handler::Home);
        if !information_left {
            self.api.disable(LIKE_MORE_ABOUT_YOU as u16);
        }
    }
    fn probe(&mut self, r: i32) {
        if r as u32 == TALK_MORE_PROBE_ATTACK {
            self.function(r);
            return;
        }
        let line = match r as u32 {
            WHAT_ARE_PROBES => {
                self.api.npc(PROBES_ARE as i32);
                self.api.set("SLYLANDRO_STACK5", 1);
                0
            }
            KNOW_MORE_PROBE => OK_WHAT_MORE_PROBE,
            WHY_PROBE_ALWAYS_ATTACK => {
                self.api.npc(ONLY_DEFEND as i32);
                self.api.set("SLYLANDRO_STACK6", 1);
                0
            }
            WHERE_PROBES_FROM => {
                self.api.npc(PROBES_FROM_MELNORME as i32);
                self.api.set("SLYLANDRO_STACK7", 1);
                0
            }
            WHY_SELL => {
                self.api.npc(SELL_FOR_INFO as i32);
                self.api.set("SLYLANDRO_STACK7", 2);
                0
            }
            HOW_LONG_AGO => {
                self.api.npc(FIFTY_THOUSAND_ROTATIONS as i32);
                self.api.set("SLYLANDRO_STACK7", 3);
                0
            }
            WHATS_PROBES_MISSION => {
                self.api.npc(SEEK_OUT_NEW_LIFE as i32);
                self.api.set("SLYLANDRO_STACK8", 1);
                0
            }
            IF_ONLY_ONE => {
                self.api.npc(THEY_REPLICATE as i32);
                self.api.set("SLYLANDRO_STACK8", 2);
                0
            }
            ENOUGH_PROBLEM => OK_ENOUGH_PROBLEM,
            _ => 0,
        };
        if line != 0 {
            self.api.npc(line as i32);
        }
        let first = if self.api.get("SLYLANDRO_KNOW_BROKEN") == 0
            && self.api.get("PROBE_EXHIBITED_BUG") != 0
        {
            match self.api.get("SLYLANDRO_STACK6") {
                0 => WHY_PROBE_ALWAYS_ATTACK,
                1 => TALK_MORE_PROBE_ATTACK,
                _ => 0,
            }
        } else {
            0
        };
        let second = [WHERE_PROBES_FROM, WHY_SELL, HOW_LONG_AGO]
            .get(self.api.get("SLYLANDRO_STACK7") as usize)
            .copied()
            .unwrap_or(0);
        let third = [WHATS_PROBES_MISSION, IF_ONLY_ONE]
            .get(self.api.get("SLYLANDRO_STACK8") as usize)
            .copied()
            .unwrap_or(0);
        // HOW_LONG_AGO and IF_ONLY_ONE leave C's LastStack at zero.
        let last = match r as u32 {
            WHERE_PROBES_FROM | WHY_SELL => 1,
            WHATS_PROBES_MISSION => 2,
            _ => 0,
        };
        let choices = [first, second, third];
        let mut information_left = false;
        for i in std::iter::once(last).chain((0..3).filter(|&i| i != last)) {
            if choices[i] != 0 {
                information_left = true;
                self.offer(choices[i], Handler::Probe);
            }
        }
        self.offer(ENOUGH_PROBE, Handler::Home);
        if !information_left {
            self.api.disable(KNOW_MORE_PROBE as u16);
        }
    }
    fn function(&mut self, r: i32) {
        let line = match r as u32 {
            TALK_MORE_PROBE_ATTACK => NO_PROBLEM_BUT_SURE,
            TELL_ME_ABOUT_BASICS => {
                self.api.npc(BASIC_COMMANDS as i32);
                self.api.set("PLAYER_KNOWS_PROGRAM", 1);
                self.api.disable(TELL_BASICS_AGAIN as u16);
                0
            }
            TELL_BASICS_AGAIN => {
                self.api.npc(OK_BASICS_AGAIN as i32);
                self.api.disable(TELL_BASICS_AGAIN as u16);
                0
            }
            WHAT_EFFECT => {
                self.api.npc(AFFECTS_BEHAVIOR as i32);
                self.api.set("PLAYER_KNOWS_EFFECTS", 1);
                self.api.disable(WHAT_EFFECT as u16);
                0
            }
            TELL_ME_ABOUT_REP_1 => {
                self.api.npc(ABOUT_REP as i32);
                self.api.set("SLYLANDRO_STACK8", 3);
                0
            }
            WHAT_SET_PRIORITY => {
                self.api.npc(MAXIMUM as i32);
                self.api.set("PLAYER_KNOWS_PRIORITY", 1);
                self.api.disable(WHAT_SET_PRIORITY as u16);
                0
            }
            HOW_DOES_PROBE_DEFEND => {
                self.api.npc(ONLY_SELF_DEFENSE as i32);
                self.api.set("SLYLANDRO_STACK9", 1);
                0
            }
            COMBAT_BEHAVIOR => {
                self.api.npc(MISSILE_BATTERIES as i32);
                self.api.set("SLYLANDRO_STACK9", 2);
                0
            }
            WHAT_MISSILE_BATTERIES => {
                self.api.npc(LIGHTNING_ONLY_FOR_HARVESTING as i32);
                self.api.set("SLYLANDRO_STACK9", 3);
                0
            }
            _ => 0,
        };
        if line != 0 {
            self.api.npc(line as i32);
        }
        let choices = [
            [
                HOW_DOES_PROBE_DEFEND,
                COMBAT_BEHAVIOR,
                WHAT_MISSILE_BATTERIES,
            ]
            .get(self.api.get("SLYLANDRO_STACK9") as usize)
            .copied()
            .unwrap_or(0),
            match self.api.get("SLYLANDRO_STACK8") {
                2 => TELL_ME_ABOUT_REP_1,
                3 if self.api.enabled(WHAT_SET_PRIORITY as u16) => WHAT_SET_PRIORITY,
                _ => 0,
            },
        ];
        let last = match r as u32 {
            TELL_ME_ABOUT_REP_1 => Some(1),
            HOW_DOES_PROBE_DEFEND | COMBAT_BEHAVIOR => Some(0),
            _ => None,
        };
        if let Some(i) = last {
            if choices[i] != 0 {
                self.offer(choices[i], Handler::Function);
            }
        }
        if self.api.get("PLAYER_KNOWS_PROGRAM") == 0 {
            self.offer(TELL_ME_ABOUT_BASICS, Handler::Function)
        } else {
            if self.api.get("PLAYER_KNOWS_PRIORITY") != 0 {
                if self.api.get("PLAYER_KNOWS_EFFECTS") != 0 {
                    self.offer(PROBE_HAS_BUG, Handler::Bug)
                }
                if self.api.enabled(WHAT_EFFECT as u16) {
                    self.offer(WHAT_EFFECT, Handler::Function)
                }
            }
            if self.api.enabled(TELL_BASICS_AGAIN as u16) {
                self.offer(TELL_BASICS_AGAIN, Handler::Function)
            }
        }
        for (i, p) in choices.into_iter().enumerate() {
            if Some(i) != last && p != 0 {
                self.offer(p, Handler::Function);
            }
        }
        self.offer(ENOUGH_PROBLEM, Handler::Probe);
    }
    fn bug(&mut self, r: i32) {
        let line = match r as u32 {
            PROBE_HAS_BUG => NO_IT_DOESNT,
            TELL_ME_ABOUT_REP_2 => REP_NO_PROBLEM,
            WHAT_ABOUT_REP_PRIORITIES => MAXIMUM_SO_WHAT,
            TELL_ME_ABOUT_ATTACK => ATTACK_NO_PROBLEM,
            _ => 0,
        };
        if line != 0 {
            self.api.npc(line as i32);
            if r as u32 != PROBE_HAS_BUG {
                self.api.disable(r as u16);
            }
        }
        if self.api.enabled(TELL_ME_ABOUT_REP_2 as u16) {
            self.offer(TELL_ME_ABOUT_REP_2, Handler::Bug);
        } else if self.api.enabled(WHAT_ABOUT_REP_PRIORITIES as u16) {
            self.offer(WHAT_ABOUT_REP_PRIORITIES, Handler::Bug);
        } else {
            self.offer(THINK_ABOUT_REP_PRIORITIES, Handler::Fix);
        }
        if self.api.enabled(TELL_ME_ABOUT_ATTACK as u16) {
            self.offer(TELL_ME_ABOUT_ATTACK, Handler::Bug);
        }
    }
    fn fix(&mut self, r: i32) {
        let (line, p) = match r as u32 {
            THINK_ABOUT_REP_PRIORITIES => (UH_OH, 0),
            HUNT_THEM_DOWN => (GROW_TOO_FAST, HUNT_THEM_DOWN),
            SUE_MELNORME => (SIGNED_WAIVER, SUE_MELNORME),
            RECALL_SIGNAL => (NOT_THIS_MODEL, RECALL_SIGNAL),
            _ => (0, 0),
        };
        if line != 0 {
            self.api.npc(line as i32);
        }
        if p != 0 {
            self.api.disable(p as u16);
        }
        for p in [HUNT_THEM_DOWN, SUE_MELNORME, RECALL_SIGNAL] {
            if self.api.enabled(p as u16) {
                self.offer(p, Handler::Fix);
            }
        }
        self.offer(MEGA_SELF_DESTRUCT, Handler::Home);
    }
    fn exit(&mut self, _r: i32) {
        self.api.segue(0);
        self.api.npc(if self.api.get("SLYLANDRO_HOME_VISITS") == 1 {
            GOODBYE_1 as i32
        } else {
            GOODBYE_2 as i32
        });
    }
    fn intro(&mut self) {
        if self.api.get("SLYLANDRO_KNOW_BROKEN") != 0 && self.api.get("RECALL_VISITS") == 0 {
            self.api.npc(RECALL_PROGRAM_1 as i32);
            self.api.set("RECALL_VISITS", 1);
        } else {
            let n = self.api.get("SLYLANDRO_HOME_VISITS");
            if let Some(&p) = [HELLO_1, HELLO_2, HELLO_3, HELLO_4].get(n as usize) {
                self.api.npc(p as i32);
            }
            self.api.set(
                "SLYLANDRO_HOME_VISITS",
                if n == 3 { n } else { n.wrapping_add(1) },
            );
        }
        self.home(0);
    }
}

// ---------------------------------------------------------------------------
// Resource keys (from resinst.h)
// ---------------------------------------------------------------------------

const RACE_PMAP_ANIM: &[u8] = b"comm.slylandro.graphics\0";
const RACE_FONT: &[u8] = b"comm.slylandro.font\0";
const RACE_COLOR_MAP: &[u8] = b"comm.slylandro.colortable\0";
const RACE_MUSIC: &[u8] = b"comm.slylandro.music\0";
const RACE_CONVERSATION_PHRASES: &[u8] = b"comm.slylandro.dialogue\0";

mod descriptor;
use descriptor::descriptor;

/// Slyhome race dialogue implementation.
#[cfg(not(test))]
mod live {
    use super::super::script::live::LiveScript;
    use super::*;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    fn exit(r: u32) {
        run(|s| s.handle(Handler::Exit, r as i32));
    }
    fn home(r: u32) {
        run(|s| s.handle(Handler::Home, r as i32));
    }
    fn human(r: u32) {
        run(|s| s.handle(Handler::Human, r as i32));
    }
    fn alien(r: u32) {
        run(|s| s.handle(Handler::Alien, r as i32));
    }
    fn probe(r: u32) {
        run(|s| s.handle(Handler::Probe, r as i32));
    }
    fn function(r: u32) {
        run(|s| s.handle(Handler::Function, r as i32));
    }
    fn bug(r: u32) {
        run(|s| s.handle(Handler::Bug, r as i32));
    }
    fn fix(r: u32) {
        run(|s| s.handle(Handler::Fix, r as i32));
    }
    fn callback(h: Handler) -> crate::comm::response::ResponseFunc {
        match h {
            Handler::Exit => exit,
            Handler::Home => home,
            Handler::Human => human,
            Handler::Alien => alien,
            Handler::Probe => probe,
            Handler::Function => function,
            Handler::Bug => bug,
            Handler::Fix => fix,
        }
    }
    pub(super) fn intro() {
        run(|s| s.intro());
    }
    pub(super) fn post() {}
    pub(super) fn uninit() -> u16 {
        0
    }
    #[no_mangle]
    pub unsafe extern "C" fn init_slylandro_comm() -> *mut crate::comm::locdata::CLocData {
        static mut DESC: Option<crate::comm::locdata::CLocData> = None;
        extern "C" {
            static ScreenWidth: i32;
        }
        let d = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(|| descriptor(ScreenWidth));
        d.alien_text_baseline.x = (1 + ((ScreenWidth - 80) >> 1)) as i16;
        d.alien_text_width = (ScreenWidth - 80) as u16;
        run(|s| s.initialize());
        d
    }
}

#[cfg(test)]
#[path = "slyhome/tests.rs"]
mod branch_tests;

pub struct SlyhomeDialogue;

impl super::RaceDialogue for SlyhomeDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_slylandro_comm()) }
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

    fn post_encounter(&self) {}

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
            crate::state::game_state_keys::bit_range("DESTRUCT_CODE_ON_SHIP").is_some(),
            "missing game state key: DESTRUCT_CODE_ON_SHIP"
        );
        assert!(
            crate::state::game_state_keys::bit_range("PLAYER_KNOWS_EFFECTS").is_some(),
            "missing game state key: PLAYER_KNOWS_EFFECTS"
        );
        assert!(
            crate::state::game_state_keys::bit_range("PLAYER_KNOWS_PRIORITY").is_some(),
            "missing game state key: PLAYER_KNOWS_PRIORITY"
        );
        assert!(
            crate::state::game_state_keys::bit_range("PLAYER_KNOWS_PROBE").is_some(),
            "missing game state key: PLAYER_KNOWS_PROBE"
        );
        assert!(
            crate::state::game_state_keys::bit_range("PLAYER_KNOWS_PROGRAM").is_some(),
            "missing game state key: PLAYER_KNOWS_PROGRAM"
        );
    }
}
