//! Supplied restart operations and retained menu resources.
use super::{do_restart::DoRestartState, types::MenuInputState};
use uqm_engine::graphics::{
    context::BatchGuard, context::ContextLease, drawable::ContextFrame, drawable::LoadedDrawable,
    font::FontLease, geometry::Color, geometry::Point, geometry::TextAlign, highlight::Overlay,
};
use uqm_engine::sound::resource_types::LoadedMusic;
use uqm_engine::time::counter_abi::{get_time_counter, ONE_SECOND};
use uqm_engine::{
    battle::Cx,
    input::{self, frames::*},
    observe::{Injected, InputView, Io},
    StoryKey,
};

pub fn seed_new_campaign(automation_seed: u32, clock_seed: impl FnOnce()) {
    if automation_seed != 0 {
        uqm_engine::math::seed_random(automation_seed);
    } else {
        clock_seed();
    }
}

pub trait RestartHost {
    fn melee(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>);
    fn credits(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, victory: bool);
    fn free_game_data(&mut self, cx: &mut Cx<'_>);
}

pub trait RestartMenuOps {
    fn input(&mut self, _view: InputView) -> Injected {
        Injected::default()
    }
    fn get_current_activity(&mut self) -> u16;
    fn set_current_activity(&mut self, value: u16);
    fn get_last_activity(&mut self) -> u16;
    fn set_last_activity(&mut self, value: u16);
    fn set_next_activity(&mut self, value: u16);
    fn get_menu_input(&mut self) -> MenuInputState;
    fn get_time_counter(&mut self) -> u32;
    fn get_utwig_bomb_on_ship(&mut self) -> u8;
    fn set_utwig_bomb_on_ship(&mut self, value: u8);
    fn get_utwig_bomb(&mut self) -> u8;
    fn get_crew_enlisted(&mut self) -> u16;
    fn set_game_paused(&mut self, value: bool);
    fn reinit_race_queues(&mut self);
    fn set_screen_context(&mut self);
    fn fade_screen(&mut self, kind: u32, duration: i16) -> u32;
    fn sleep_thread_until(&mut self, time: u32);
    fn sleep_thread(&mut self, duration: u32);
    fn batch_graphics(&mut self);
    fn unbatch_graphics(&mut self);
    fn clear_drawable(&mut self);
    fn flush_color_xforms(&mut self);
    fn screen_transition(&mut self, kind: i32);
    fn set_bg_color(&mut self, color: Color);
    fn seed_random(&mut self);
    fn load_menu_graphic(&mut self);
    fn destroy_menu_graphic(&mut self);
    fn draw_restart_menu_graphic(&mut self);
    fn draw_restart_menu_state(&mut self, state: u8);
    fn set_menu_sounds(&mut self, first: u16, second: u16);
    fn set_default_menu_repeat_delay(&mut self);
    fn set_transition_source_null(&mut self);
    fn run_do_input(&mut self, reset_input: bool, state: &mut DoRestartState);
    fn has_menu_music(&mut self) -> bool;
    fn load_menu_music(&mut self) -> bool;
    fn play_music(&mut self);
    fn stop_music(&mut self);
    fn destroy_music(&mut self);
    fn fade_music(&mut self, end_volume: u8, duration: i16) -> u32;
    fn create_flash_overlay(&mut self);
    fn flash_process(&mut self);
    fn flash_pause(&mut self);
    fn flash_continue(&mut self);
    fn flash_start(&mut self);
    fn flash_terminate(&mut self);
    fn flash_set_merge_factors(&mut self, start: i32, end: i32, denominator: i32);
    fn flash_set_speed(&mut self, fade_in: u32, on: u32, fade_out: u32, off: u32);
    fn flash_set_frame_time(&mut self, time: u32);
    fn flash_set_state_fade_in(&mut self, duration: u32);
    fn melee(&mut self);
    fn setup_menu(&mut self);
    fn free_game_data(&mut self);
    fn credits(&mut self, victory: bool);
    fn victory(&mut self);
    fn introduction(&mut self);
    fn splash_screen(&mut self);
    fn do_popup_window_msg(&mut self, string_id: u16);
    fn set_player_control(&mut self, player: u8, control: u16);
}

#[derive(Default)]
pub struct RestartMenu {
    music: Option<LoadedMusic>,
    overlay: Option<Overlay>,
    graphic: Option<LoadedDrawable>,
    font: Option<FontLease>,
    context: Option<ContextLease>,
    batch: Option<BatchGuard>,
    playing: bool,
}

impl RestartMenu {
    pub fn operations<'a, 'world, 'observer>(
        &'a mut self,
        cx: &'a mut Cx<'world>,
        io: &'a mut Io<'observer>,
        host: &'a mut dyn RestartHost,
    ) -> impl RestartMenuOps + 'a + use<'a, 'world, 'observer> {
        Operations {
            menu: self,
            cx,
            io,
            host,
        }
    }

