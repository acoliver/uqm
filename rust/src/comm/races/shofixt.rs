//! Shofixti dialogue, ported from shofixt.c.
pub(crate) use super::script::{GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME};
pub(crate) use crate::comm::races::descriptor::Point;
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
pub(crate) mod api;
pub(crate) use api::ShofixtApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    Exit,
    GiveMaidens,
    Console,
    Explain,
    Realize,
    Friendly,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
impl<A: ShofixtApi> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::Exit => self.exit(r),
            Handler::GiveMaidens => self.give_maidens(r),
            Handler::Console => self.console(r),
            Handler::Explain => self.explain(r),
            Handler::Realize => self.realize(),
            Handler::Friendly => self.friendly(r),
        }
    }
    pub(crate) fn exit(&mut self, r: i32) {
        self.api.segue(1);
        if r == BYE0 {
            self.api.npc(GOODBYE);
            self.api.segue(0)
        } else if r == GO_AHEAD {
            self.api.npc(ON_SECOND_THOUGHT);
            self.api.segue(0)
        } else if r == NEED_YOU_FOR_DUTY {
            self.api.npc(OK_WILL_BE_SENTRY);
            self.api.segue(0)
        } else if [FEMALES, NUBILES, RAT_BABES].contains(&r) {
            self.api.npc(LEAPING_HAPPINESS);
            self.api.set(StoryKey::SHOFIXTI_RECRUITED, 1);
            self.api.set(StoryKey::MAIDENS_ON_SHIP, 0);
            self.api.segue(0);
            self.api.schedule_return()
        } else if r == DONT_ATTACK {
            self.api.npc(TYPICAL_PLOY);
            self.api.set(StoryKey::SHOFIXTI_STACK1, 1)
        } else if r == HEY_STOP {
            self.api.npc(ONLY_STOP);
            self.api.set(StoryKey::SHOFIXTI_STACK1, 2)
        } else if r == LOOK_YOU_ARE {
            self.api.npc(TOO_BAD);
            self.api.set(StoryKey::SHOFIXTI_STACK1, 3)
        } else if r == NO_ONE_INSULTS {
            self.api.npc(YOU_LIMP);
            self.api.set(StoryKey::SHOFIXTI_STACK2, 1)
        } else if r == MIGHTY_WORDS {
            self.api.npc(HANG_YOUR);
            self.api.set(StoryKey::SHOFIXTI_STACK2, 2)
        } else if r == DONT_KNOW {
            self.api.npc(NEVER);
            self.api.set(StoryKey::SHOFIXTI_STACK3, 1)
        } else if r == LOOK0 {
            self.api.npc(FOR_YOU);
            self.api.set(StoryKey::SHOFIXTI_STACK3, 2)
        } else if r == NO_BLOODSHED {
            self.api.npc(YES_BLOODSHED);
            self.api.set(StoryKey::SHOFIXTI_STACK3, 3)
        } else if r == DONT_WANT_TO_FIGHT {
            let n = self.api.get(StoryKey::SHOFIXTI_STACK4);
            if let Some(&phrase) = [
                MUST_FIGHT_YOU_URQUAN_1,
                MUST_FIGHT_YOU_URQUAN_2,
                MUST_FIGHT_YOU_URQUAN_3,
                MUST_FIGHT_YOU_URQUAN_4,
            ]
            .get(usize::from(n))
            {
                self.api.npc(phrase);
            }
            self.api.set(
                StoryKey::SHOFIXTI_STACK4,
                if n == 3 { n } else { n.wrapping_add(1) },
            );
        }
    }
    pub(crate) fn give_maidens(&mut self, r: i32) {
        if r == IMPORTANT_DUTY {
            self.api.npc(WHAT_DUTY);
            for p in [PROCREATING_WILDLY, REPLENISHING_YOUR_SPECIES, HOPE_YOU_HAVE] {
                self.api.response(p as u16, Handler::GiveMaidens, &[])
            }
        } else {
            self.api.npc(SOUNDS_GREAT_BUT_HOW);
            for p in [FEMALES, NUBILES, RAT_BABES] {
                self.api.response(p as u16, Handler::Exit, &[])
            }
        }
    }
    pub(crate) fn console(&mut self, r: i32) {
        if r == DONT_DO_IT {
            self.api.npc(YES_I_DO_IT);
            self.api.disable(DONT_DO_IT as u16)
        } else {
            self.api.npc(VERY_SAD_KILL_SELF)
        }
        if self.api.get(StoryKey::MAIDENS_ON_SHIP) != 0 {
            self.api
                .response(IMPORTANT_DUTY as u16, Handler::GiveMaidens, &[])
        }
        if self.api.enabled(DONT_DO_IT as u16) {
            self.api.response(DONT_DO_IT as u16, Handler::Console, &[])
        }
        self.api
            .response(NEED_YOU_FOR_DUTY as u16, Handler::Exit, &[]);
        self.api.response(GO_AHEAD as u16, Handler::Exit, &[])
    }
    pub(crate) fn explain(&mut self, r: i32) {
        self.api.npc(if r == I_AM_NICE {
            MUST_UNDERSTAND
        } else if r == I_AM_GUY {
            NICE_BUT_WHAT_IS_DONKEY
        } else {
            SO_SORRY
        });
        self.api.npc(IS_DEFEAT_TRUE);
        for p in [YES_AND_NO, CLOBBERED, BUTT_BLASTED] {
            self.api.response(p as u16, Handler::Console, &[])
        }
    }
    pub(crate) fn realize(&mut self) {
        self.api.npc(DGRUNTI);
        self.api.set(StoryKey::SHOFIXTI_STACK1, 0);
        self.api.set(StoryKey::SHOFIXTI_STACK3, 0);
        self.api.set(StoryKey::SHOFIXTI_STACK2, 3);
        let mut fragments = vec![I_AM_CAPTAIN0, GLOBAL_PLAYER_NAME, I_AM_CAPTAIN1];
        fragments.extend(self.api.alliance_fragments(NAME_1 as u16));
        fragments.extend([I_AM_CAPTAIN2, GLOBAL_SHIP_NAME, I_AM_CAPTAIN3]);
        self.api
            .response(I_AM_CAPTAIN0 as u16, Handler::Explain, &fragments);
        for p in [I_AM_NICE, I_AM_GUY] {
            self.api.response(p as u16, Handler::Explain, &[])
        }
    }
    pub(crate) fn hostile(&mut self) {
        match self.api.get(StoryKey::SHOFIXTI_STACK1) {
            0 => self.api.response(DONT_ATTACK as u16, Handler::Exit, &[]),
            1 => self.api.response(HEY_STOP as u16, Handler::Exit, &[]),
            2 => self.api.response(LOOK_YOU_ARE as u16, Handler::Exit, &[]),
            _ => {}
        }
        match self.api.get(StoryKey::SHOFIXTI_STACK2) {
            0 => self.api.response(NO_ONE_INSULTS as u16, Handler::Exit, &[]),
            1 => self.api.response(MIGHTY_WORDS as u16, Handler::Exit, &[]),
            2 => self
                .api
                .response(DONKEY_BREATH as u16, Handler::Realize, &[]),
            _ => {}
        }
        match self.api.get(StoryKey::SHOFIXTI_STACK3) {
            0 => self.api.response(DONT_KNOW as u16, Handler::Exit, &[]),
            1 => self.api.response(
                LOOK0 as u16,
                Handler::Exit,
                &[LOOK0, self.api.shofixti_name(), LOOK1],
            ),
            2 => self.api.response(LOOK_YOU_ARE as u16, Handler::Exit, &[]),
            _ => {}
        }
        self.api
            .response(DONT_WANT_TO_FIGHT as u16, Handler::Exit, &[])
    }
    pub(crate) fn friendly(&mut self, r: i32) {
        let mut last = 0usize;
        let mut responses: [Option<(i32, Vec<i32>)>; 3] = [None, None, None];
        if r == REPORT0 {
            self.api.npc(NOTHING_NEW);
            self.api.disable(REPORT0 as u16)
        } else if r == WHY_HERE0 {
            self.api.npc(I_GUARD);
            last = 1;
            self.api.set(StoryKey::SHOFIXTI_STACK1, 1)
        } else if r == WHAT_HAPPENED {
            self.api.npc(MET_VUX);
            last = 1;
            self.api.set(StoryKey::SHOFIXTI_STACK1, 2)
        } else if r == GLORY_DEVICE {
            self.api.npc(SWITCH_BROKE);
            self.api.set(StoryKey::SHOFIXTI_STACK1, 3)
        } else if r == WHERE_WORLD {
            self.api.npc(BLEW_IT_UP);
            last = 2;
            self.api.set(StoryKey::SHOFIXTI_STACK3, 1)
        } else if r == HOW_SURVIVE {
            self.api.npc(NOT_HERE);
            self.api.set(StoryKey::SHOFIXTI_STACK3, 2)
        }
        if self.api.enabled(REPORT0 as u16) {
            responses[0] = Some((REPORT0, vec![REPORT0, self.api.shofixti_name(), REPORT1]))
        }
        responses[1] = match self.api.get(StoryKey::SHOFIXTI_STACK1) {
            0 => Some((
                WHY_HERE0,
                vec![WHY_HERE0, self.api.shofixti_name(), WHY_HERE1],
            )),
            1 => Some((WHAT_HAPPENED, vec![])),
            2 => Some((GLORY_DEVICE, vec![])),
            _ => None,
        };
        responses[2] = match self.api.get(StoryKey::SHOFIXTI_STACK3) {
            0 => Some((WHERE_WORLD, vec![])),
            1 => Some((HOW_SURVIVE, vec![])),
            _ => None,
        };
        for i in std::iter::once(last).chain((0..3).filter(|&i| i != last)) {
            if let Some((p, fr)) = &responses[i] {
                self.api.response(*p as u16, Handler::Friendly, fr)
            }
        }
        if self.api.get(StoryKey::MAIDENS_ON_SHIP) != 0 {
            self.api
                .response(IMPORTANT_DUTY as u16, Handler::GiveMaidens, &[])
        }
        self.api.response(
            BYE0 as u16,
            Handler::Exit,
            &[BYE0, self.api.shofixti_name(), BYE1],
        )
    }
    pub(crate) fn intro(&mut self) {
        if self.api.activity() as u8 == crate::comm::dispatch::WON_LAST_BATTLE as u8 {
            self.api.npc(OUT_TAKES);
            self.api.segue(0);
            return;
        }
        let name = self.api.shofixti_name();
        if self.api.get(StoryKey::SHOFIXTI_STACK2) > 2 {
            self.api.npc(FRIENDLY_HELLO);
            self.friendly(0)
        } else {
            let n = self.api.get(StoryKey::SHOFIXTI_VISITS);
            let greetings: &[i32] = if name == KATANA {
                &[
                    HOSTILE_KATANA_1,
                    HOSTILE_KATANA_2,
                    HOSTILE_KATANA_3,
                    HOSTILE_KATANA_4,
                ]
            } else {
                &[
                    HOSTILE_TANAKA_1,
                    HOSTILE_TANAKA_2,
                    HOSTILE_TANAKA_3,
                    HOSTILE_TANAKA_4,
                    HOSTILE_TANAKA_5,
                    HOSTILE_TANAKA_6,
                    HOSTILE_TANAKA_7,
                    HOSTILE_TANAKA_8,
                ]
            };
            if let Some(&phrase) = greetings.get(usize::from(n)) {
                self.api.npc(phrase);
            }
            self.api.set(
                StoryKey::SHOFIXTI_VISITS,
                if usize::from(n) + 1 == greetings.len() {
                    n
                } else {
                    n.wrapping_add(1)
                },
            );
            self.hostile()
        }
    }
    pub(crate) fn initialize(&mut self) {
        self.api.segue(0)
    }
}
pub(crate) fn text_layout(screen_width: i32) -> (Point, u16) {
    let text_width = screen_width - 80;
    (
        Point {
            x: (1 + (text_width >> 1)) as i16,
            y: 0,
        },
        text_width as u16,
    )
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
