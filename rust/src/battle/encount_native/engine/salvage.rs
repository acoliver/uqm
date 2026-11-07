//! Retained salvage panel resources and the ordered post-battle reward display.
use super::*;
use crate::engine::{battle::Cx, observe::Io};
use crate::graphics::context_abi::{ContextFrame, ContextLease};

fn draw_salvage_line(
    kernel: &crate::engine::Kernel,
    string: &std::ffi::CStr,
    baseline: POINT,
    style: SalvageText,
) {
    let context = crate::graphics::context_abi::ContextLease::retain(
        std::ptr::NonNull::new(kernel.space_context)
            .unwrap_or_else(|| panic!("salvage requires a space context")),
    );
    let (font, align, color) = match style {
        SalvageText::Caption => (
            kernel.tiny_font_lease(),
            TEXT_ALIGN_ALIGN_CENTER,
            rgb15(8, 8, 8),
        ),
        SalvageText::Amount => (
            kernel.micro_font_lease(),
            TEXT_ALIGN_ALIGN_RIGHT,
            rgb15(0, 0, 24),
        ),
        SalvageText::Total => (
            kernel.micro_font_lease(),
            TEXT_ALIGN_ALIGN_CENTER,
            rgb15(0, 0, 24),
        ),
    };
    let font = font.unwrap_or_else(|| panic!("salvage requires its retained text font"));
    context.draw_text(&font, string, baseline, align, color);
}

fn draw_salvage_captions(kernel: &crate::engine::Kernel, name: &[UNICODE; 80], rect: &RECT) {
    let baseline = POINT {
        x: rect.corner.x.wrapping_add(100),
        y: rect.corner.y.wrapping_add(68),
    };
    let bytes = name.map(|byte| byte as u8);
    let name = std::ffi::CStr::from_bytes_until_nul(&bytes)
        .unwrap_or_else(|error| panic!("unterminated salvage name: {error}"));
    draw_salvage_line(kernel, name, baseline, SalvageText::Caption);
    draw_salvage_line(
        kernel,
        kernel.game_text((ENCOUNTER_STRING_BASE + 3) as usize),
        POINT {
            y: baseline.y.wrapping_add(6),
            ..baseline
        },
        SalvageText::Caption,
    );
}

fn draw_fade_text(
    cx: &mut crate::engine::battle::Cx<'_>,
    input: &crate::engine::input::InputSession,
    str1: &std::ffi::CStr,
    str2: &std::ffi::CStr,
    fade_in: BOOLEAN,
    rect: &RECT,
) {
    let context = crate::graphics::context_abi::ContextLease::retain(
        std::ptr::NonNull::new(cx.kernel.space_context)
            .unwrap_or_else(|| panic!("salvage requires a space context")),
    );
    let font = cx
        .kernel
        .micro_font_lease()
        .unwrap_or_else(|| panic!("salvage requires the micro font"));
    let baseline = POINT {
        x: rect.corner.x.wrapping_add(100),
        y: rect.corner.y.wrapping_add(45),
    };
    let render = |color| {
        context.draw_text(&font, str1, baseline, TEXT_ALIGN_ALIGN_CENTER, color);
        context.draw_text(
            &font,
            str2,
            POINT {
                y: baseline.y.wrapping_add(11),
                ..baseline
            },
            TEXT_ALIGN_ALIGN_CENTER,
            color,
        );
    };
    let runtime = cx.kernel.input_runtime();
    runtime.flush();
    let mut time = GetTimeCounter();
    if fade_in != 0 {
        let mut i = 0usize;
        while i < TEXT_FADE.len() {
            runtime.update(cx);
            if input.current().any_button_pressed() {
                i = TEXT_FADE.len() - 1;
            }
            let v = TEXT_FADE[i];
            render(rgb15(v, v, v));
            runtime.sleep_until(cx, time.wrapping_add(ONE_SECOND / 20));
            time = GetTimeCounter();
            i += 1;
        }
    } else {
        let mut i = TEXT_FADE.len() as i16 - 1;
        while i >= 0 {
            runtime.update(cx);
            if input.current().any_button_pressed() {
                i = 0;
            }
            let v = TEXT_FADE[i as usize];
            render(rgb15(v, v, v));
            runtime.sleep_until(cx, time.wrapping_add(ONE_SECOND / 20));
            time = GetTimeCounter();
            i -= 1;
        }
        render(rgb15(10, 10, 10));
    }
}