    fn context(&self) -> &ContextLease {
        self.context
            .as_ref()
            .unwrap_or_else(|| panic!("restart screen context not selected"))
    }
    fn frame(&self) -> ContextFrame {
        self.graphic
            .as_ref()
            .unwrap_or_else(|| panic!("restart graphic not loaded"))
            .lease()
            .context_frame(0)
    }
    fn overlay(&mut self) -> &mut Overlay {
        self.overlay
            .as_mut()
            .unwrap_or_else(|| panic!("restart overlay not created"))
    }
}
impl Drop for RestartMenu {
    fn drop(&mut self) {
        drop(self.batch.take());
        if self.playing {
            crate::ui::settings::stop_music();
        }
    }
}

struct Operations<'a, 'world, 'observer> {
    menu: &'a mut RestartMenu,
    cx: &'a mut Cx<'world>,
    io: &'a mut Io<'observer>,
    host: &'a mut dyn RestartHost,
}
impl RestartMenuOps for Operations<'_, '_, '_> {
    fn input(&mut self, view: InputView) -> Injected {
        input::admit_observer(self.cx, self.io, view)
    }
    fn get_current_activity(&mut self) -> u16 {
        self.cx.world.activity()
    }
    fn set_current_activity(&mut self, value: u16) {
        self.cx.world.set_activity(value);
    }
    fn get_last_activity(&mut self) -> u16 {
        self.cx.kernel.last_activity
    }
    fn set_last_activity(&mut self, value: u16) {
        self.cx.kernel.last_activity = value;
    }
    fn set_next_activity(&mut self, value: u16) {
        self.cx.world.next_activity = value;
    }
    fn get_menu_input(&mut self) -> MenuInputState {
        let menu = self.cx.kernel.input_controls().pulsed().menu;
        MenuInputState {
            select: menu[KEY_MENU_SELECT as usize] != 0,
            up: menu[KEY_MENU_UP as usize] != 0,
            down: menu[KEY_MENU_DOWN as usize] != 0,
            left: menu[KEY_MENU_LEFT as usize] != 0,
            right: menu[KEY_MENU_RIGHT as usize] != 0,
            mouse_down: uqm_engine::input_port::sdl_input::mouse_button_down(),
        }
    }
    fn get_time_counter(&mut self) -> u32 {
        get_time_counter()
    }
    fn get_utwig_bomb_on_ship(&mut self) -> u8 {
        self.cx.world.story_state(StoryKey::UTWIG_BOMB_ON_SHIP) as u8
    }
    fn set_utwig_bomb_on_ship(&mut self, value: u8) {
        self.cx
            .world
            .set_story_state(StoryKey::UTWIG_BOMB_ON_SHIP, u32::from(value));
    }
    fn get_utwig_bomb(&mut self) -> u8 {
        self.cx.world.story_state(StoryKey::UTWIG_BOMB) as u8
    }
    fn get_crew_enlisted(&mut self) -> u16 {
        self.cx.world.data.flagship.crew_enlisted
    }
    fn set_game_paused(&mut self, value: bool) {
        self.cx.kernel.input_controls().set_paused(value);
    }
    fn reinit_race_queues(&mut self) {
        for side in 0..2 {
            self.cx.kernel.ships(side).reinit();
        }
    }
    fn set_screen_context(&mut self) {
        let context = ContextLease::require(self.cx.kernel.screen_context);
        context.select();
        self.menu.context = Some(context);
    }
    fn fade_screen(&mut self, kind: u32, duration: i16) -> u32 {
        self.menu.context().fade_screen(kind, duration)
    }
    fn sleep_thread_until(&mut self, time: u32) {
        self.cx.kernel.input_runtime().sleep_until(self.cx, time);
    }
    fn sleep_thread(&mut self, duration: u32) {
        self.cx.kernel.input_runtime().sleep(self.cx, duration);
    }
    fn batch_graphics(&mut self) {
        assert!(
            self.menu.batch.is_none(),
            "restart drawing batch already open"
        );
        self.menu.batch = Some(self.menu.context().batch());
    }
    fn unbatch_graphics(&mut self) {
        drop(
            self.menu
                .batch
                .take()
                .unwrap_or_else(|| panic!("restart drawing batch not open")),
        );
    }
    fn clear_drawable(&mut self) {
        self.menu.context().clear();
    }
    fn flush_color_xforms(&mut self) {
        self.menu.context().finish_color_transforms();
    }
    fn screen_transition(&mut self, kind: i32) {
        assert_eq!(kind, 3);
        self.menu.context().transition(None);
    }
    fn set_bg_color(&mut self, color: Color) {
        self.menu.context().set_background(color);
    }
    fn seed_random(&mut self) {
        let seed = self
            .input(InputView::Seed {
                domain: 3,
                fallback: 0,
            })
            .seed
            .unwrap_or(0);
        seed_new_campaign(seed, || {
            crate::ui::util::seed_random_numbers();
        });
    }
    fn load_menu_graphic(&mut self) {
        self.menu.graphic = Some(
            LoadedDrawable::load(c"graphics.newgame").unwrap_or_else(|| panic!("restart graphic")),
        );
        self.menu.font = Some(
            self.cx
                .kernel
                .tiny_font_lease()
                .unwrap_or_else(|| panic!("restart tiny font")),
        );
    }
    fn destroy_menu_graphic(&mut self) {
        self.menu.graphic = None;
        self.menu.font = None;
    }
    fn draw_restart_menu_graphic(&mut self) {
        let context = self.menu.context();
        let frame = self.menu.frame();
        let bounds = frame.rect().extent;
        let (width, height) = context.screen_dimensions();
        context.set_background(Color {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        });
        let version = std::ffi::CString::new(concat!("v", env!("CARGO_PKG_VERSION")))
            .unwrap_or_else(|_| panic!("package version contains NUL"));
        context.with_batch(|| {
            context.clear();
            context.finish_color_transforms();
            context.draw_stamp(
                &frame,
                Point {
                    x: centered_coordinate(width, bounds.width),
                    y: centered_coordinate(height, bounds.height),
                },
            );
            context.draw_text(
                self.menu
                    .font
                    .as_ref()
                    .unwrap_or_else(|| panic!("restart tiny font")),
                &version,
                Point {
                    x: (width - 3) as i16,
                    y: (height - 2) as i16,
                },
                TextAlign::Right,
                Color {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
            );
        });
    }
    fn draw_restart_menu_state(&mut self, state: u8) {
        let frame = self.menu.frame().wrapped_frame(u16::from(state) + 1);
        self.menu.overlay().set_frame(Point { x: 0, y: 0 }, frame);
    }
    fn set_menu_sounds(&mut self, first: u16, second: u16) {
        self.cx
            .kernel
            .input_controls()
            .set_menu_sounds(first, second);
    }
    fn set_default_menu_repeat_delay(&mut self) {
        self.cx.kernel.input_controls().configure_repeat(
            ONE_SECOND / 12,
            ONE_SECOND / 2,
            ONE_SECOND / 12,
            false,
            get_time_counter(),
        );
    }
    fn set_transition_source_null(&mut self) {
        self.menu.context().transition_source(None);
    }
    fn run_do_input(&mut self, reset_input: bool, state: &mut DoRestartState) {
        let menu = &mut *self.menu;
        let host = &mut *self.host;
        self.cx.run_input(self.io, reset_input, |cx, io| {
            let mut ops = Operations { menu, cx, io, host };
            super::do_restart::do_restart_frame(&mut ops, state)
        });
    }
    fn has_menu_music(&mut self) -> bool {
        self.menu.music.is_some()
    }
    fn load_menu_music(&mut self) -> bool {
        self.menu.music = LoadedMusic::load(c"music.mainmenu");
        self.menu.music.is_some()
    }
    fn play_music(&mut self) {
        crate::ui::settings::play_loaded_music(self.cx.world, self.menu.music.as_ref(), 1, 1);
        self.menu.playing = true;
    }
    fn stop_music(&mut self) {
        crate::ui::settings::stop_music();
        self.menu.playing = false;
    }
    fn destroy_music(&mut self) {
        self.menu.music = None;
    }
    fn fade_music(&mut self, end_volume: u8, duration: i16) -> u32 {
        uqm_engine::sound::heart_ffi::fade_music(end_volume, duration)
    }
    fn create_flash_overlay(&mut self) {
        self.menu.overlay = Some(self.menu.context().overlay());
    }
    fn flash_process(&mut self) {
        self.menu.overlay().process();
    }
    fn flash_pause(&mut self) {
        self.menu.overlay().pause();
    }
    fn flash_continue(&mut self) {
        self.menu.overlay().resume();
    }
    fn flash_start(&mut self) {
        self.menu.overlay().start();
    }
    fn flash_terminate(&mut self) {
        self.menu.overlay = None;
    }
    fn flash_set_merge_factors(&mut self, start: i32, end: i32, denominator: i32) {
        self.menu
            .overlay()
            .set_merge_factors(start, end, denominator);
    }
    fn flash_set_speed(&mut self, fade_in: u32, on: u32, fade_out: u32, off: u32) {
        self.menu.overlay().set_speed(fade_in, on, fade_out, off);
    }
    fn flash_set_frame_time(&mut self, time: u32) {
        self.menu.overlay().set_frame_time(time);
    }
    fn flash_set_state_fade_in(&mut self, duration: u32) {
        self.menu.overlay().fade_in(duration);
    }
    fn melee(&mut self) {
        self.host.melee(self.cx, self.io);
    }
    fn setup_menu(&mut self) {
        use crate::front_end::setupmenu::{setup_menu, SetupConfiguration, SetupResources};
        use uqm_engine::{configuration::runtime::RuntimeSettings, resource::settings::Settings};
        let context = self.menu.context().clone();
        let runtime = RuntimeSettings::capture(&context);
        let font = self
            .cx
            .kernel
            .battle_font()
            .unwrap_or_else(|| panic!("setup font not loaded"));
        let resources = SetupResources::load(context, font);
        let configuration = SetupConfiguration {
            input: self.cx.kernel.input_runtime().input().clone(),
            runtime,
            settings: Settings,
            names: uqm_engine::input_port::templates::input_templates(),
        };
        setup_menu(self.cx, self.io, configuration, resources);
    }
    fn free_game_data(&mut self) {
        self.host.free_game_data(self.cx);
    }
    fn credits(&mut self, victory: bool) {
        self.host.credits(self.cx, self.io, victory);
    }
    fn victory(&mut self) {
        crate::front_end::fmv::victory(self.cx, self.io);
    }
    fn introduction(&mut self) {
        crate::front_end::fmv::introduction(self.cx, self.io);
    }
    fn splash_screen(&mut self) {
        crate::front_end::fmv::splash_screen(self.cx, self.io, |_, _| {});
    }
    fn do_popup_window_msg(&mut self, string_id: u16) {
        let index = (crate::starbase::ids::MAINMENU_STRING_BASE as u16).wrapping_add(string_id);
        let text = self.cx.kernel.game_text(usize::from(index)).to_owned();
        crate::ui::confirm::do_popup_window(self.cx, self.io, &text);
    }
    fn set_player_control(&mut self, player: u8, control: u16) {
        self.cx.kernel.player_control[usize::from(player)] = control as u8;
    }
}

fn centered_coordinate(screen: i32, image: i16) -> i16 {
    ((screen - i32::from(image)) >> 1) as i16
}

#[cfg(test)]
mod layout_tests {
    #[test]
    fn centers_restart_frame_with_original_signed_shift_rounding() {
        use super::centered_coordinate;
        assert_eq!(centered_coordinate(320, 320), 0);
        assert_eq!(centered_coordinate(320, 301), 9);
        assert_eq!(centered_coordinate(200, 203), -2);
    }
}
