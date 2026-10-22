//! Syreen dialogue, ported from syreenc.c.
use super::script::{animation, as_comm_data};
use crate::comm::locdata::{CColor, CLocData, CPoint};
use crate::comm::types::CommData;
mod phrases;
use phrases::*;
mod api;
use api::SyreenApi;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Handler {
    FriendlyExit,
    Sex,
    Foreplay,
    AfterAmbush,
    AmbushReady,
    SyreenShuttle,
    NormalSyreen,
    InitialSyreen,
    SyreenVault,
}
struct Dialogue<'a, A> {
    api: &'a mut A,
}
impl<A: SyreenApi> Dialogue<'_, A> {
    fn handle(&mut self, h: Handler, r: i32) {
        match h {
            Handler::FriendlyExit => self.friendly_exit(r),
            Handler::Sex => self.sex(r),
            Handler::Foreplay => self.foreplay(r),
            Handler::AfterAmbush => self.after_ambush(r),
            Handler::AmbushReady => self.ambush_ready(r),
            Handler::SyreenShuttle => self.syreen_shuttle(r),
            Handler::NormalSyreen => self.normal_syreen(r),
            Handler::InitialSyreen => self.initial_syreen(r),
            Handler::SyreenVault => self.syreen_vault(r),
        }
    }
    fn friendly_exit(&mut self, r: i32) {
        self.api.segue(0);
        if r == BYE {
            self.api.npc(GOODBYE);
        } else if ((r == IM_ON_MY_WAY) || (r == DOING_THIS_FOR_YOU)) || (r == IF_I_DIE) {
            self.api.npc(GOOD_LUCK);
        } else if r == BYE_BEFORE_VAULT {
            self.api.npc(GOODBYE_BEFORE_VAULT);
        } else if r == BYE_AFTER_VAULT {
            self.api.npc(GOODBYE_AFTER_VAULT);
        } else if r == BYE_BEFORE_AMBUSH {
            self.api.npc(GOODBYE_BEFORE_AMBUSH);
        } else if r == BYE_AFTER_AMBUSH {
            self.api.npc(GOODBYE_AFTER_AMBUSH);
        } else {
            if r == HANDS_OFF {
                self.api.npc(OK_WONT_USE_HANDS);
            } else if r == NOT_MUCH_MORE_TO_SAY {
                self.api.npc(THEN_STOP_TALKING);
            }
            self.api.npc(LATER);
            self.api.npc(SEX_GOODBYE);
            self.api.talk_segue(2_u16);
            self.api.fade_palette(0_u16, (840 / 2) as i16);
            self.api.talk_segue((!0_u16 as i32) as u16);
            self.api.set("PLAYER_HAD_SEX", 1_u8);
            self.api.set("PLAYER_HAVING_SEX", 0_u8);
        }
    }
    fn sex(&mut self, r: i32) {
        if r == IN_THE_SPIRIT {
            self.api.npc(OK_SPIRIT);
        } else if r == WHAT_IN_MIND {
            self.api.npc(SOMETHING_LIKE_THIS);
        } else if r == DISEASE {
            self.api.npc(JUST_RELAX);
        } else if r == WHAT_HAPPENS_IF_I_TOUCH_THIS {
            self.api.npc(THIS_HAPPENS);
            self.api.disable((WHAT_HAPPENS_IF_I_TOUCH_THIS) as u16);
        } else if r == ARE_YOU_SURE_THIS_IS_OK {
            self.api.npc(YES_SURE);
            self.api.disable((ARE_YOU_SURE_THIS_IS_OK) as u16);
        } else if r == BOY_THEY_NEVER_TAUGHT {
            self.api.npc(THEN_LET_ME_TEACH);
            self.api.disable((BOY_THEY_NEVER_TAUGHT) as u16);
        }
        if (!(self.api.enabled((WHAT_HAPPENS_IF_I_TOUCH_THIS) as u16))
            && !(self.api.enabled((ARE_YOU_SURE_THIS_IS_OK) as u16)))
            && !(self.api.enabled((BOY_THEY_NEVER_TAUGHT) as u16))
        {
            self.api
                .response((NOT_MUCH_MORE_TO_SAY) as u16, Handler::FriendlyExit, &[]);
        } else {
            if self.api.enabled((WHAT_HAPPENS_IF_I_TOUCH_THIS) as u16) {
                self.api
                    .response((WHAT_HAPPENS_IF_I_TOUCH_THIS) as u16, Handler::Sex, &[]);
            }
            if self.api.enabled((ARE_YOU_SURE_THIS_IS_OK) as u16) {
                self.api
                    .response((ARE_YOU_SURE_THIS_IS_OK) as u16, Handler::Sex, &[]);
            }
            if self.api.enabled((BOY_THEY_NEVER_TAUGHT) as u16) {
                self.api
                    .response((BOY_THEY_NEVER_TAUGHT) as u16, Handler::Sex, &[]);
            }
        }
    }
    fn foreplay(&mut self, r: i32) {
        if (r == WHATS_MY_REWARD) || (r == WHAT_ABOUT_US) {
            if r == WHATS_MY_REWARD {
                self.api.npc(HERES_REWARD);
            } else {
                self.api.npc(ABOUT_US);
            }
            self.api.npc(MORE_COMFORTABLE);
            self.api.talk_segue(1_u16);
            self.api.fade_palette(1_u16, 840_i16);
            self.api.talk_segue((!0_u16 as i32) as u16);
            self.api.set("PLAYER_HAVING_SEX", 1_u8);
        } else if r == WHY_LIGHTS_OFF {
            self.api.npc(LIGHTS_OFF_BECAUSE);
            self.api.disable((WHY_LIGHTS_OFF) as u16);
        } else if r == EVIL_MONSTER {
            self.api.npc(NOT_EVIL_MONSTER);
            self.api.disable((EVIL_MONSTER) as u16);
        }
        if self.api.enabled((WHY_LIGHTS_OFF) as u16) {
            self.api
                .response((WHY_LIGHTS_OFF) as u16, Handler::Foreplay, &[]);
        } else if self.api.enabled((EVIL_MONSTER) as u16) {
            self.api
                .response((EVIL_MONSTER) as u16, Handler::Foreplay, &[]);
        } else {
            self.api.response((DISEASE) as u16, Handler::Sex, &[]);
        }
        self.api.response((IN_THE_SPIRIT) as u16, Handler::Sex, &[]);
        self.api.response((WHAT_IN_MIND) as u16, Handler::Sex, &[]);
        self.api
            .response((HANDS_OFF) as u16, Handler::FriendlyExit, &[]);
    }
    fn after_ambush(&mut self, r: i32) {
        if r == WHAT_NOW_AFTER_AMBUSH {
            self.api.npc(DO_THIS_AFTER_AMBUSH);
            self.api.disable((WHAT_NOW_AFTER_AMBUSH) as u16);
        } else if r == WHAT_ABOUT_YOU {
            self.api.npc(ABOUT_ME);
            self.api.disable((WHAT_ABOUT_YOU) as u16);
        } else if r == WHATS_UP_AFTER_AMBUSH {
            let mut num_visits: i32;
            num_visits = i32::from(self.api.get("SYREEN_INFO"));
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(GENERAL_INFO_AFTER_AMBUSH_1);
                }
                1 => {
                    self.api.npc(GENERAL_INFO_AFTER_AMBUSH_2);
                }
                2 => {
                    self.api.npc(GENERAL_INFO_AFTER_AMBUSH_3);
                }
                3 => {
                    self.api.npc(GENERAL_INFO_AFTER_AMBUSH_4);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.api.set("SYREEN_INFO", (num_visits) as u8);
            self.api.disable((WHATS_UP_AFTER_AMBUSH) as u16);
        }
        if self.api.enabled((WHAT_ABOUT_YOU) as u16) {
            self.api
                .response((WHAT_ABOUT_YOU) as u16, Handler::AfterAmbush, &[]);
        } else if !(i32::from(self.api.get("PLAYER_HAD_SEX")) != 0) {
            self.api
                .response((WHAT_ABOUT_US) as u16, Handler::Foreplay, &[]);
        }
        if self.api.enabled((WHAT_NOW_AFTER_AMBUSH) as u16) {
            self.api
                .response((WHAT_NOW_AFTER_AMBUSH) as u16, Handler::AfterAmbush, &[]);
        }
        if self.api.enabled((WHATS_UP_AFTER_AMBUSH) as u16) {
            self.api
                .response((WHATS_UP_AFTER_AMBUSH) as u16, Handler::AfterAmbush, &[]);
        }
        self.api
            .response((BYE_AFTER_AMBUSH) as u16, Handler::FriendlyExit, &[]);
    }
    fn ambush_ready(&mut self, r: i32) {
        if r == REPEAT_PLAN {
            self.api.npc(OK_REPEAT_PLAN);
            self.api.disable((REPEAT_PLAN) as u16);
        }
        if self.api.enabled((REPEAT_PLAN) as u16) {
            self.api
                .response((REPEAT_PLAN) as u16, Handler::AmbushReady, &[]);
        }
        self.api
            .response((BYE_BEFORE_AMBUSH) as u16, Handler::FriendlyExit, &[]);
    }
    fn syreen_shuttle(&mut self, r: i32) {
        if r == WHATS_NEXT_STEP {
            self.api.npc(OPEN_VAULT);
            self.api.disable((WHATS_NEXT_STEP) as u16);
        } else if r == WHAT_DO_I_GET_FOR_THIS {
            self.api.npc(GRATITUDE);
            self.api.disable((WHAT_DO_I_GET_FOR_THIS) as u16);
        } else if r == NOT_SURE {
            self.api.npc(PLEASE);
            self.api.disable((NOT_SURE) as u16);
        } else if r == WHERE_IS_IT {
            self.api.npc(DONT_KNOW_WHERE);
            self.api.npc(GIVE_SHUTTLE);
            self.api.set("SYREEN_SHUTTLE", 1_u8);
            self.api.set("SYREEN_SHUTTLE_ON_SHIP", 1_u8);
            self.api.disable((WHERE_IS_IT) as u16);
        } else if r == BEEN_THERE {
            self.api.npc(GREAT);
            self.api.npc(GIVE_SHUTTLE);
            self.api.set("SYREEN_SHUTTLE", 1_u8);
            self.api.set("SYREEN_SHUTTLE_ON_SHIP", 1_u8);
            self.api.disable((BEEN_THERE) as u16);
        }
        if self.api.enabled((WHATS_NEXT_STEP) as u16) {
            self.api
                .response((WHATS_NEXT_STEP) as u16, Handler::SyreenShuttle, &[]);
        } else {
            if !(i32::from(self.api.get("KNOW_SYREEN_VAULT")) != 0) {
                if self.api.enabled((WHERE_IS_IT) as u16) {
                    self.api
                        .response((WHERE_IS_IT) as u16, Handler::SyreenShuttle, &[]);
                }
            } else {
                if self.api.enabled((BEEN_THERE) as u16) {
                    self.api
                        .response((BEEN_THERE) as u16, Handler::SyreenShuttle, &[]);
                }
            }
            if !(self.api.enabled((WHERE_IS_IT) as u16)) || !(self.api.enabled((BEEN_THERE) as u16))
            {
                self.api
                    .response((IM_ON_MY_WAY) as u16, Handler::FriendlyExit, &[]);
                self.api
                    .response((DOING_THIS_FOR_YOU) as u16, Handler::FriendlyExit, &[]);
                self.api
                    .response((IF_I_DIE) as u16, Handler::FriendlyExit, &[]);
            }
        }
        if self.api.enabled((WHAT_DO_I_GET_FOR_THIS) as u16) {
            self.api
                .response((WHAT_DO_I_GET_FOR_THIS) as u16, Handler::SyreenShuttle, &[]);
        }
        if self.api.enabled((NOT_SURE) as u16) {
            self.api
                .response((NOT_SURE) as u16, Handler::SyreenShuttle, &[]);
        }
    }
    fn normal_syreen(&mut self, r: i32) {
        let mut i: i32;
        let mut last_stack: i32;
        let mut p_str = [0i32; 4];
        last_stack = 0;
        p_str[3_usize] = 0;
        p_str[2_usize] = p_str[3_usize];
        p_str[1_usize] = p_str[2_usize];
        p_str[0_usize] = p_str[1_usize];
        if r == WE_HERE_TO_HELP {
            self.api.npc(NO_NEED_HELP);
        } else if r == WE_NEED_HELP {
            self.api.npc(CANT_GIVE_HELP);
        } else if r == KNOW_ABOUT_DEEP_CHILDREN {
            self.api.npc(WHAT_ABOUT_DEEP_CHILDREN);
            self.api.disable((KNOW_ABOUT_DEEP_CHILDREN) as u16);
        } else if r == MYCONS_INVOLVED {
            self.api.npc(WHAT_PROOF);
            self.api.set("KNOW_ABOUT_SHATTERED", 3_u8);
        } else if r == HAVE_NO_PROOF {
            self.api.npc(NEED_PROOF);
            self.api.set("SYREEN_WANT_PROOF", 1_u8);
        } else if r == HAVE_PROOF {
            self.api.npc(SEE_PROOF);
            self.api.disable((HAVE_PROOF) as u16);
        } else if r == WHAT_DOING_HERE {
            self.api.npc(OUR_NEW_WORLD);
            self.api.set("SYREEN_STACK0", 1_u8);
            last_stack = 1;
        } else if r == WHAT_ABOUT_WAR {
            self.api.npc(ABOUT_WAR);
            self.api.set("SYREEN_STACK0", 2_u8);
            last_stack = 1;
        } else if r == HELP_US {
            self.api.npc(WONT_HELP);
            self.api.set("SYREEN_STACK0", 3_u8);
        } else if r == WHAT_ABOUT_HISTORY {
            self.api.npc(BEFORE_WAR);
            self.api.set("SYREEN_STACK1", 1_u8);
            last_stack = 2;
        } else if r == WHAT_ABOUT_HOMEWORLD {
            self.api.npc(ABOUT_HOMEWORLD);
            self.api.set("SYREEN_STACK1", 2_u8);
            last_stack = 2;
        } else if r == WHAT_HAPPENED {
            self.api.npc(DONT_KNOW_HOW);
            self.api.set("KNOW_SYREEN_WORLD_SHATTERED", 1_u8);
            self.api.set("SYREEN_STACK1", 3_u8);
        } else if r == WHAT_ABOUT_OUTFIT {
            self.api.npc(HOPE_YOU_LIKE_IT);
            self.api.set("SYREEN_STACK2", 1_u8);
            last_stack = 3;
        } else if r == WHERE_MATES {
            self.api.npc(MATES_KILLED);
            self.api.set("SYREEN_STACK2", 2_u8);
            last_stack = 3;
        } else if r == GET_LONELY {
            self.api.npc(MAKE_OUT_ALL_RIGHT);
            self.api.set("SYREEN_STACK2", 3_u8);
        } else if r == LOOK_AT_EGG_SACKS {
            self.api.npc(HORRIBLE_TRUTH);
            self.api.segue(0);
            self.api.set("SYREEN_HOME_VISITS", 0_u8);
            self.api.set("SYREEN_KNOW_ABOUT_MYCON", 1_u8);
            self.syreen_shuttle(0_u16 as i32);
            return;
        }
        if i32::from(self.api.get("KNOW_ABOUT_SHATTERED")) < 3 {
            if (i32::from(self.api.get("KNOW_ABOUT_SHATTERED")) == 2)
                && (i32::from(self.api.get("KNOW_SYREEN_WORLD_SHATTERED")) != 0)
            {
                if self.api.enabled((KNOW_ABOUT_DEEP_CHILDREN) as u16) {
                    p_str[0_usize] = KNOW_ABOUT_DEEP_CHILDREN;
                } else {
                    p_str[0_usize] = MYCONS_INVOLVED;
                }
            }
        } else {
            if ((i32::from(self.api.get("EGG_CASE0_ON_SHIP")) != 0)
                || (i32::from(self.api.get("EGG_CASE1_ON_SHIP")) != 0))
                || (i32::from(self.api.get("EGG_CASE2_ON_SHIP")) != 0)
            {
                if self.api.enabled((HAVE_PROOF) as u16) {
                    p_str[0_usize] = HAVE_PROOF;
                } else {
                    p_str[0_usize] = LOOK_AT_EGG_SACKS;
                }
            } else if !(i32::from(self.api.get("SYREEN_WANT_PROOF")) != 0) {
                p_str[0_usize] = HAVE_NO_PROOF;
            }
        }
        match i32::from(self.api.get("SYREEN_STACK0")) {
            0 => {
                p_str[1_usize] = WHAT_DOING_HERE;
            }
            1 => {
                p_str[1_usize] = WHAT_ABOUT_WAR;
            }
            2 => {
                p_str[1_usize] = HELP_US;
            }
            _ => {}
        }
        match i32::from(self.api.get("SYREEN_STACK1")) {
            0 => {
                p_str[2_usize] = WHAT_ABOUT_HISTORY;
            }
            1 => {
                p_str[2_usize] = WHAT_ABOUT_HOMEWORLD;
            }
            2 => {
                p_str[2_usize] = WHAT_HAPPENED;
            }
            _ => {}
        }
        match i32::from(self.api.get("SYREEN_STACK2")) {
            0 => {
                p_str[3_usize] = WHAT_ABOUT_OUTFIT;
            }
            1 => {
                p_str[3_usize] = WHERE_MATES;
            }
            2 => {
                p_str[3_usize] = GET_LONELY;
            }
            _ => {}
        }
        if p_str[last_stack as usize] != 0 {
            self.api.response(
                (p_str[last_stack as usize]) as u16,
                Handler::NormalSyreen,
                &[],
            );
        }
        {
            i = 0;
            while i < 4 {
                {
                    if (i != last_stack) && (p_str[i as usize] != 0) {
                        self.api
                            .response((p_str[i as usize]) as u16, Handler::NormalSyreen, &[]);
                    }
                }
                {
                    i = (i + 1) as u8 as i32;
                    i
                };
            }
        }
        self.api.response((BYE) as u16, Handler::FriendlyExit, &[]);
    }
    fn initial_syreen(&mut self, r: i32) {
        if r == WE_ARE_VICE_SQUAD {
            self.api.npc(OK_VICE);
            self.api.npc(HOW_CAN_YOU_BE_HERE);
        } else if r == WE_ARE_THE_ONE_FOR_YOU_BABY {
            self.api.npc(MAYBE_CAPTAIN);
            self.api.npc(HOW_CAN_YOU_BE_HERE);
        } else if r == WE_ARE_VINDICATOR0 {
            self.api.npc(WELCOME_VINDICATOR0);
            if !(self.api.using_speech()) {
                self.api.npc(super::script::GLOBAL_PLAYER_NAME);
                self.api.npc(WELCOME_VINDICATOR1);
                self.api.npc(super::script::GLOBAL_SHIP_NAME);
                self.api.npc(WELCOME_VINDICATOR2);
            }
            self.api.npc(HOW_CAN_YOU_BE_HERE);
        } else if r == WE_ARE_IMPRESSED {
            self.api.npc(SO_AM_I_CAPTAIN);
            self.api.npc(HOW_CAN_YOU_BE_HERE);
        } else if r == I_NEED_YOU {
            self.api.npc(OK_NEED);
            self.api.disable((I_NEED_YOU) as u16);
            self.api.disable((I_NEED_TOUCH_O_VISION) as u16);
        } else if r == I_NEED_TOUCH_O_VISION {
            self.api.npc(TOUCH_O_VISION);
            self.api.disable((I_NEED_YOU) as u16);
            self.api.disable((I_NEED_TOUCH_O_VISION) as u16);
        }
        self.api
            .response((WE_HERE_TO_HELP) as u16, Handler::NormalSyreen, &[]);
        self.api
            .response((WE_NEED_HELP) as u16, Handler::NormalSyreen, &[]);
        if self.api.enabled((I_NEED_YOU) as u16) {
            self.api
                .response((I_NEED_YOU) as u16, Handler::InitialSyreen, &[]);
        }
        if self.api.enabled((I_NEED_TOUCH_O_VISION) as u16) {
            self.api
                .response((I_NEED_TOUCH_O_VISION) as u16, Handler::InitialSyreen, &[]);
        }
    }
    fn plan_ambush(&mut self, _r: i32) {
        self.api.npc(OK_FOUND_VAULT);
        self.api.set("MYCON_AMBUSH", 1_u8);
        self.api.set("SYREEN_HOME_VISITS", 0_u8);
        self.api
            .response((WHATS_MY_REWARD) as u16, Handler::Foreplay, &[]);
        self.api
            .response((BYE_AFTER_VAULT) as u16, Handler::FriendlyExit, &[]);
    }
    fn syreen_vault(&mut self, r: i32) {
        if r == VAULT_HINT {
            self.api.npc(OK_HINT);
            self.api.disable((VAULT_HINT) as u16);
        }
        if self.api.enabled((VAULT_HINT) as u16) {
            self.api
                .response((VAULT_HINT) as u16, Handler::SyreenVault, &[]);
        }
        self.api
            .response((BYE_BEFORE_VAULT) as u16, Handler::FriendlyExit, &[]);
    }
    fn intro(&mut self) {
        let mut num_visits: i32;
        if (i32::from(self.api.activity()) as u8 as i32) == 5 {
            self.api.npc(OUT_TAKES);
            self.api.segue(0);
            return;
        }
        num_visits = i32::from(self.api.get("SYREEN_HOME_VISITS"));
        if i32::from(self.api.get("MYCON_KNOW_AMBUSH")) != 0 {
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HELLO_AFTER_AMBUSH_1);
                    self.api.set_allied(
                        (crate::comm::dispatch::ship::SYREEN_SHIP as i32) as u16,
                        1 != 0,
                    );
                }
                1 => {
                    self.api.npc(HELLO_AFTER_AMBUSH_2);
                }
                2 => {
                    self.api.npc(HELLO_AFTER_AMBUSH_3);
                }
                3 => {
                    self.api.npc(HELLO_AFTER_AMBUSH_3);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.after_ambush(0_u16 as i32);
        } else if i32::from(self.api.get("MYCON_AMBUSH")) != 0 {
            if {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            } == 0
            {
                self.api.npc(READY_FOR_AMBUSH);
                {
                    num_visits = (num_visits - 1) as u8 as i32;
                    num_visits
                };
            }
            self.ambush_ready(0_u16 as i32);
        } else if !(i32::from(self.api.get("SYREEN_KNOW_ABOUT_MYCON")) != 0) {
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(HELLO_BEFORE_AMBUSH_1);
                }
                1 => {
                    self.api.npc(HELLO_BEFORE_AMBUSH_2);
                }
                2 => {
                    self.api.npc(HELLO_BEFORE_AMBUSH_3);
                }
                3 => {
                    self.api.npc(HELLO_BEFORE_AMBUSH_4);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            if num_visits > 1 {
                self.normal_syreen(0_u16 as i32);
            } else {
                self.api
                    .response((WE_ARE_VICE_SQUAD) as u16, Handler::InitialSyreen, &[]);
                self.api.response(
                    (WE_ARE_THE_ONE_FOR_YOU_BABY) as u16,
                    Handler::InitialSyreen,
                    &[],
                );
                self.api.response(
                    (WE_ARE_VINDICATOR0) as u16,
                    Handler::InitialSyreen,
                    &[
                        WE_ARE_VINDICATOR0,
                        super::script::GLOBAL_PLAYER_NAME,
                        WE_ARE_VINDICATOR1,
                        super::script::GLOBAL_SHIP_NAME,
                        WE_ARE_VINDICATOR2,
                    ],
                );
                self.api
                    .response((WE_ARE_IMPRESSED) as u16, Handler::InitialSyreen, &[]);
            }
        } else if i32::from(self.api.get("SHIP_VAULT_UNLOCKED")) != 0 {
            self.plan_ambush(0_u16 as i32);
            num_visits = 0;
        } else {
            let res = {
                let old = num_visits;
                num_visits = (num_visits + 1) as u8 as i32;
                old
            };
            match res {
                0 => {
                    self.api.npc(FOUND_VAULT_YET_1);
                }
                1 => {
                    self.api.npc(FOUND_VAULT_YET_2);
                    {
                        num_visits = (num_visits - 1) as u8 as i32;
                        num_visits
                    };
                }
                _ => {}
            }
            self.syreen_vault(0_u16 as i32);
        }
        self.api.set("SYREEN_HOME_VISITS", (num_visits) as u8);
    }
    fn post_syreen_enc(&mut self) {}
    fn initialize(&mut self) {
        self.api.segue(0);
    }
}
fn descriptor() -> CLocData {
    let mut d = CLocData {
        alien_frame_res: c"comm.syreen.graphics".as_ptr(),
        alien_font_res: c"comm.syreen.font".as_ptr(),
        alien_colormap_res: c"comm.syreen.colortable".as_ptr(),
        alien_song_res: c"comm.syreen.music".as_ptr(),
        conversation_phrases_res: c"comm.syreen.dialogue".as_ptr(),
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
        alien_text_baseline: CPoint { x: 121, y: 0 },
        alien_text_width: 224,
        alien_text_align: 1,
        alien_text_valign: 0,
        num_animations: 15,
        ..CLocData::default()
    };
    d.alien_ambient_array[0] = animation(5, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[1] = animation(7, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[2] = animation(9, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[3] = animation(11, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[4] = animation(13, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[5] = animation(15, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[5].block_mask = 4096;
    d.alien_ambient_array[6] = animation(17, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[7] = animation(19, 2, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[7].block_mask = 8192;
    d.alien_ambient_array[8] = animation(21, 6, 1, (56, 56), (840, 2520));
    d.alien_ambient_array[9] = animation(27, 4, 4, (56, 56), (8400, 2520));
    d.alien_ambient_array[9].block_mask = 16384;
    d.alien_ambient_array[10] = animation(31, 6, 2, (56, 56), (840, 2520));
    d.alien_ambient_array[11] = animation(37, 4, 1, (56, 56), (56, 56));
    d.alien_ambient_array[12] = animation(41, 3, 4, (84, 56), (8400, 2520));
    d.alien_ambient_array[12].block_mask = 32;
    d.alien_ambient_array[13] = animation(44, 4, 12, (140, 0), (2520, 840));
    d.alien_ambient_array[13].block_mask = 16512;
    d.alien_ambient_array[14] = animation(48, 3, 12, (112, 56), (8400, 840));
    d.alien_ambient_array[14].block_mask = 8704;
    d.alien_transition_desc = animation(0, 0, 0, (0, 0), (0, 0));
    d.alien_talk_desc = animation(1, 4, 0, (56, 0), (70, 0));
    #[cfg(not(test))]
    {
        d.init_encounter_func = live::intro as *const () as *const _;
        d.post_encounter_func = live::post as *const () as *const _;
        d.uninit_encounter_func = live::uninit as *const () as *const _;
    }
    d
}
#[cfg(not(test))]
mod live {
    use super::super::script::live::LiveScript;
    use super::*;
    fn run(f: impl FnOnce(&mut Dialogue<'_, LiveScript<Handler>>)) {
        let mut api = LiveScript { callback };
        f(&mut Dialogue { api: &mut api });
    }
    extern "C" fn friendly_exit(r: u32) {
        run(|s| {
            s.handle(
                Handler::FriendlyExit,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    extern "C" fn sex(r: u32) {
        run(|s| s.handle(Handler::Sex, i32::try_from(r).expect("invalid response")));
    }
    extern "C" fn foreplay(r: u32) {
        run(|s| {
            s.handle(
                Handler::Foreplay,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    extern "C" fn after_ambush(r: u32) {
        run(|s| {
            s.handle(
                Handler::AfterAmbush,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    extern "C" fn ambush_ready(r: u32) {
        run(|s| {
            s.handle(
                Handler::AmbushReady,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    extern "C" fn syreen_shuttle(r: u32) {
        run(|s| {
            s.handle(
                Handler::SyreenShuttle,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    extern "C" fn normal_syreen(r: u32) {
        run(|s| {
            s.handle(
                Handler::NormalSyreen,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    extern "C" fn initial_syreen(r: u32) {
        run(|s| {
            s.handle(
                Handler::InitialSyreen,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    extern "C" fn syreen_vault(r: u32) {
        run(|s| {
            s.handle(
                Handler::SyreenVault,
                i32::try_from(r).expect("invalid response"),
            )
        });
    }
    fn callback(h: Handler) -> extern "C" fn(u32) {
        match h {
            Handler::FriendlyExit => friendly_exit,
            Handler::Sex => sex,
            Handler::Foreplay => foreplay,
            Handler::AfterAmbush => after_ambush,
            Handler::AmbushReady => ambush_ready,
            Handler::SyreenShuttle => syreen_shuttle,
            Handler::NormalSyreen => normal_syreen,
            Handler::InitialSyreen => initial_syreen,
            Handler::SyreenVault => syreen_vault,
        }
    }
    pub(super) extern "C" fn intro() {
        run(|s| s.intro());
    }
    pub(super) extern "C" fn post() {
        run(|s| s.post_syreen_enc());
    }
    pub(super) extern "C" fn uninit() -> u16 {
        0
    }
    /// Initialize the encounter on the game thread.
    ///
    /// # Safety
    /// The caller must copy the descriptor before another encounter initializes.
    #[no_mangle]
    pub unsafe extern "C" fn init_syreen_comm() -> *mut CLocData {
        static mut DESC: Option<CLocData> = None;
        let desc = (&mut *std::ptr::addr_of_mut!(DESC)).get_or_insert_with(descriptor);
        run(|s| s.initialize());
        desc
    }
}
pub struct SyreenDialogue;
impl super::RaceDialogue for SyreenDialogue {
    fn init(&self) -> CommData {
        #[cfg(not(test))]
        {
            unsafe { as_comm_data(&*live::init_syreen_comm()) }
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
