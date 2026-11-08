use super::*;

impl Frame for StarMap {
    fn admit(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, view: InputView) -> Injected {
        self.observe(io);
        crate::engine::input::admit_observer(cx, io, view)
    }

    fn frame(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>) -> bool {
        let runtime = cx.kernel.input_runtime();
        let keep_going = self.cursor_frame(cx, io, runtime.input());
        if !keep_going {
            self.initialized = false;
        }
        self.observe(io);
        keep_going
    }
}

impl StarMap {
    fn cursor_frame(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, input: &InputSession) -> bool {
        let time_in = GetTimeCounter();
        let activity = cx.world.activity();
        if !self.initialized {
            self.initialized = true;
            self.flash(cx.kernel);
            self.last_label = c"".into();
            self.update_cursor_info(cx.kernel, cx.world);
            self.fuel(cx.kernel, cx.world);
            return true;
        }
        let keys = input.pulsed().menu;
        if keys[10] != 0 {
            return false;
        } else if keys[9] != 0 {
            if !self.select_destination(cx, activity) {
                return false;
            }
        } else if keys[21] != 0 {
            self.search(cx, io, input);
        } else {
            self.move_frame(cx, &keys, time_in);
        }
        self.flash(cx.kernel);
        cx.world.activity() & 0x4000 == 0
    }
    fn select_destination(&mut self, cx: &mut Cx<'_>, activity: u16) -> bool {
        cx.world.data.game.autopilot = self.state.cursor;
        #[cfg(feature = "debug-process")]
        if cx.kernel.instant_move {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 3);
            if activity & 0xff == 3 {
                crate::front_end::uqmdebug::move_flagship(cx.world, self.dimensions());
            } else if activity & 0xff == 4 {
                cx.world.set_activity(activity | 0x0800);
            }
            return false;
        }
        self.draw_map(cx.world, 0, None, true);
        true
    }

    fn search(&mut self, cx: &mut Cx<'_>, io: &mut Io<'_>, input: &InputSession) {
        if which_space(cx.world) <= 1 {
            let old = self.state.cursor;
            if !search::run(self, cx, io, input) {
                self.move_cursor(cx.world, 0, 0, Some(old));
            }
            self.last_label = c"  <random garbage>  ".into();
            self.update_cursor_info(cx.kernel, cx.world);
            self.fuel(cx.kernel, cx.world);
            input.configure_repeat(14, 105, 7, true, GetTimeCounter());
            input.set_menu_sounds(0, 0);
        } else {
            crate::ui::settings::play_menu_sound(cx.kernel, cx.world, 2);
        }
    }

    fn move_frame(&mut self, cx: &mut Cx<'_>, keys: &[i32], time_in: u32) {
        let zoom = if keys[16] != 0 {
            1
        } else if keys[17] != 0 {
            -1
        } else {
            0
        };
        self.zoom(cx, zoom);
        let sx = if keys[8] != 0 {
            1
        } else if keys[7] != 0 {
            -1
        } else {
            0
        };
        let sy = if keys[6] != 0 {
            1
        } else if keys[5] != 0 {
            -1
        } else {
            0
        };
        if sx != 0 || sy != 0 {
            self.move_cursor(cx.world, sx, sy, None);
            self.update_cursor_info(cx.kernel, cx.world);
            self.fuel(cx.kernel, cx.world);
        }
        cx.kernel
            .input_runtime()
            .sleep_until(cx, time_in.wrapping_add(14));
    }
}
