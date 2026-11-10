use crate::comm::descriptors::starbas::descriptor;
use crate::comm::live::script::LiveScript;
use crate::comm::races::starbas::*;
use crate::comm::{locdata::CLocData, Comm};
use crate::engine::battle::Cx;
fn run(
    comm: &mut Comm,
    cx: &mut Cx<'_>,
    f: impl FnOnce(&mut Dialogue<'_, LiveScript<'_, '_, Handler>>),
) {
    let mut session = comm.race_scratch.starbas.take().unwrap_or_default();
    {
        let mut api = LiveScript::new(comm, cx, callback);
        f(&mut Dialogue {
            api: &mut api,
            session: &mut session,
        });
    }
    comm.race_scratch.starbas = Some(session);
}

fn bye_bye(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::ByeBye,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn alliance_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AllianceInfo,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn hierarchy_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::HierarchyInfo,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn alien_races(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AlienRaces,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn war_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::WarInfo,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn ancient_history(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::AncientHistory,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn tell_history(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::TellHistory,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn defeat_urquan(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::DefeatUrquan,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn tell_mission(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::TellMission,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn tell_star_base(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::TellStarBase,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn need_info(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NeedInfo,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn normal_starbase(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::NormalStarbase,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn sell_minerals(comm: &mut Comm, cx: &mut Cx<'_>, r: u32) {
    run(comm, cx, |s| {
        s.handle(
            Handler::SellMinerals,
            u16::try_from(r).expect("response reference exceeds COUNT"),
        )
    });
}
fn callback(handler: Handler) -> crate::comm::response::ResponseFunc {
    match handler {
        Handler::ByeBye => bye_bye,
        Handler::AllianceInfo => alliance_info,
        Handler::HierarchyInfo => hierarchy_info,
        Handler::AlienRaces => alien_races,
        Handler::WarInfo => war_info,
        Handler::AncientHistory => ancient_history,
        Handler::TellHistory => tell_history,
        Handler::DefeatUrquan => defeat_urquan,
        Handler::TellMission => tell_mission,
        Handler::TellStarBase => tell_star_base,
        Handler::NeedInfo => need_info,
        Handler::NormalStarbase => normal_starbase,
        Handler::SellMinerals => sell_minerals,
    }
}
pub(crate) fn intro(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.intro());
}
pub(crate) fn post(comm: &mut Comm, cx: &mut Cx<'_>) {
    run(comm, cx, |s| s.post_starbase_enc());
}
pub(crate) fn uninit(_comm: &mut Comm, _cx: &mut Cx<'_>) -> u16 {
    0
}
/// Initialize the encounter descriptor and initial battle segue.
pub fn init_starbase_comm(comm: &mut Comm, cx: &mut Cx<'_>) -> CLocData {
    let mut desc = descriptor();
    run(comm, cx, |s| s.initialize(&mut desc));
    desc
}

mod api {

    use super::Handler;
    use crate::comm::races::starbas::api::*;

    mod live {
        use super::*;
        use crate::comm::live::script::LiveScript;

        use crate::threading::legacy_abi::TaskSwitch;

        use crate::engine::world::fleet::escorts_worth;

        use crate::comm::comm_abi::enable_talking_anim;

        use crate::planets::cargo::draw_cargo;

        use crate::planets::cargo::show_capacity;

        use crate::threading::pacing::linked::sleep_thread as SleepThread;
        use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;

        impl StarbaseApi for LiveScript<'_, '_, Handler> {
            fn date(&self) -> Date {
                let cx = self.cx.borrow();
                let clock = &cx.world.data.game.game_clock;
                Date {
                    year: clock.year_index,
                    month: clock.month_index,
                    day: clock.day_index,
                }
            }
            fn ship(&self) -> Ship {
                let cx = self.cx.borrow();
                let sis = &cx.world.data.flagship;
                Ship {
                    drives: sis.drive_slots,
                    jets: sis.jet_slots,
                    modules: sis.module_slots,
                    crew: sis.crew_enlisted,
                    fuel: sis.fuel_on_board,
                    landers: sis.num_landers,
                    units: sis.res_units,
                    mineral_mass: sis.total_element_mass,
                }
            }
            fn escorts_worth(&self) -> u16 {
                // SAFETY: response callbacks run with the game-thread context published.
                {
                    {
                        let mut owner = self.cx.borrow_mut();
                        let cx = &mut **owner;
                        escorts_worth(cx.world)
                    }
                }
            }
            fn using_speech(&self) -> bool {
                self.cx.borrow().kernel.using_speech != 0
            }
            fn random(&mut self) -> u32 {
                self.cx.borrow_mut().with_legacy(crate::math::random)
            }
            fn talking_animation(&mut self, enabled: bool) {
                let comm = &mut **self.comm.borrow_mut();
                // SAFETY: the exclusive Cx loan publishes live game-thread UI and pacing owners.
                self.cx.borrow_mut().with_legacy(|| unsafe {
                    enable_talking_anim(comm, (u8::from(enabled)) as i32);
                })
            }
            fn mineral_amount(&self, category: usize) -> u16 {
                self.cx.borrow().world.data.flagship.element_amounts[category]
            }
            fn mineral_worth(&self, category: usize) -> u8 {
                self.cx.borrow().world.data.game.element_worth[category]
            }
            fn remove_minerals(&mut self, category: usize, amount: u16) {
                let mut cx = self.cx.borrow_mut();
                cx.world.data.flagship.element_amounts[category] -= amount;
                cx.world.data.flagship.total_element_mass -= amount;
            }
            fn draw_cargo(&mut self, old: u8, new: u8) {
                let cx = self.cx.borrow();
                draw_cargo(cx.kernel, cx.world, old, new);
            }
            fn show_capacity(&mut self) {
                let cx = self.cx.borrow();
                show_capacity(cx.kernel, cx.world);
            }
            fn clear_display(&mut self) {
                let cx = &mut *self.cx.borrow_mut();
                let dimensions = cx.screen_dimensions();
                crate::ui::sis::clear_sis_rect(cx.kernel, cx.world, dimensions, 1 << 3)
            }
            fn now(&mut self) -> u32 {
                self.cx
                    .borrow_mut()
                    .with_legacy(|| crate::engine::time::counter_abi::GetTimeCounter())
            }
            fn sleep(&mut self, ticks: u32) {
                // SAFETY: the exclusive Cx loan publishes live game-thread UI and pacing owners.
                self.cx.borrow_mut().with_legacy(|| unsafe {
                    SleepThread(ticks);
                })
            }
            fn sleep_until(&mut self, time: u32) {
                // SAFETY: the exclusive Cx loan publishes live game-thread UI and pacing owners.
                self.cx.borrow_mut().with_legacy(|| unsafe {
                    SleepThreadUntil(time);
                })
            }
            fn task_switch(&mut self) {
                // SAFETY: the exclusive Cx loan publishes live game-thread UI and pacing owners.
                self.cx.borrow_mut().with_legacy(|| unsafe {
                    TaskSwitch();
                })
            }
            fn skip_sale(&mut self) -> bool {
                let runtime = self.cx.borrow().kernel.input_runtime();
                let input = runtime.input();
                let mut cx = self.cx.borrow_mut();
                runtime.update(&mut cx);
                let pressed = input.current().any_button_pressed();
                pressed || cx.world.activity() & (1 << 14) != 0
            }
        }
    }
}
