//! Setup menus and persistent options, ported from uqm/setupmenu.c.
#![allow(non_snake_case)]

use crate::navigation::abi::*;
use std::ffi::c_int;

pub type GlobalOptions = GLOBALOPTS;
const GAMMA_CURVE_B: f32 = 2.1972;

#[derive(Clone, Copy, Debug)]
pub struct GammaRange {
    pub min: f32,
    pub max: f32,
    min_x: f32,
    max_x: f32,
}
impl GammaRange {
    pub fn new(min: f32, max: f32) -> Self {
        let mut range = Self {
            min,
            max,
            min_x: 0.0,
            max_x: 0.0,
        };
        range.update_bounds(true);
        range
    }
    fn curve(x: f32) -> f32 {
        (((x - 1.0) * GAMMA_CURVE_B) as f64).exp() as f32
    }
    fn solve(y: f32) -> f32 {
        ((y as f64).ln() / GAMMA_CURVE_B as f64 + 1.0) as f32
    }
    pub fn gamma_to_slider(&self, gamma: f32) -> c_int {
        let x = Self::solve(gamma);
        let step = (self.max_x - self.min_x) / 100.0;
        (((x - self.min_x) / step) as f64 + 0.5) as c_int
    }
    pub fn slider_to_gamma(&self, value: c_int) -> f32 {
        let step = (self.max_x - self.min_x) / 100.0;
        let x = self.min_x + step * value as f32;
        let g = Self::curve(x);
        if ((g - 1.0) as f64).abs() < 0.001_f32 as f64 {
            1.0
        } else {
            g
        }
    }
    pub fn update_bounds(&mut self, use_upper: bool) {
        self.min_x = Self::solve(self.min);
        self.max_x = Self::solve(self.max);
        let slider = self.gamma_to_slider(1.0);
        let g = self.slider_to_gamma(slider);
        if g == 1.0 {
            return;
        }
        let x = Self::solve(g);
        if use_upper {
            let d = (x - 1.0) * 100.0 / slider as f32;
            self.max_x -= d;
            self.max = Self::curve(self.max_x);
        } else {
            let d = (x - 1.0) * 100.0 / (100 - slider) as f32;
            self.min_x -= d;
            self.min = Self::curve(self.min_x);
        }
    }
}
#[cfg(not(test))]
pub(crate) mod runtime {
    use super::*;
    use crate::memory::{rust_hfree as HFree, rust_hmalloc as HMalloc};
    use crate::navigation::globals::GlobData;
    use crate::video::control::{InitVideoPlayer, UninitVideoPlayer};
    use std::{ffi::c_char, mem, ptr};
    const MENU_FRAME_RATE: DWORD = ONE_SECOND / 20;
    const MENU_SOUND_ARROWS: MENU_SOUND_FLAGS = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3);
    const MENU_SOUND_SELECT: MENU_SOUND_FLAGS = 1 << 4;
    static mut GAMMA: GammaRange = GammaRange {
        min: 0.4,
        max: 2.5,
        min_x: 0.0,
        max_x: 0.0,
    };

    #[repr(C)]
    struct SetupMenuState {
        InputFunc: Option<unsafe extern "C" fn(*mut SetupMenuState) -> BOOLEAN>,
        initialized: BOOLEAN,
        anim_frame_count: c_int,
        NextTime: DWORD,
    }
    static mut SETUP_TAB: STRING = ptr::null_mut();
    static mut DONE: BOOLEAN = 0;
    static mut CURRENT: *mut WIDGET = ptr::null_mut();
    static mut NEXT: *mut WIDGET = ptr::null_mut();
    static mut MENUS: [WIDGET_MENU_SCREEN; 8] = unsafe { mem::zeroed() };
    static mut CHOICES: [WIDGET_CHOICE; 24] = unsafe { mem::zeroed() };
    static mut SLIDERS: [WIDGET_SLIDER; 4] = unsafe { mem::zeroed() };
    static mut BUTTONS: [WIDGET_BUTTON; 10] = unsafe { mem::zeroed() };
    static mut LABELS: [WIDGET_LABEL; 4] = unsafe { mem::zeroed() };
    static mut TEXTENTRIES: [WIDGET_TEXTENTRY; 1] = unsafe { mem::zeroed() };
    static mut CONTROLENTRIES: [WIDGET_CONTROLENTRY; 7] = unsafe { mem::zeroed() };
    static mut MENU_CHILDREN: [[*mut WIDGET; 12]; 8] = [[ptr::null_mut(); 12]; 8];
    static mut BANK: *mut stringbank = ptr::null_mut();
    static mut SETUP_FRAME: FRAME = ptr::null_mut();
    const CHOICE_WIDTHS: [c_int; 24] = [
        3, 2, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 2, 3, 3, 2, 3, 3, 3, 2, 2, 2,
    ];

    const fn rgb15(r: u8, g: u8, b: u8) -> Color {
        Color {
            r: (r << 3) | (r >> 2),
            g: (g << 3) | (g >> 2),
            b: (b << 3) | (b >> 2),
            a: 255,
        }
    }

    unsafe fn number_res_options() -> c_int {
        if TFB_SupportsHardwareScaling() != 0 {
            6
        } else {
            2
        }
    }
    unsafe fn enter_menu(index: usize, event: c_int, focus_event: c_int) -> c_int {
        if event != WIDGET_EVENT_SELECT as c_int {
            return 0;
        }
        NEXT = ptr::addr_of_mut!(MENUS[index]).cast();
        ((*NEXT).receiveFocus.unwrap())(NEXT, focus_event);
        1
    }
    unsafe extern "C" fn quit_main_menu(_: *mut WIDGET, event: c_int) -> c_int {
        if event == WIDGET_EVENT_SELECT as c_int {
            NEXT = ptr::null_mut();
            1
        } else {
            0
        }
    }
    unsafe extern "C" fn quit_sub_menu(_: *mut WIDGET, event: c_int) -> c_int {
        enter_menu(0, event, WIDGET_EVENT_SELECT as c_int)
    }
    macro_rules! menu_handler {
        ($name:ident, $index:expr) => {
            unsafe extern "C" fn $name(_: *mut WIDGET, event: c_int) -> c_int {
                enter_menu($index, event, WIDGET_EVENT_DOWN as c_int)
            }
        };
    }
    menu_handler!(do_graphics, 1);
    menu_handler!(do_audio, 2);
    menu_handler!(do_engine, 3);
    menu_handler!(do_resources, 4);
    menu_handler!(do_keyconfig, 5);
    menu_handler!(do_advanced, 6);

    unsafe fn populate_editkeys(template: c_int) {
        let widget = &mut *ptr::addr_of_mut!(TEXTENTRIES[0]);
        libc::strncpy(
            widget.value.as_mut_ptr(),
            input_templates[template as usize].name.as_ptr(),
            widget.maxlen as usize,
        );
        widget.value[(widget.maxlen - 1) as usize] = 0;
        for (i, entry) in std::slice::from_raw_parts_mut(
            ptr::addr_of_mut!(CONTROLENTRIES).cast::<WIDGET_CONTROLENTRY>(),
            7,
        )
        .iter_mut()
        .enumerate()
        {
            for j in 0..2 {
                InterrogateInputState(
                    template,
                    i as c_int,
                    j as c_int,
                    entry.controlname[j].as_mut_ptr(),
                    WIDGET_CONTROLENTRY_WIDTH as c_int,
                );
            }
        }
    }
    unsafe extern "C" fn do_editkeys(_: *mut WIDGET, event: c_int) -> c_int {
        if event != WIDGET_EVENT_SELECT as c_int {
            return 0;
        }
        NEXT = ptr::addr_of_mut!(MENUS[7]).cast();
        CHOICES[20].selected = 0;
        populate_editkeys(0);
        ((*NEXT).receiveFocus.unwrap())(NEXT, WIDGET_EVENT_DOWN as c_int);
        1
    }
    unsafe extern "C" fn change_template(widget: *mut WIDGET_CHOICE, _: c_int) {
        populate_editkeys((*widget).selected);
    }
    unsafe extern "C" fn rename_template(widget: *mut WIDGET_TEXTENTRY) {
        let name = input_templates[CHOICES[20].selected as usize]
            .name
            .as_mut_ptr();
        libc::strncpy(name, (*widget).value.as_ptr(), 30);
        *name.add(29) = 0;
    }

    macro_rules! option_choices {
        ($options:ident, $action:ident) => {
            $action!($options, res, 0);
            $action!($options, driver, 1);
            $action!($options, scaler, 2);
            $action!($options, scanlines, 3);
            $action!($options, menu, 4);
            $action!($options, text, 5);
            $action!($options, cscan, 6);
            $action!($options, scroll, 7);
            $action!($options, subtitles, 8);
            $action!($options, music3do, 9);
            $action!($options, fullscreen, 10);
            $action!($options, intro, 11);
            $action!($options, fps, 12);
            $action!($options, meleezoom, 13);
            $action!($options, stereo, 14);
            $action!($options, adriver, 15);
            $action!($options, aquality, 16);
            $action!($options, shield, 17);
            $action!($options, player1, 18);
            $action!($options, player2, 19);
            $action!($options, musicremix, 21);
            $action!($options, speech, 22);
            $action!($options, keepaspect, 23);
        };
    }
    macro_rules! to_choice {
        ($opts:ident, $field:ident, $i:expr) => {
            CHOICES[$i].selected = $opts.$field as c_int
        };
    }
    macro_rules! from_choice {
        ($opts:ident, $field:ident, $i:expr) => {
            $opts.$field = CHOICES[$i].selected as _
        };
    }
    unsafe fn SetDefaults() {
        let mut opts: GLOBALOPTS = mem::zeroed();
        GetGlobalOptions(&mut opts);
        CHOICES[0].numopts =
            number_res_options() + c_int::from(opts.res == OPT_RESTYPE_OPTVAL_CUSTOM);
        option_choices!(opts, to_choice);
        CHOICES[20].selected = 0;
        SLIDERS[0].value = opts.musicvol;
        SLIDERS[1].value = opts.sfxvol;
        SLIDERS[2].value = opts.speechvol;
        SLIDERS[3].value = opts.gamma;
    }
    unsafe fn PropagateResults() {
        let mut opts: GLOBALOPTS = mem::zeroed();
        option_choices!(opts, from_choice);
        opts.musicvol = SLIDERS[0].value;
        opts.sfxvol = SLIDERS[1].value;
        opts.speechvol = SLIDERS[2].value;
        opts.gamma = SLIDERS[3].value;
        SetGlobalOptions(&mut opts);
    }
    unsafe extern "C" fn DoSetupMenu(state: *mut SetupMenuState) -> BOOLEAN {
        GamePaused = 0;
        if (*state).initialized == 0 {
            SetDefaultMenuRepeatDelay();
            (*state).NextTime = GetTimeCounter();
            SetDefaults();
            Widget_SetFont(StarConFont);
            Widget_SetWindowColors(
                rgb15(0x14, 0x14, 0x14),
                rgb15(8, 8, 8),
                rgb15(0x10, 0x10, 0x10),
            );
            CURRENT = ptr::null_mut();
            NEXT = ptr::addr_of_mut!(MENUS[0]).cast();
            ((*NEXT).receiveFocus.unwrap())(NEXT, WIDGET_EVENT_DOWN as c_int);
            (*state).initialized = 1;
        }
        if CURRENT != NEXT {
            SetTransitionSource(ptr::null());
        }
        BatchGraphics();
        ((*NEXT).draw.unwrap())(NEXT, 0, 0);
        if CURRENT != NEXT {
            ScreenTransition(3, ptr::null());
            CURRENT = NEXT;
        }
        UnbatchGraphics();
        for (key, event) in [
            (KEY_MENU_UP, WIDGET_EVENT_UP),
            (KEY_MENU_DOWN, WIDGET_EVENT_DOWN),
            (KEY_MENU_LEFT, WIDGET_EVENT_LEFT),
            (KEY_MENU_RIGHT, WIDGET_EVENT_RIGHT),
        ] {
            if PulsedInputState.menu[key as usize] != 0 {
                Widget_Event(event as c_int);
                break;
            }
        }
        for (key, event) in [
            (KEY_MENU_SELECT, WIDGET_EVENT_SELECT),
            (KEY_MENU_CANCEL, WIDGET_EVENT_CANCEL),
            (KEY_MENU_DELETE, WIDGET_EVENT_DELETE),
        ] {
            if PulsedInputState.menu[key as usize] != 0 {
                Widget_Event(event as c_int);
            }
        }
        SleepThreadUntil((*state).NextTime.wrapping_add(MENU_FRAME_RATE));
        (*state).NextTime = GetTimeCounter();
        BOOLEAN::from(
            GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0 && !NEXT.is_null(),
        )
    }
    unsafe fn redraw_menu() {
        BatchGraphics();
        ((*NEXT).draw.unwrap())(NEXT, 0, 0);
        UnbatchGraphics();
    }
    unsafe extern "C" fn OnTextEntryChange(state: *mut TEXTENTRY_STATE) -> BOOLEAN {
        let widget = (*state).CbParam.cast::<WIDGET_TEXTENTRY>();
        (*widget).cursor_pos = (*state).CursorPos;
        if (*state).JoystickMode != 0 {
            (*widget).state |= WIDGET_TEXTENTRY_STATE_WTE_BLOCKCUR;
        } else {
            (*widget).state &= !WIDGET_TEXTENTRY_STATE_WTE_BLOCKCUR;
        }
        1
    }
    unsafe extern "C" fn OnTextEntryFrame(state: *mut TEXTENTRY_STATE) -> BOOLEAN {
        redraw_menu();
        SleepThreadUntil((*state).NextTime);
        (*state).NextTime = GetTimeCounter().wrapping_add(MENU_FRAME_RATE);
        1
    }
    unsafe extern "C" fn OnTextEntryEvent(widget: *mut WIDGET_TEXTENTRY) -> c_int {
        (*widget).cursor_pos = utf8StringCount(((*widget).value.as_ptr()).cast()) as c_int;
        (*widget).state = WIDGET_TEXTENTRY_STATE_WTE_EDITING;
        redraw_menu();
        let mut revert_buf = [0; 256];
        utf8StringCopy(
            revert_buf.as_mut_ptr(),
            mem::size_of_val(&revert_buf),
            ((*widget).value.as_ptr()).cast(),
        );
        let mut state: TEXTENTRY_STATE = mem::zeroed();
        state.Initialized = 0;
        state.NextTime = GetTimeCounter().wrapping_add(MENU_FRAME_RATE);
        state.BaseStr = (*widget).value.as_mut_ptr();
        state.MaxSize = (*widget).maxlen;
        state.CursorPos = (*widget).cursor_pos;
        state.CbParam = widget.cast();
        state.ChangeCallback = Some(OnTextEntryChange);
        state.FrameCallback = Some(OnTextEntryFrame);
        SetMenuSounds(0, MENU_SOUND_SELECT);
        if DoTextEntry(&mut state) == 0 {
            utf8StringCopy(
                ((*widget).value.as_mut_ptr()).cast(),
                (*widget).maxlen as usize,
                revert_buf.as_ptr(),
            );
        } else if let Some(callback) = (*widget).onChange {
            callback(widget);
        }
        SetMenuSounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        (*widget).state = WIDGET_TEXTENTRY_STATE_WTE_NORMAL;
        redraw_menu();
        1
    }
    unsafe extern "C" fn gamma_HandleEventSlider(widget: *mut WIDGET, event: c_int) -> c_int {
        let slider = widget.cast::<WIDGET_SLIDER>();
        let previous = (*slider).value;
        if event == WIDGET_EVENT_LEFT as c_int {
            (*slider).value -= (*slider).step;
        } else if event == WIDGET_EVENT_RIGHT as c_int {
            (*slider).value += (*slider).step;
        } else {
            return 0;
        }
        let range = &mut *ptr::addr_of_mut!(GAMMA);
        let mut gamma = range.slider_to_gamma((*slider).value);
        let set = TFB_SetGamma(gamma);
        if !set {
            (*slider).value = previous;
            gamma = range.slider_to_gamma((*slider).value);
        }
        if gamma < range.min || (!set && event == WIDGET_EVENT_LEFT as c_int) {
            range.min = gamma;
            range.update_bounds(true);
            (*slider).value = 0;
        } else if gamma > range.max || (!set && event == WIDGET_EVENT_RIGHT as c_int) {
            range.max = gamma;
            range.update_bounds(false);
            (*slider).value = 100;
        }
        1
    }
    unsafe extern "C" fn gamma_DrawValue(slider: *mut WIDGET_SLIDER, x: c_int, y: c_int) {
        let gamma = (&*ptr::addr_of!(GAMMA)).slider_to_gamma((*slider).value);
        let mut buffer = [0 as c_char; 16];
        libc::snprintf(
            buffer.as_mut_ptr(),
            buffer.len(),
            c"%.4f".as_ptr(),
            gamma as f64,
        );
        let mut text: TEXT = mem::zeroed();
        text.baseline = POINT {
            x: x as _,
            y: y as _,
        };
        text.align = TEXT_ALIGN_ALIGN_CENTER;
        text.CharCount = COUNT::MAX;
        text.pStr = buffer.as_ptr();
        font_DrawText(&mut text);
    }
    unsafe extern "C" fn rebind_control(widget: *mut WIDGET_CONTROLENTRY) {
        let template = CHOICES[20].selected;
        FlushInput();
        DrawLabelAsWindow(ptr::addr_of_mut!(LABELS[3]), ptr::null_mut());
        RebindInputState(template, (*widget).controlindex, (*widget).highlighted);
        populate_editkeys(template);
        FlushInput();
    }
    unsafe extern "C" fn clear_control(widget: *mut WIDGET_CONTROLENTRY) {
        let template = CHOICES[20].selected;
        RemoveInputState(template, (*widget).controlindex, (*widget).highlighted);
        populate_editkeys(template);
    }

    macro_rules! widget_header {
        ($widget:expr, $tag:ident, $event:ident, $focus:ident, $draw:ident, $height:ident) => {{
            let w = ($widget as *mut _ as *mut WIDGET);
            (*w).tag = $tag;
            (*w).parent = ptr::null_mut();
            (*w).handleEvent = Some($event);
            (*w).receiveFocus = Some($focus);
            (*w).draw = Some($draw);
            (*w).height = Some($height);
            (*w).width = Some(Widget_WidthFullScreen);
        }};
    }
    unsafe fn fatal(message: &std::ffi::CStr) -> ! {
        // The setup table is an external resource; C terminates on malformed input.
        crate::log_abi::log_message(1, &message.to_string_lossy());
        libc::exit(libc::EXIT_FAILURE)
    }
    unsafe fn table_string(index: c_int) -> *const c_char {
        GetStringAddress(SetAbsStringTableIndex(SETUP_TAB, index as COUNT))
    }
    unsafe fn split_entry(
        index: &mut c_int,
        count: c_int,
        buffer: &mut [*const c_char; 100],
        message: &std::ffi::CStr,
    ) -> c_int {
        if *index >= count {
            fatal(message);
        }
        let string = table_string(*index);
        *index += 1;
        SplitString(string, b'\n' as c_char, 100, buffer.as_mut_ptr(), BANK)
    }
    unsafe fn init_widgets() {
        let mut buffer = [ptr::null(); 100];
        if BANK.is_null() {
            BANK = StringBank_Create();
        }
        if SETUP_FRAME.is_null() {
            SETUP_FRAME =
                CaptureDrawable(LoadGraphicInstance(MENUBKG_PMAP_ANIM.as_ptr().cast()).cast());
        }
        let count = GetStringTableCount(SETUP_TAB) as c_int;
        if count < 3 {
            fatal(c"PANIC: Setup string table too short to even hold all indices!");
        }
        let title = StringBank_AddOrFindString(BANK, table_string(0));
        if SplitString(
            table_string(1),
            b'\n' as c_char,
            100,
            buffer.as_mut_ptr(),
            BANK,
        ) != 8
        {
            fatal(c"PANIC: Incorrect number of Menu Subtitles");
        }
        macro_rules! b {
            ($i:expr) => {
                ptr::addr_of_mut!(BUTTONS[$i]).cast::<WIDGET>()
            };
        }
        macro_rules! c {
            ($i:expr) => {
                ptr::addr_of_mut!(CHOICES[$i]).cast::<WIDGET>()
            };
        }
        macro_rules! s {
            ($i:expr) => {
                ptr::addr_of_mut!(SLIDERS[$i]).cast::<WIDGET>()
            };
        }
        macro_rules! l {
            ($i:expr) => {
                ptr::addr_of_mut!(LABELS[$i]).cast::<WIDGET>()
            };
        }
        let children: [&[*mut WIDGET]; 8] = [
            &[b!(2), b!(3), b!(4), b!(5), b!(6), b!(7), b!(0)],
            &[c!(0), c!(23), c!(10), s!(3), c!(2), c!(3), b!(1)],
            &[s!(0), s!(1), s!(2), c!(14), c!(9), c!(21), c!(22), b!(1)],
            &[
                c!(4),
                c!(5),
                c!(6),
                c!(7),
                c!(8),
                c!(13),
                c!(11),
                c!(17),
                b!(1),
            ],
            &[l!(0), b!(1)],
            &[c!(18), c!(19), l!(1), b!(8), b!(1)],
            &[c!(12), c!(15), c!(16), b!(1)],
            &[
                c!(20),
                l!(2),
                ptr::addr_of_mut!(TEXTENTRIES[0]).cast(),
                ptr::addr_of_mut!(CONTROLENTRIES[0]).cast(),
                ptr::addr_of_mut!(CONTROLENTRIES[1]).cast(),
                ptr::addr_of_mut!(CONTROLENTRIES[2]).cast(),
                ptr::addr_of_mut!(CONTROLENTRIES[3]).cast(),
                ptr::addr_of_mut!(CONTROLENTRIES[4]).cast(),
                ptr::addr_of_mut!(CONTROLENTRIES[5]).cast(),
                ptr::addr_of_mut!(CONTROLENTRIES[6]).cast(),
                b!(9),
            ],
        ];
        for i in 0..8 {
            MENU_CHILDREN[i] = [ptr::null_mut(); 12];
            MENU_CHILDREN[i][..children[i].len()].copy_from_slice(children[i]);
            let m = ptr::addr_of_mut!(MENUS[i]);
            widget_header!(
                m,
                WIDGET_TYPE_WIDGET_TYPE_MENU_SCREEN,
                Widget_HandleEventMenuScreen,
                Widget_ReceiveFocusMenuScreen,
                Widget_DrawMenuScreen,
                Widget_HeightFullScreen
            );
            (*m).title = title;
            (*m).subtitle = buffer[i];
            (*m).bgStamp.origin = POINT { x: 0, y: 0 };
            (*m).bgStamp.frame = SetAbsFrameIndex(SETUP_FRAME, [0, 1, 1, 2, 3, 1, 2, 1][i]);
            (*m).num_children = children[i].len() as c_int;
            (*m).child = MENU_CHILDREN[i].as_mut_ptr();
            (*m).highlighted = 0;
        }
        if SplitString(
            table_string(2),
            b'\n' as c_char,
            100,
            buffer.as_mut_ptr(),
            BANK,
        ) != 24
        {
            fatal(c"PANIC: Incorrect number of Choice Options");
        }
        for i in 0..24 {
            let c = ptr::addr_of_mut!(CHOICES[i]);
            widget_header!(
                c,
                WIDGET_TYPE_WIDGET_TYPE_CHOICE,
                Widget_HandleEventChoice,
                Widget_ReceiveFocusChoice,
                Widget_DrawChoice,
                Widget_HeightChoice
            );
            (*c).category = buffer[i];
            (*c).numopts = 0;
            (*c).options = ptr::null_mut();
            (*c).selected = 0;
            (*c).highlighted = 0;
            (*c).maxcolumns = CHOICE_WIDTHS[i];
            (*c).onChange = None;
        }
        let mut index = 3;
        for choice in
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(CHOICES).cast::<WIDGET_CHOICE>(), 24)
        {
            let n = split_entry(
                &mut index,
                count,
                &mut buffer,
                c"PANIC: String table cut short while reading choices",
            );
            choice.numopts = n;
            choice.options = HMalloc(n as usize * mem::size_of::<CHOICE_OPTION>()).cast();
            for (j, &name) in buffer.iter().take(n as usize).enumerate() {
                let option = &mut *choice.options.add(j);
                option.optname = name;
                option.tooltip = [c"".as_ptr(); 3];
            }
            for j in 0..n as usize {
                let tips = split_entry(
                    &mut index,
                    count,
                    &mut buffer,
                    c"PANIC: String table cut short while reading choices",
                )
                .min(3);
                let option = &mut *choice.options.add(j);
                option.tooltip[..tips as usize].copy_from_slice(&buffer[..tips as usize]);
            }
        }
        CHOICES[0].numopts = number_res_options();
        for (i, template) in
            std::slice::from_raw_parts(ptr::addr_of!(input_templates).cast::<INPUT_TEMPLATE>(), 6)
                .iter()
                .enumerate()
        {
            (*CHOICES[18].options.add(i)).optname = template.name.as_ptr();
            (*CHOICES[19].options.add(i)).optname = template.name.as_ptr();
            (*CHOICES[20].options.add(i)).optname = template.name.as_ptr();
        }
        CHOICES[20].onChange = Some(change_template);
        if split_entry(
            &mut index,
            count,
            &mut buffer,
            c"PANIC: String table cut short while reading sliders",
        ) != 4
        {
            fatal(c"PANIC: Incorrect number of Slider Options");
        }
        for i in 0..4 {
            let s = ptr::addr_of_mut!(SLIDERS[i]);
            widget_header!(
                s,
                WIDGET_TYPE_WIDGET_TYPE_SLIDER,
                Widget_HandleEventSlider,
                Widget_ReceiveFocusSimple,
                Widget_DrawSlider,
                Widget_HeightOneLine
            );
            (*s).draw_value = Some(Widget_Slider_DrawValue);
            (*s).min = 0;
            (*s).max = 100;
            (*s).step = 5;
            (*s).value = 75;
            (*s).category = buffer[i];
            (*s).tooltip = [c"".as_ptr(); 3];
        }
        SLIDERS[3].step = 1;
        SLIDERS[3].handleEvent = Some(gamma_HandleEventSlider);
        SLIDERS[3].draw_value = Some(gamma_DrawValue);
        for slider in
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(SLIDERS).cast::<WIDGET_SLIDER>(), 4)
        {
            let tips = split_entry(
                &mut index,
                count,
                &mut buffer,
                c"PANIC: String table cut short while reading sliders",
            )
            .min(3);
            slider.tooltip[..tips as usize].copy_from_slice(&buffer[..tips as usize]);
        }
        if split_entry(
            &mut index,
            count,
            &mut buffer,
            c"PANIC: String table cut short while reading buttons",
        ) != 10
        {
            fatal(c"PANIC: Incorrect number of Button Options");
        }
        let handlers: [unsafe extern "C" fn(*mut WIDGET, c_int) -> c_int; 10] = [
            quit_main_menu,
            quit_sub_menu,
            do_graphics,
            do_engine,
            do_audio,
            do_resources,
            do_keyconfig,
            do_advanced,
            do_editkeys,
            do_keyconfig,
        ];
        for i in 0..10 {
            let b = ptr::addr_of_mut!(BUTTONS[i]);
            widget_header!(
                b,
                WIDGET_TYPE_WIDGET_TYPE_BUTTON,
                quit_main_menu,
                Widget_ReceiveFocusSimple,
                Widget_DrawButton,
                Widget_HeightOneLine
            );
            (*b).handleEvent = Some(handlers[i]);
            (*b).name = buffer[i];
            (*b).tooltip = [c"".as_ptr(); 3];
        }
        for button in
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(BUTTONS).cast::<WIDGET_BUTTON>(), 10)
        {
            let tips = split_entry(
                &mut index,
                count,
                &mut buffer,
                c"PANIC: String table cut short while reading buttons",
            )
            .min(3);
            button.tooltip[..tips as usize].copy_from_slice(&buffer[..tips as usize]);
        }
        if split_entry(
            &mut index,
            count,
            &mut buffer,
            c"PANIC: String table cut short while reading labels",
        ) != 4
        {
            fatal(c"PANIC: Incorrect number of Label Options");
        }
        for label in
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(LABELS).cast::<WIDGET_LABEL>(), 4)
        {
            let l = label as *mut WIDGET_LABEL;
            widget_header!(
                l,
                WIDGET_TYPE_WIDGET_TYPE_LABEL,
                Widget_HandleEventIgnoreAll,
                Widget_ReceiveFocusRefuseFocus,
                Widget_DrawLabel,
                Widget_HeightLabel
            );
            (*l).line_count = 0;
            (*l).lines = ptr::null_mut();
        }
        for label in
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(LABELS).cast::<WIDGET_LABEL>(), 4)
        {
            let lines = split_entry(
                &mut index,
                count,
                &mut buffer,
                c"PANIC: String table cut short while reading labels",
            );
            label.line_count = lines;
            label.lines = HMalloc(lines as usize * mem::size_of::<*const c_char>()).cast();
            for (j, line) in buffer.iter().enumerate().take(lines as usize) {
                *label.lines.add(j) = *line;
            }
        }
        if split_entry(
            &mut index,
            count,
            &mut buffer,
            c"PANIC: String table cut short while reading text entries",
        ) != 1
        {
            fatal(c"PANIC: Incorrect number of Text Entries");
        }
        let t = ptr::addr_of_mut!(TEXTENTRIES[0]);
        widget_header!(
            t,
            WIDGET_TYPE_WIDGET_TYPE_TEXTENTRY,
            Widget_HandleEventTextEntry,
            Widget_ReceiveFocusSimple,
            Widget_DrawTextEntry,
            Widget_HeightOneLine
        );
        (*t).handleEventSelect = Some(OnTextEntryEvent);
        (*t).category = buffer[0];
        (*t).value[0] = 0;
        (*t).maxlen = WIDGET_TEXTENTRY_WIDTH as c_int - 1;
        (*t).state = WIDGET_TEXTENTRY_STATE_WTE_NORMAL;
        (*t).cursor_pos = 0;
        if split_entry(
            &mut index,
            count,
            &mut buffer,
            c"PANIC: String table cut short while reading text entries",
        ) != 1
        {
            fatal(c"PANIC: Incorrect number of Text Entries");
        }
        libc::strncpy((*t).value.as_mut_ptr(), buffer[0], (*t).maxlen as usize);
        (*t).value[(*t).maxlen as usize] = 0;
        (*t).onChange = Some(rename_template);
        if split_entry(
            &mut index,
            count,
            &mut buffer,
            c"PANIC: String table cut short while reading control entries",
        ) != 7
        {
            fatal(c"PANIC: Incorrect number of Control Entries");
        }
        for i in 0..7 {
            let c = ptr::addr_of_mut!(CONTROLENTRIES[i]);
            widget_header!(
                c,
                WIDGET_TYPE_WIDGET_TYPE_CONTROLENTRY,
                Widget_HandleEventControlEntry,
                Widget_ReceiveFocusControlEntry,
                Widget_DrawControlEntry,
                Widget_HeightOneLine
            );
            (*c).category = buffer[i];
            (*c).highlighted = 0;
            (*c).controlname[0][0] = 0;
            (*c).controlname[1][0] = 0;
            (*c).controlindex = i as c_int;
            (*c).onChange = Some(rebind_control);
            (*c).onDelete = Some(clear_control);
        }
        if index < count {
            crate::log_abi::log_message(
                3,
                &format!(
                    "WARNING: Setup strings had {} garbage entries at the end.",
                    count - index
                ),
            );
        }
    }
    unsafe fn clean_up_widgets() {
        for choice in
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(CHOICES).cast::<WIDGET_CHOICE>(), 24)
        {
            if !choice.options.is_null() {
                HFree(choice.options.cast());
            }
        }
        for label in
            std::slice::from_raw_parts_mut(ptr::addr_of_mut!(LABELS).cast::<WIDGET_LABEL>(), 4)
        {
            if !label.lines.is_null() {
                HFree(label.lines.cast());
            }
        }
        if !SETUP_TAB.is_null() {
            DestroyStringTable(ReleaseStringTable(SETUP_TAB));
            SETUP_TAB = ptr::null_mut();
        }
        if !BANK.is_null() {
            StringBank_Free(BANK);
            BANK = ptr::null_mut();
        }
        if !SETUP_FRAME.is_null() {
            DestroyDrawable(ReleaseDrawable(SETUP_FRAME));
            SETUP_FRAME = ptr::null_mut();
        }
    }

    /// # Safety
    /// Must run on the game thread with graphics, input and resources initialized.
    #[no_mangle]
    pub unsafe extern "C" fn SetupMenu() {
        let mut state: SetupMenuState = mem::zeroed();
        state.InputFunc = Some(DoSetupMenu);
        state.initialized = 0;
        SetMenuSounds(MENU_SOUND_ARROWS, MENU_SOUND_SELECT);
        SETUP_TAB = CaptureStringTable(LoadStringTableInstance(SETUP_MENU_STRTAB.as_ptr().cast()));
        if !SETUP_TAB.is_null() {
            init_widgets();
        } else {
            fatal(c"PANIC: Could not find strings for the setup menu!");
        }
        DONE = 0;
        DoInput(ptr::addr_of_mut!(state).cast(), 1);
        GlobData.Game_state.CurrentActivity &= !(CHECK_ABORT as u16);
        PropagateResults();
        if !SETUP_TAB.is_null() {
            clean_up_widgets();
        }
    }

    /// # Safety
    /// `opts` must be writable and access to the live options must be serialized.
    #[no_mangle]
    pub unsafe extern "C" fn GetGlobalOptions(opts: *mut GLOBALOPTS) {
        let opts = &mut *opts;
        opts.scaler = if GfxFlags & TFB_GFXFLAGS_SCALE_XBRZ3 as c_int != 0 {
            OPT_SCALETYPE_OPTVAL_XBRZ3_SCALE
        } else if GfxFlags & TFB_GFXFLAGS_SCALE_XBRZ4 as c_int != 0 {
            OPT_SCALETYPE_OPTVAL_XBRZ4_SCALE
        } else if GfxFlags & TFB_GFXFLAGS_SCALE_HQXX as c_int != 0 {
            OPT_SCALETYPE_OPTVAL_HQXX_SCALE
        } else {
            OPT_SCALETYPE_OPTVAL_XBRZ3_SCALE
        };
        opts.fullscreen = u32::from(GfxFlags & TFB_GFXFLAGS_FULLSCREEN as c_int != 0);
        opts.subtitles = u32::from(optSubtitles != 0);
        opts.scanlines = u32::from(GfxFlags & TFB_GFXFLAGS_SCANLINES as c_int != 0);
        opts.menu = u32::from(optWhichMenu == OPT_3DO as c_int);
        opts.text = u32::from(optWhichFonts == OPT_3DO as c_int);
        opts.cscan = u32::from(optWhichCoarseScan == OPT_3DO as c_int);
        opts.scroll = u32::from(optSmoothScroll == OPT_3DO as c_int);
        opts.intro = u32::from(optWhichIntro == OPT_3DO as c_int);
        opts.shield = u32::from(optWhichShield == OPT_3DO as c_int);
        opts.fps = u32::from(GfxFlags & TFB_GFXFLAGS_SHOWFPS as c_int != 0);
        opts.meleezoom = u32::from(optMeleeScale != SCALE_TFB_SCALE_STEP as c_int);
        opts.stereo = u32::from(optStereoSFX != 0);
        opts.music3do = u32::from(opt3doMusic != 0);
        opts.musicremix = u32::from(optRemixMusic != 0);
        opts.speech = u32::from(optSpeech != 0);
        opts.keepaspect = u32::from(optKeepAspectRatio != 0);
        opts.adriver = if snddriver == audio_DRIVER_OPENAL as c_int {
            OPT_ADRIVERTYPE_OPTVAL_OPENAL
        } else if snddriver == audio_DRIVER_MIXSDL as c_int {
            OPT_ADRIVERTYPE_OPTVAL_MIXSDL
        } else {
            OPT_ADRIVERTYPE_OPTVAL_SILENCE
        };
        opts.aquality = if soundflags & audio_QUALITY_HIGH as c_int != 0 {
            OPT_AQUALITYTYPE_OPTVAL_HIGH
        } else if soundflags & audio_QUALITY_LOW as c_int != 0 {
            OPT_AQUALITYTYPE_OPTVAL_LOW
        } else {
            OPT_AQUALITYTYPE_OPTVAL_MEDIUM
        };
        opts.driver = OPT_DRIVERTYPE_OPTVAL_PURE_IF_POSSIBLE;
        opts.res = match ScreenWidthActual {
            320 | 640 => {
                let (height, res) = if ScreenWidthActual == 320 {
                    (240, OPT_RESTYPE_OPTVAL_320_240)
                } else {
                    (480, OPT_RESTYPE_OPTVAL_640_480)
                };
                if GraphicsDriver == TFB_GFXDRIVER_SDL_PURE as c_int {
                    res
                } else if ScreenHeightActual != height {
                    OPT_RESTYPE_OPTVAL_CUSTOM
                } else {
                    opts.driver = OPT_DRIVERTYPE_OPTVAL_ALWAYS_GL;
                    res
                }
            }
            800 if ScreenHeightActual == 600 => OPT_RESTYPE_OPTVAL_800_600,
            1024 if ScreenHeightActual == 768 => OPT_RESTYPE_OPTVAL_1024_768,
            1280 if ScreenHeightActual == 960 => OPT_RESTYPE_OPTVAL_1280_960,
            960 if ScreenHeightActual == 720 => OPT_RESTYPE_OPTVAL_960_720,
            _ => OPT_RESTYPE_OPTVAL_CUSTOM,
        };
        if res_IsBoolean(c"config.alwaysgl".as_ptr()) != 0 {
            opts.driver = if res_GetBoolean(c"config.alwaysgl".as_ptr()) != 0 {
                OPT_DRIVERTYPE_OPTVAL_ALWAYS_GL
            } else {
                OPT_DRIVERTYPE_OPTVAL_PURE_IF_POSSIBLE
            };
        }
        let range = &mut *ptr::addr_of_mut!(GAMMA);
        let which_bound = optGamma < range.max;
        if optGamma <= range.min {
            range.min = optGamma - 0.03;
        } else if optGamma >= range.max {
            range.max = optGamma + 0.3;
        }
        range.update_bounds(which_bound);
        opts.gamma = range.gamma_to_slider(optGamma);
        let controls = ptr::addr_of!(PlayerControls).cast::<CONTROL_TEMPLATE>();
        opts.player1 = controls.read();
        opts.player2 = controls.add(1).read();
        opts.musicvol = ((musicVolumeScale * 100.0) as c_int + 2) / 5 * 5;
        opts.sfxvol = ((sfxVolumeScale * 100.0) as c_int + 2) / 5 * 5;
        opts.speechvol = ((speechVolumeScale * 100.0) as c_int + 2) / 5 * 5;
    }

    /// # Safety
    /// `opts` must point to initialized options. Game, graphics and resource access
    /// must be serialized on the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn SetGlobalOptions(opts: *mut GLOBALOPTS) {
        let opts = &*opts;
        let mut flags = GfxFlags & !(TFB_GFXFLAGS_SCALE_ANY as c_int);
        let mut width = ScreenWidthActual;
        let mut height = ScreenHeightActual;
        let mut driver = GraphicsDriver;
        match opts.res {
            OPT_RESTYPE_OPTVAL_320_240 | OPT_RESTYPE_OPTVAL_640_480 => {
                (width, height) = if opts.res == OPT_RESTYPE_OPTVAL_320_240 {
                    (320, 240)
                } else {
                    (640, 480)
                };
                driver = TFB_GFXDRIVER_SDL_PURE as c_int;
            }
            OPT_RESTYPE_OPTVAL_800_600 => {
                width = 800;
                height = 600;
                driver = TFB_GFXDRIVER_SDL_OPENGL as c_int;
            }
            OPT_RESTYPE_OPTVAL_1024_768 => {
                width = 1024;
                height = 768;
                driver = TFB_GFXDRIVER_SDL_OPENGL as c_int;
            }
            OPT_RESTYPE_OPTVAL_1280_960 => {
                width = 1280;
                height = 960;
                driver = TFB_GFXDRIVER_SDL_OPENGL as c_int;
            }
            OPT_RESTYPE_OPTVAL_960_720 => {
                width = 960;
                height = 720;
                driver = TFB_GFXDRIVER_SDL_OPENGL as c_int;
            }
            _ => {}
        }
        res_PutInteger(c"config.reswidth".as_ptr(), width);
        res_PutInteger(c"config.resheight".as_ptr(), height);
        res_PutBoolean(
            c"config.alwaysgl".as_ptr(),
            c_int::from(opts.driver == OPT_DRIVERTYPE_OPTVAL_ALWAYS_GL),
        );
        res_PutBoolean(
            c"config.usegl".as_ptr(),
            c_int::from(driver == TFB_GFXDRIVER_SDL_OPENGL as c_int),
        );
        let scaler = match opts.scaler {
            OPT_SCALETYPE_OPTVAL_HQXX_SCALE => {
                flags |= TFB_GFXFLAGS_SCALE_HQXX as c_int;
                c"hq"
            }
            OPT_SCALETYPE_OPTVAL_XBRZ3_SCALE => {
                flags |= TFB_GFXFLAGS_SCALE_XBRZ3 as c_int;
                c"xbrz3"
            }
            OPT_SCALETYPE_OPTVAL_XBRZ4_SCALE => {
                flags |= TFB_GFXFLAGS_SCALE_XBRZ4 as c_int;
                c"xbrz4"
            }
            _ => c"no",
        };
        res_PutString(c"config.scaler".as_ptr(), scaler.as_ptr());
        if opts.scanlines != 0 {
            flags |= TFB_GFXFLAGS_SCANLINES as c_int;
        } else {
            flags &= !(TFB_GFXFLAGS_SCANLINES as c_int);
        }
        if opts.fullscreen != 0 {
            flags |= TFB_GFXFLAGS_FULLSCREEN as c_int;
        } else {
            flags &= !(TFB_GFXFLAGS_FULLSCREEN as c_int);
        }
        res_PutBoolean(
            c"config.scanlines".as_ptr(),
            c_int::from(opts.scanlines != 0),
        );
        res_PutBoolean(
            c"config.fullscreen".as_ptr(),
            c_int::from(opts.fullscreen != 0),
        );
        if width != ScreenWidthActual
            || height != ScreenHeightActual
            || driver != GraphicsDriver
            || flags != GfxFlags
        {
            FlushGraphics();
            UninitVideoPlayer();
            TFB_DrawScreen_ReinitVideo(driver, flags, width, height);
            FlushGraphics();
            InitVideoPlayer(1);
        }
        let range = &*ptr::addr_of!(GAMMA);
        if optGamma != 1.0 || range.slider_to_gamma(opts.gamma) != 1.0 {
            optGamma = range.slider_to_gamma(opts.gamma);
            setGammaCorrection(optGamma);
        }
        optSubtitles = BOOLEAN::from(opts.subtitles == OPT_ENABLABLE_OPTVAL_ENABLED);
        optWhichMenu = if opts.menu == OPT_CONSOLETYPE_OPTVAL_3DO {
            OPT_3DO
        } else {
            OPT_PC
        } as c_int;
        optWhichFonts = if opts.text == OPT_CONSOLETYPE_OPTVAL_3DO {
            OPT_3DO
        } else {
            OPT_PC
        } as c_int;
        optWhichCoarseScan = if opts.cscan == OPT_CONSOLETYPE_OPTVAL_3DO {
            OPT_3DO
        } else {
            OPT_PC
        } as c_int;
        optSmoothScroll = if opts.scroll == OPT_CONSOLETYPE_OPTVAL_3DO {
            OPT_3DO
        } else {
            OPT_PC
        } as c_int;
        optWhichShield = if opts.shield == OPT_CONSOLETYPE_OPTVAL_3DO {
            OPT_3DO
        } else {
            OPT_PC
        } as c_int;
        optMeleeScale = if opts.meleezoom == OPT_CONSOLETYPE_OPTVAL_3DO {
            SCALE_TFB_SCALE_TRILINEAR
        } else {
            SCALE_TFB_SCALE_STEP
        } as c_int;
        opt3doMusic = BOOLEAN::from(opts.music3do == OPT_ENABLABLE_OPTVAL_ENABLED);
        optRemixMusic = BOOLEAN::from(opts.musicremix == OPT_ENABLABLE_OPTVAL_ENABLED);
        optSpeech = BOOLEAN::from(opts.speech == OPT_ENABLABLE_OPTVAL_ENABLED);
        optWhichIntro = if opts.intro == OPT_CONSOLETYPE_OPTVAL_3DO {
            OPT_3DO
        } else {
            OPT_PC
        } as c_int;
        optStereoSFX = BOOLEAN::from(opts.stereo == OPT_ENABLABLE_OPTVAL_ENABLED);
        optKeepAspectRatio = BOOLEAN::from(opts.keepaspect == OPT_ENABLABLE_OPTVAL_ENABLED);
        let controls = ptr::addr_of_mut!(PlayerControls).cast::<CONTROL_TEMPLATE>();
        controls.write(opts.player1);
        controls.add(1).write(opts.player2);
        for (key, value) in [
            (
                c"config.subtitles",
                opts.subtitles == OPT_ENABLABLE_OPTVAL_ENABLED,
            ),
            (c"config.textmenu", opts.menu == OPT_CONSOLETYPE_OPTVAL_PC),
            (
                c"config.textgradients",
                opts.text == OPT_CONSOLETYPE_OPTVAL_PC,
            ),
            (
                c"config.iconicscan",
                opts.cscan == OPT_CONSOLETYPE_OPTVAL_3DO,
            ),
            (
                c"config.smoothscroll",
                opts.scroll == OPT_CONSOLETYPE_OPTVAL_3DO,
            ),
            (
                c"config.3domusic",
                opts.music3do == OPT_ENABLABLE_OPTVAL_ENABLED,
            ),
            (
                c"config.remixmusic",
                opts.musicremix == OPT_ENABLABLE_OPTVAL_ENABLED,
            ),
            (
                c"config.speech",
                opts.speech == OPT_ENABLABLE_OPTVAL_ENABLED,
            ),
            (
                c"config.3domovies",
                opts.intro == OPT_CONSOLETYPE_OPTVAL_3DO,
            ),
            (c"config.showfps", opts.fps == OPT_ENABLABLE_OPTVAL_ENABLED),
            (
                c"config.smoothmelee",
                opts.meleezoom == OPT_CONSOLETYPE_OPTVAL_3DO,
            ),
            (
                c"config.positionalsfx",
                opts.stereo == OPT_ENABLABLE_OPTVAL_ENABLED,
            ),
            (
                c"config.pulseshield",
                opts.shield == OPT_CONSOLETYPE_OPTVAL_3DO,
            ),
            (
                c"config.keepaspectratio",
                opts.keepaspect == OPT_ENABLABLE_OPTVAL_ENABLED,
            ),
        ] {
            res_PutBoolean(key.as_ptr(), c_int::from(value));
        }
        res_PutInteger(
            c"config.gamma".as_ptr(),
            ((optGamma * GAMMA_SCALE as f32) as f64 + 0.5) as c_int,
        );
        res_PutInteger(c"config.player1control".as_ptr(), opts.player1 as c_int);
        res_PutInteger(c"config.player2control".as_ptr(), opts.player2 as c_int);
        match opts.adriver {
            OPT_ADRIVERTYPE_OPTVAL_SILENCE => {
                res_PutString(c"config.audiodriver".as_ptr(), c"none".as_ptr())
            }
            OPT_ADRIVERTYPE_OPTVAL_MIXSDL => {
                res_PutString(c"config.audiodriver".as_ptr(), c"mixsdl".as_ptr())
            }
            OPT_ADRIVERTYPE_OPTVAL_OPENAL => {
                res_PutString(c"config.audiodriver".as_ptr(), c"openal".as_ptr())
            }
            _ => {}
        }
        match opts.aquality {
            OPT_AQUALITYTYPE_OPTVAL_LOW => {
                res_PutString(c"config.audioquality".as_ptr(), c"low".as_ptr())
            }
            OPT_AQUALITYTYPE_OPTVAL_MEDIUM => {
                res_PutString(c"config.audioquality".as_ptr(), c"medium".as_ptr())
            }
            OPT_AQUALITYTYPE_OPTVAL_HIGH => {
                res_PutString(c"config.audioquality".as_ptr(), c"high".as_ptr())
            }
            _ => {}
        }
        res_PutInteger(c"config.musicvol".as_ptr(), opts.musicvol);
        res_PutInteger(c"config.sfxvol".as_ptr(), opts.sfxvol);
        res_PutInteger(c"config.speechvol".as_ptr(), opts.speechvol);
        musicVolumeScale = opts.musicvol as f32 / 100.0;
        sfxVolumeScale = opts.sfxvol as f32 / 100.0;
        speechVolumeScale = opts.speechvol as f32 / 100.0;
        SetMusicVolume(i32::from(musicVolume as COUNT));
        SetSpeechVolume(speechVolumeScale);
        for (i, key) in [
            c"keys.1.name",
            c"keys.2.name",
            c"keys.3.name",
            c"keys.4.name",
            c"keys.5.name",
            c"keys.6.name",
        ]
        .iter()
        .enumerate()
        {
            res_PutString(key.as_ptr(), input_templates[i].name.as_ptr());
        }
        SaveResourceIndex(
            (configDir).cast(),
            c"uqm.cfg".as_ptr(),
            c"config.".as_ptr(),
            1,
        );
        SaveKeyConfiguration((configDir).cast(), c"flight.cfg".as_ptr());
    }
}

#[cfg(not(test))]
pub use runtime::{GetGlobalOptions, SetGlobalOptions, SetupMenu};

#[cfg(test)]
mod tests {
    use super::GammaRange;
    #[test]
    fn gamma_range_has_exact_unity_step() {
        for upper in [true, false] {
            let mut range = GammaRange::new(0.4, 2.5);
            range.update_bounds(upper);
            assert_eq!(range.slider_to_gamma(range.gamma_to_slider(1.0)), 1.0);
            assert!(range.slider_to_gamma(0) < 1.0);
            assert!(range.slider_to_gamma(100) > 1.0);
        }
    }
    #[test]
    fn gamma_slider_is_monotonic_and_round_trips() {
        let range = GammaRange::new(0.4, 2.5);
        for value in 0..=100 {
            assert_eq!(range.gamma_to_slider(range.slider_to_gamma(value)), value);
            if value > 0 {
                assert!(range.slider_to_gamma(value) > range.slider_to_gamma(value - 1));
            }
        }
    }
}