pub(super) struct Salvage {
    context: ContextLease,
    rect: RECT,
    salvage_rect: RECT,
    stamp_origin: POINT,
    stamp_frame: ContextFrame,
    str1: std::ffi::CString,
    str2: std::ffi::CString,
    buf: [UNICODE; 80],
    recycle: u16,
    pub(super) victory: i16,
    sleepy: bool,
}
impl Salvage {
    pub(super) fn new(kernel: &crate::engine::Kernel, victory: i16) -> Self {
        let rect = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: 0,
                height: 0,
            },
        };
        Self {
            context: ContextLease::require(kernel.space_context),
            rect,
            salvage_rect: rect,
            stamp_origin: POINT { x: 0, y: 0 },
            stamp_frame: ContextFrame::retain(None),
            str1: std::ffi::CString::default(),
            str2: std::ffi::CString::default(),
            buf: [0; 80],
            recycle: 0,
            victory,
            sleepy: true,
        }
    }
    pub(super) fn panel(
        &mut self,
        cx: &mut Cx<'_>,
        catalog: Option<&[CStarDesc; CATALOG_LEN]>,
        draw_armada: impl FnOnce(
            &crate::engine::Kernel,
            &crate::engine::World,
            i32,
            &mut crate::comm::locdata::CRect,
        ),
    ) {
        crate::ui::border::draw_sis_frame(cx.kernel, cx.world, cx.screen_dimensions());
        let message = crate::ui::sis::default_message_text(cx.kernel, cx.world, catalog);
        crate::ui::sis::draw_message(
            cx.kernel,
            cx.screen_dimensions().0,
            &message,
            -1,
            -1,
            DSME_NONE as u16,
        );
        display::draw_title(cx, catalog);
        crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context).select();
        if self.victory != 0 {
            let mut panel = crate::comm::locdata::CRect {
                corner: crate::comm::locdata::CPoint { x: 0, y: 0 },
                width: 0,
                height: 0,
            };
            draw_armada(cx.kernel, cx.world, 1, &mut panel);
            self.salvage_rect = RECT {
                corner: POINT {
                    x: panel.corner.x,
                    y: panel.corner.y,
                },
                extent: EXTENT {
                    width: panel.width,
                    height: panel.height,
                },
            };
        }
    }
    pub(super) fn start(
        &mut self,
        cx: &mut Cx<'_>,
        input: &crate::engine::input::InputSession,
        frag: &SHIP_FRAGMENT,
    ) {
        self.recycle = 0;
        crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
        let kernel = &*cx.kernel;
        self.stamp_origin.x = self.salvage_rect.corner.x.wrapping_add(32);
        self.stamp_origin.y = self.salvage_rect.corner.y.wrapping_add(56);
        let resources = cx.world.fleet_resources(frag.species_id);
        self.stamp_frame = crate::graphics::context_abi::ContextFrame::retain(
            std::ptr::NonNull::new(resources.icon().raw()),
        );
        self.context
            .draw_stamp(&self.stamp_frame.next_frame(), self.stamp_origin);
        let (name, _) = resources.label(0);
        crate::ships::native::copy_display_name(&mut self.buf, name.to_bytes());
        for byte in &mut self.buf {
            *byte = (*byte as u8).to_ascii_uppercase() as UNICODE;
        }
        draw_salvage_captions(kernel, &self.buf, &self.salvage_rect);

        self.str1 = kernel
            .game_text((ENCOUNTER_STRING_BASE + 4) as usize)
            .to_owned();
        self.str2 = kernel
            .game_text((ENCOUNTER_STRING_BASE + 5) as usize)
            .to_owned();
        draw_fade_text(cx, input, &self.str1, &self.str2, 1, &self.salvage_rect);
    }
    pub(super) fn reward(&mut self, cx: &mut Cx<'_>, input: &crate::engine::input::InputSession) {
        self.rect.corner.y = self.salvage_rect.corner.y.wrapping_add(9);
        self.rect.extent.height = 22;
        self.rect.extent.width = 34;
        self.rect.corner.x = (self.salvage_rect.corner.x as i32
            + self.salvage_rect.extent.width as i32
            - (10 + self.rect.extent.width as i32)) as i16;
        self.context.fill_rectangle(rgb15(0, 0, 0), self.rect);
        let bounty = RACE_BOUNTY[cx.world.encounter_race as usize] >> 3;
        self.recycle = self.recycle.wrapping_add(bounty);
        let amount = std::ffi::CString::new(self.recycle.to_string())
            .unwrap_or_else(|error| panic!("invalid salvage amount: {error}"));
        draw_salvage_line(
            cx.kernel,
            &amount,
            POINT {
                x: (self.rect.corner.x as i32 + self.rect.extent.width as i32 - 1) as i16,
                y: self.rect.corner.y.wrapping_add(14),
            },
            SalvageText::Amount,
        );
        crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 0, 0, i32::from(bounty));
        let before = self.victory;
        self.victory = self.victory.wrapping_add(1);
        if (before - 1) % 5 != 0 {
            self.stamp_origin.x = self.stamp_origin.x.wrapping_add(17);
        } else {
            self.rect.corner.x = self.salvage_rect.corner.x.wrapping_add(10);
            self.rect.extent.width = 104;
            self.context.fill_rectangle(rgb15(0, 0, 0), self.rect);
            self.stamp_origin.x = self.rect.corner.x.wrapping_add(2);
            self.stamp_origin.y = self.salvage_rect.corner.y.wrapping_add(12);
        }
        self.animate(cx, input);
    }
    fn animate(&mut self, cx: &mut Cx<'_>, input: &crate::engine::input::InputSession) {
        let runtime = cx.kernel.input_runtime();
        if self.sleepy {
            let mut time = GetTimeCounter();
            for (r, g, b) in SHIP_FADE {
                runtime.update(cx);
                self.sleepy = !input.current().any_button_pressed()
                    && cx.world.activity() & CHECK_ABORT as u16 == 0;
                if !self.sleepy {
                    break;
                }
                self.context.draw_filled_stamp(
                    &self.stamp_frame,
                    self.stamp_origin,
                    rgb15(r, g, b),
                );
                runtime.sleep_until(cx, time.wrapping_add(ONE_SECOND / 15));
                time = GetTimeCounter();
            }
        }
        self.context
            .draw_stamp(&self.stamp_frame, self.stamp_origin);
    }
    pub(super) fn finish(
        &mut self,
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        input: &crate::engine::input::InputSession,
        mut wait: impl FnMut(&mut Cx<'_>, &mut Io<'_>, u32),
    ) {
        if self.victory != 0 {
            wait(cx, io, ONE_SECOND * 3);
            if !escape_pressed(input) {
                draw_fade_text(cx, input, &self.str1, &self.str2, 0, &self.salvage_rect);
                if !escape_pressed(input) {
                    self.rect.corner.x = self.salvage_rect.corner.x.wrapping_add(10);
                    self.rect.extent.width = 132;
                    self.context.fill_rectangle(rgb15(0, 0, 0), self.rect);
                    let mut total = format!("{} ", self.recycle).into_bytes();
                    total.extend_from_slice(
                        cx.kernel
                            .game_text((STATUS_STRING_BASE + 1) as usize)
                            .to_bytes(),
                    );
                    let total = std::ffi::CString::new(total)
                        .unwrap_or_else(|error| panic!("invalid salvage total: {error}"));
                    draw_salvage_line(
                        cx.kernel,
                        &total,
                        POINT {
                            x: (self.rect.corner.x as i32 + (self.rect.extent.width as i32 >> 1))
                                as i16,
                            y: self.rect.corner.y.wrapping_add(14),
                        },
                        SalvageText::Total,
                    );
                    self.str1 = cx
                        .kernel
                        .game_text((ENCOUNTER_STRING_BASE + 6) as usize)
                        .to_owned();
                    self.str2 = cx
                        .kernel
                        .game_text((ENCOUNTER_STRING_BASE + 7) as usize)
                        .to_owned();
                    draw_fade_text(cx, input, &self.str1, &self.str2, 1, &self.salvage_rect);
                    wait(cx, io, ONE_SECOND * 2);
                    if !escape_pressed(input) {
                        draw_fade_text(cx, input, &self.str1, &self.str2, 0, &self.salvage_rect);
                    }
                }
            }
            crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{
        battle::{
            frames::FrameSelection, input::InputContext, link_queue::ShipPickerResources, Battle,
        },
        input::{Frame, InputRuntime, InputSession},
        observe::{CampaignView, Observer, Unobserved},
        Game,
    };
    use std::{cell::Cell, rc::Rc};

    #[derive(Default)]
    struct Pacing {
        input: InputSession,
        sleeps: Cell<usize>,
        updates: Cell<usize>,
    }
    impl InputRuntime for Pacing {
        fn input(&self) -> &InputSession {
            &self.input
        }
        fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
            Box::new(Unobserved)
        }
        fn sleep(&self, _: &mut Cx<'_>, _: u32) {
            panic!("salvage uses absolute pacing");
        }
        fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
            self.sleeps.set(self.sleeps.get() + 1);
        }
        fn yield_poll(&self, _: &mut Cx<'_>) {
            panic!("salvage must not poll combat");
        }
        fn flush(&self) {
            self.input.clear_menu_state();
        }
        fn update(&self, _: &mut Cx<'_>) {
            self.updates.set(self.updates.get() + 1);
        }
        fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
            panic!("unexpected quit");
        }
        fn battle_input(&self, _: &InputContext) -> u8 {
            panic!("unexpected battle input");
        }
        fn run(&self, _: &mut Cx<'_>, _: &mut Io<'_>, _: bool, _: &mut dyn Frame) {
            panic!("salvage pacing is synchronous");
        }
    }

    #[test]
    #[serial_test::serial]
    fn owned_salvage_keeps_rewards_icon_rows_fades_and_wait_order() {
        if !crate::native_test_process::own_graphics_process("battle::encount_native::engine::salvage::tests::owned_salvage_keeps_rewards_icon_rows_fades_and_wait_order") { return; }
        use crate::{
            campaign_display_test::CampaignDisplay, graphics::drawable_abi::LoadedDrawable,
            strings::owned_table::LoadedTextTable,
        };
        let mut game = Game::new();
        let display = CampaignDisplay::install(&mut game)
            .unwrap_or_else(|error| panic!("salvage display: {error}"));
        game.kernel.space_context = game.kernel.status_context;
        game.kernel.screen_context = game.kernel.status_context;
        game.kernel.star_con_font = game.kernel.tiny_font;
        game.kernel.configuration.update_preferences(|prefs| {
            prefs.fonts = 1;
            prefs.menu = 2;
        });
        game.kernel.install_game_text(LoadedTextTable::fixture(vec![
            b"A\0".to_vec();
            (ENCOUNTER_STRING_BASE + 8)
                .max(STATUS_STRING_BASE + 6)
                .max(NAVIGATION_STRING_BASE + 4)
                .max(MONTHS_STRING_BASE + 12)
                as usize
        ]));
        game.world.set_activity(IN_HYPERSPACE as u16);
        game.world
            .data
            .flagship
            .module_slots
            .fill(NUM_MODULES as u8);
        game.world.data.flagship.drive_slots.fill(NUM_MODULES as u8);
        game.world.data.flagship.jet_slots.fill(NUM_MODULES as u8);
        game.kernel
            .set_status_message_mode(StatMsgMode_SMM_RES_UNITS);
        let frames =
            LoadedDrawable::create(8, 64, 140, 100).unwrap_or_else(|| panic!("salvage frames"));
        let frame = frames
            .lease()
            .frame(0)
            .unwrap_or_else(|| panic!("salvage frame"));
        game.kernel.flag_stat_frame = frame;
        game.kernel.font_grad_frame = frame;
        let names = LoadedTextTable::fixture(vec![b"vux\0".to_vec()]);
        game.world.retain_fleet_resources(
            1,
            ShipPickerResources::new(names.entry(0), FrameSelection::retain(frame)),
        );
        let fragment = {
            let mut fleet = game.world.npc_built_ships();
            let id = fleet.alloc().unwrap_or_else(|| panic!("salvage fragment"));
            fleet.get_mut(id).species_id = 1;
            *fleet.get(id)
        };
        let pacing = Rc::new(Pacing::default());
        game.kernel.install_input_runtime(pacing.clone());
        let mut battle = Battle::new();
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
        let mut salvage = Salvage::new(cx.kernel, 1);
        let mut drew_armada = false;
        salvage.panel(&mut cx, None, |_, _, side, panel| {
            assert_eq!(side, 1);
            drew_armada = true;
            panel.corner.x = 10;
            panel.corner.y = 20;
            panel.width = 180;
            panel.height = 90;
        });
        assert!(drew_armada);
        salvage.start(&mut cx, &pacing.input, &fragment);
        assert_eq!(
            salvage.buf[..3]
                .iter()
                .map(|&b| b as u8)
                .collect::<Vec<_>>(),
            b"VUX"
        );
        assert_eq!(pacing.sleeps.get(), 7);
        salvage.reward(&mut cx, &pacing.input);
        assert_eq!((salvage.recycle, salvage.victory), (200, 2));
        assert_eq!((salvage.stamp_origin.x, salvage.stamp_origin.y), (22, 32));
        salvage.reward(&mut cx, &pacing.input);
        assert_eq!((salvage.recycle, salvage.stamp_origin.x), (400, 39));
        assert_eq!(cx.world.data.flagship.res_units, 400);
        assert_eq!(pacing.sleeps.get(), 27);
        let mut observer = Unobserved;
        let mut io = Io {
            observer: &mut observer,
        };
        let mut waits = Vec::new();
        salvage.finish(&mut cx, &mut io, &pacing.input, |_, _, duration| {
            waits.push(duration)
        });
        assert_eq!(waits, [ONE_SECOND * 3, ONE_SECOND * 2]);
        assert_eq!(pacing.sleeps.get(), 48);
        let mut pressed = crate::engine::input::frames::InputState::default();
        pressed.menu[KEY_MENU_SELECT as usize] = 1;
        pacing.input.advance(pressed, pressed, 0);
        salvage.reward(&mut cx, &pacing.input);
        assert!(!salvage.sleepy);
        assert_eq!(pacing.sleeps.get(), 48);
        let updates = pacing.updates.get();
        salvage.reward(&mut cx, &pacing.input);
        assert_eq!(pacing.updates.get(), updates);
        salvage.victory = 0;
        salvage.finish(&mut cx, &mut io, &pacing.input, |_, _, _| {
            panic!("no salvage wait on defeat")
        });
        salvage.context.flush();
        drop(salvage);
        game.world.clear_fleet_resources();
        game.kernel.flag_stat_frame = std::ptr::null_mut();
        game.kernel.font_grad_frame = std::ptr::null_mut();
        drop(frames);
        display.finish(game);
    }
}
