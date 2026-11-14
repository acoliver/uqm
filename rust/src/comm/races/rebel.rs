//! Yehat Rebel dialogue, ported from rebel.c.
pub(crate) use crate::comm::races::descriptor::Point;
use crate::engine::world::activity::ActivityKind;
pub(crate) use crate::engine::StoryKey;
mod phrases;
use phrases::*;
pub(crate) mod api;
pub(crate) use api::RebelApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Handler {
    Exit,
    Info,
    Rebels,
}
pub(crate) struct Dialogue<'a, A> {
    pub(crate) api: &'a mut A,
}
impl<A: RebelApi> Dialogue<'_, A> {
    pub(crate) fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::Exit => {
                if r == BYE_REBEL {
                    self.api.npc(GOODBYE_REBEL)
                }
            }
            Handler::Info => self.info(r),
            Handler::Rebels => self.rebels(r),
        }
    }
    pub(crate) fn info(&mut self, r: i32) {
        if r == GIVE_INFO_REBELS {
            self.api.npc(WHAT_INFO)
        } else if r == WHAT_ABOUT_URQUAN {
            self.api.npc(ABOUT_URQUAN);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_ROYALTY {
            self.api.npc(ABOUT_ROYALTY);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_WAR {
            self.api.npc(ABOUT_WAR);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_VUX {
            self.api.npc(ABOUT_VUX);
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_CLUE {
            self.api.npc(ABOUT_CLUE);
            self.api.disable(r as u16)
        }
        let mut any = false;
        for p in [
            WHAT_ABOUT_URQUAN,
            WHAT_ABOUT_ROYALTY,
            WHAT_ABOUT_WAR,
            WHAT_ABOUT_VUX,
            WHAT_ABOUT_CLUE,
        ] {
            if self.api.enabled(p as u16) {
                self.api.response(p as u16, Handler::Info, &[]);
                any = true
            }
        }
        self.api.response(ENOUGH_INFO as u16, Handler::Rebels, &[]);
        if !any {
            self.api.disable(GIVE_INFO_REBELS as u16)
        }
    }
    pub(crate) fn rebels(&mut self, r: i32) {
        if r == HOW_GOES_REVOLUTION {
            let n = self.api.get(StoryKey::YEHAT_REBEL_INFO);
            if let Some(&phrase) = [
                REBEL_REVOLUTION_1,
                REBEL_REVOLUTION_2,
                REBEL_REVOLUTION_3,
                REBEL_REVOLUTION_4,
            ]
            .get(usize::from(n))
            {
                self.api.npc(phrase);
            }
            self.api.set(
                StoryKey::YEHAT_REBEL_INFO,
                if n == 3 { n } else { n.wrapping_add(1) },
            );
            self.api.disable(r as u16)
        } else if r == ANY_SHIPS {
            if self.api.get(StoryKey::YEHAT_SHIP_MONTH) != 0 && self.api.ship_not_ready() {
                self.api.npc(NO_SHIPS_YET)
            } else {
                let n = self
                    .api
                    .escort_feasibility(crate::engine::species::ship::YEHAT_SHIP as u16)
                    as i8;
                if n == 0 {
                    self.api.npc(NO_ROOM)
                } else {
                    let count = n.min(4);
                    self.api.npc(if n < 4 {
                        HAVE_FEW_SHIPS
                    } else {
                        HAVE_ALL_SHIPS
                    });
                    self.api.talk_segue(u16::MAX);
                    self.api.add_escorts(
                        crate::engine::species::ship::YEHAT_SHIP as u16,
                        count as i16,
                    );
                    self.api.prepare_ship()
                }
            }
            self.api.disable(r as u16)
        } else if r == WHAT_ABOUT_PKUNK_REBEL {
            self.api
                .npc(if self.api.get(StoryKey::YEHAT_ABSORBED_PKUNK) != 0 {
                    PKUNK_ABSORBED_REBEL
                } else {
                    HATE_PKUNK_REBEL
                });
            self.api.set(StoryKey::YEHAT_REBEL_TOLD_PKUNK, 1)
        } else if r == ENOUGH_INFO {
            self.api.npc(OK_ENOUGH_INFO)
        }
        if self.api.enabled(HOW_GOES_REVOLUTION as u16) {
            self.api
                .response(HOW_GOES_REVOLUTION as u16, Handler::Rebels, &[])
        }
        if self.api.get(StoryKey::YEHAT_REBEL_TOLD_PKUNK) == 0
            && self.api.get(StoryKey::PKUNK_VISITS) != 0
            && self.api.get(StoryKey::PKUNK_HOME_VISITS) != 0
        {
            self.api
                .response(WHAT_ABOUT_PKUNK_REBEL as u16, Handler::Rebels, &[])
        }
        if self.api.enabled(ANY_SHIPS as u16) {
            self.api.response(ANY_SHIPS as u16, Handler::Rebels, &[])
        }
        if self.api.enabled(GIVE_INFO_REBELS as u16) {
            self.api
                .response(GIVE_INFO_REBELS as u16, Handler::Info, &[])
        }
        self.api.response(BYE_REBEL as u16, Handler::Exit, &[])
    }
    pub(crate) fn intro(&mut self) {
        self.api.segue(0);
        if self.api.activity() as u8 == ActivityKind::InLastBattle as u8 {
            self.api.npc(YEHAT_CAVALRY);
            self.api.talk_segue(u16::MAX);
            let n = (self
                .api
                .escort_feasibility(crate::engine::species::ship::YEHAT_REBEL_SHIP as u16)
                as u8)
                .min(8);
            self.api.add_escorts(
                crate::engine::species::ship::YEHAT_REBEL_SHIP as u16,
                (n - (n >> 1)) as i16,
            );
            self.api.add_escorts(
                crate::engine::species::ship::PKUNK_SHIP as u16,
                (n >> 1) as i16,
            )
        } else {
            let n = self.api.get(StoryKey::YEHAT_REBEL_VISITS);
            if let Some(&phrase) =
                [REBEL_HELLO_1, REBEL_HELLO_2, REBEL_HELLO_3, REBEL_HELLO_4].get(usize::from(n))
            {
                self.api.npc(phrase);
            }
            self.api.set(
                StoryKey::YEHAT_REBEL_VISITS,
                if n == 3 { n } else { n.wrapping_add(1) },
            );
            self.rebels(0)
        }
    }
    pub(crate) fn initialize(&mut self) {
        self.api.segue(0)
    }
}
pub(crate) fn text_layout(screen_width: i32) -> (Point, u16) {
    (
        Point {
            x: ((screen_width - 78) * 2 / 3) as i16,
            y: 60,
        },
        ((screen_width - 96) * 2 / 3) as u16,
    )
}

#[cfg(test)]
mod tests;

pub(crate) mod descriptor;
