//! Netplay connection dialog, ported from uqm/cnctdlg.c.
#![allow(non_snake_case)]
#![cfg(not(test))]

use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use std::{
    ffi::{c_char, c_int},
    mem, ptr,
};

const MCD_WIDTH: c_int = 260;
const MCD_HEIGHT: c_int = 110;
const MENU_FRAME_RATE: DWORD = ONE_SECOND / 20;

#[repr(C)]
pub struct ConnectDialogState {
    pub InputFunc: Option<unsafe extern "C" fn(*mut ConnectDialogState) -> BOOLEAN>,
    pub NextTime: DWORD,
    pub Initialized: BOOLEAN,
    pub which_side: c_int,
    pub confirmed: c_int,
    pub session: *mut crate::supermelee::netplay::NetplaySession,
}
static mut MENU: WIDGET_MENU_SCREEN = unsafe { mem::zeroed() };
static mut BUTTONS: [WIDGET_BUTTON; 3] = unsafe { mem::zeroed() };
static mut SLIDER: WIDGET_SLIDER = unsafe { mem::zeroed() };
static mut TEXTS: [WIDGET_TEXTENTRY; 2] = unsafe { mem::zeroed() };
static mut MENU_WIDGETS: [*mut WIDGET; 6] = [ptr::null_mut(); 6];
static mut DONE: BOOLEAN = 0;
static mut CURRENT_STATE: *mut ConnectDialogState = ptr::null_mut();
static mut PLAYER_FONT_HANDLE: FONT = ptr::null_mut();

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
const MENU_TEXT_COLOR: Color = rgb15(6, 6, 6);
const MENU_HIGHLIGHT_COLOR: Color = rgb15(31, 31, 0);
const MENU_CURSOR_COLOR: Color = rgb15(31, 31, 31);
unsafe fn game_string(index: usize) -> *const c_char {
    GetStringAddress(SetAbsStringTableIndex(GameStrings, index as COUNT))
}
unsafe extern "C" fn MCD_DrawMenuScreen(widget: *mut WIDGET, x: c_int, y: c_int) {
    let screen = widget.cast::<WIDGET_MENU_SCREEN>();
    let mut widget_y = y + 8;
    for i in 0..(*screen).num_children as usize {
        let child = *(*screen).child.add(i);
        ((*child).draw.unwrap())(child, x, widget_y);
        widget_y += ((*child).height.unwrap())(child) + 8;
    }
}
unsafe extern "C" fn MCD_DrawButton(widget: *mut WIDGET, _x: c_int, y: c_int) {
    let button = widget.cast::<WIDGET_BUTTON>();
    let old_font = SetContextFont(StarConFont);
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let mut text: TEXT = mem::zeroed();
    text.baseline = POINT {
        x: 160,
        y: y as COORD,
    };
    text.align = TEXT_ALIGN_ALIGN_CENTER;
    text.CharCount = COUNT::MAX;
    text.pStr = (*button).name;
    let old_text = SetContextForeGroundColor(if widget_focus == widget {
        MENU_HIGHLIGHT_COLOR
    } else {
        MENU_TEXT_COLOR
    });
    font_DrawText(&mut text);
    SetContextFontEffect(old_effect);
    SetContextFont(old_font);
    SetContextForeGroundColor(old_text);
}
unsafe extern "C" fn MCD_DrawSlider(widget: *mut WIDGET, x: c_int, y: c_int) {
    let slider_widget = widget.cast::<WIDGET_SLIDER>();
    let old_font = SetContextFont(PLAYER_FONT_HANDLE);
    let old_effect = SetContextFontEffect(ptr::null_mut());
    let tick = MCD_WIDTH / 8;
    let mut text: TEXT = mem::zeroed();
    text.baseline = POINT {
        x: x as COORD,
        y: y as COORD,
    };
    text.align = TEXT_ALIGN_ALIGN_LEFT;
    text.CharCount = COUNT::MAX;
    text.pStr = (*slider_widget).category;
    let old_text = SetContextForeGroundColor(if widget_focus == widget {
        MENU_HIGHLIGHT_COLOR
    } else {
        MENU_TEXT_COLOR
    });
    font_DrawText(&mut text);
    let mut rectangle: RECT = mem::zeroed();
    rectangle.corner.x = (text.baseline.x as c_int + 3 * tick) as COORD;
    rectangle.corner.y = (text.baseline.y as c_int - 4) as COORD;
    rectangle.extent.height = 2;
    rectangle.extent.width = (3 * tick) as SIZE;
    DrawFilledRectangle(&mut rectangle);
    rectangle.extent.width = 3;
    rectangle.extent.height = 8;
    rectangle.corner.y = (text.baseline.y as c_int - 7) as COORD;
    rectangle.corner.x = (text.baseline.x as c_int
        + 3 * tick
        + 3 * tick * ((*slider_widget).value - (*slider_widget).min)
            / ((*slider_widget).max - (*slider_widget).min)
        - 1) as COORD;
    DrawFilledRectangle(&mut rectangle);
    ((*slider_widget).draw_value.unwrap())(
        slider_widget,
        text.baseline.x as c_int + 7 * tick,
        text.baseline.y as c_int,
    );
    SetContextFontEffect(old_effect);
    SetContextFont(old_font);
    SetContextForeGroundColor(old_text);
}
unsafe extern "C" fn MCD_DrawTextEntry(widget: *mut WIDGET, x: c_int, y: c_int) {
    let entry = widget.cast::<WIDGET_TEXTENTRY>();
    let old_font = SetContextFont(PLAYER_FONT_HANDLE);
    let old_effect = SetContextFontEffect(ptr::null_mut());
    BatchGraphics();
    let mut text: TEXT = mem::zeroed();
    text.baseline = POINT {
        x: x as COORD,
        y: y as COORD,
    };
    text.align = TEXT_ALIGN_ALIGN_LEFT;
    text.CharCount = COUNT::MAX;
    text.pStr = (*entry).category;
    SetContextForeGroundColor(if widget_focus == widget {
        MENU_HIGHLIGHT_COLOR
    } else {
        MENU_TEXT_COLOR
    });
    font_DrawText(&mut text);
    let old_text;
    (*entry).value[WIDGET_TEXTENTRY_WIDTH as usize - 1] = 0;
    text.baseline.y = y as COORD;
    text.CharCount = utf8StringCount(((*entry).value.as_ptr()).cast()) as COUNT;
    text.pStr = (*entry).value.as_ptr();
    if (*entry).state & WIDGET_TEXTENTRY_STATE_WTE_EDITING == 0 {
        text.baseline.x = 160;
        text.align = TEXT_ALIGN_ALIGN_CENTER;
        old_text = SetContextForeGroundColor(if widget_focus == widget {
            MENU_HIGHLIGHT_COLOR
        } else {
            MENU_TEXT_COLOR
        });
        font_DrawText(&mut text);
    } else {
        let mut text_rect: RECT = mem::zeroed();
        let mut deltas = [0 as BYTE; WIDGET_TEXTENTRY_WIDTH as usize];
        let mut leading: SIZE = 0;
        text.baseline.x = (x + 90) as COORD;
        text.align = TEXT_ALIGN_ALIGN_LEFT;
        GetContextFontLeading(&mut leading);
        let mut rectangle: RECT = mem::zeroed();
        rectangle.corner.x = (text.baseline.x as c_int - 1) as COORD;
        rectangle.corner.y = (text.baseline.y as c_int - leading as c_int + 2) as COORD;
        rectangle.extent.width = (MCD_WIDTH - rectangle.corner.x as c_int - 10) as SIZE;
        rectangle.extent.height = (leading as c_int + 2) as SIZE;
        TextRect(&mut text, &mut text_rect, deltas.as_mut_ptr());
        old_text = SetContextForeGroundColor(MENU_HIGHLIGHT_COLOR);
        DrawFilledRectangle(&mut rectangle);
        let mut delta = deltas.as_ptr();
        for _ in 0..(*entry).cursor_pos as COUNT {
            rectangle.corner.x = (rectangle.corner.x as c_int + *delta as c_int) as COORD;
            delta = delta.add(1);
        }
        if (*entry).cursor_pos < text.CharCount as c_int {
            rectangle.corner.x -= 1;
        }
        rectangle.extent.width = if (*entry).state & WIDGET_TEXTENTRY_STATE_WTE_BLOCKCUR != 0 {
            if (*entry).cursor_pos == text.CharCount as c_int {
                1
            } else if (*entry).cursor_pos + 1 == text.CharCount as c_int {
                *delta as SIZE + 2
            } else {
                *delta as SIZE + 1
            }
        } else {
            1
        };
        rectangle.corner.x += 1;
        rectangle.corner.y += 1;
        rectangle.extent.height -= 2;
        SetContextForeGroundColor(MENU_CURSOR_COLOR);
        DrawFilledRectangle(&mut rectangle);
        SetContextForeGroundColor(MENU_TEXT_COLOR);
        font_DrawText(&mut text);
    }
    UnbatchGraphics();
    SetContextFontEffect(old_effect);
    SetContextFont(old_font);
    SetContextForeGroundColor(old_text);
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
    DrawConnectDialog();
    SleepThreadUntil((*state).NextTime);
    (*state).NextTime = GetTimeCounter().wrapping_add(MENU_FRAME_RATE);
    1
}
unsafe extern "C" fn OnTextEntryEvent(widget: *mut WIDGET_TEXTENTRY) -> c_int {
    (*widget).cursor_pos = utf8StringCount(((*widget).value.as_ptr()).cast()) as c_int;
    (*widget).state = WIDGET_TEXTENTRY_STATE_WTE_EDITING;
    DrawConnectDialog();
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
    if DoTextEntry(&mut state) == 0 {
        utf8StringCopy(
            ((*widget).value.as_mut_ptr()).cast(),
            (*widget).maxlen as usize,
            revert_buf.as_ptr(),
        );
    } else if let Some(callback) = (*widget).onChange {
        callback(widget);
    }
    (*widget).state = WIDGET_TEXTENTRY_STATE_WTE_NORMAL;
    DrawConnectDialog();
    1
}
unsafe extern "C" fn do_connect(_: *mut WIDGET, event: c_int) -> c_int {
    if event == WIDGET_EVENT_SELECT as c_int {
        let peer = &mut (*(*CURRENT_STATE).session).options_mut().peer
            [(*CURRENT_STATE).which_side as usize];
        peer.host = TEXTS[0].value.as_ptr();
        peer.port = TEXTS[1].value.as_ptr();
        peer.isServer = false;
        (*CURRENT_STATE).confirmed = 1;
        (*(*CURRENT_STATE).session).options_mut().inputDelay = SLIDER.value as usize;
        DONE = 1;
    }
    0
}
unsafe extern "C" fn do_listen(_: *mut WIDGET, event: c_int) -> c_int {
    if event == WIDGET_EVENT_SELECT as c_int {
        let peer = &mut (*(*CURRENT_STATE).session).options_mut().peer
            [(*CURRENT_STATE).which_side as usize];
        peer.port = TEXTS[1].value.as_ptr();
        peer.isServer = true;
        (*(*CURRENT_STATE).session).options_mut().inputDelay = SLIDER.value as usize;
        (*CURRENT_STATE).confirmed = 1;
        DONE = 1;
    }
    0
}
unsafe extern "C" fn do_cancel(_: *mut WIDGET, event: c_int) -> c_int {
    if event == WIDGET_EVENT_SELECT as c_int {
        (*CURRENT_STATE).confirmed = 0;
        DONE = 1;
    }
    0
}
macro_rules! widget_header {
    ($widget:expr,$tag:ident,$event:ident,$focus:ident,$draw:ident,$height:ident) => {{
        let w = $widget as *mut _ as *mut WIDGET;
        (*w).tag = $tag;
        (*w).parent = ptr::null_mut();
        (*w).handleEvent = Some($event);
        (*w).receiveFocus = Some($focus);
        (*w).draw = Some($draw);
        (*w).height = Some($height);
        (*w).width = Some(Widget_WidthFullScreen);
    }};
}
unsafe fn CreateWidgets() {
    DONE = 0;
    for button in
        std::slice::from_raw_parts_mut(ptr::addr_of_mut!(BUTTONS).cast::<WIDGET_BUTTON>(), 3)
    {
        let button = button as *mut WIDGET_BUTTON;
        widget_header!(
            button,
            WIDGET_TYPE_WIDGET_TYPE_BUTTON,
            do_cancel,
            Widget_ReceiveFocusSimple,
            MCD_DrawButton,
            Widget_HeightOneLine
        );
    }
    BUTTONS[0].name = game_string(NETMELEE_STRING_BASE as usize + 19);
    BUTTONS[1].name = game_string(NETMELEE_STRING_BASE as usize + 20);
    BUTTONS[2].name = game_string(NETMELEE_STRING_BASE as usize + 21);
    BUTTONS[0].handleEvent = Some(do_connect);
    BUTTONS[1].handleEvent = Some(do_listen);
    BUTTONS[2].handleEvent = Some(do_cancel);
    widget_header!(
        ptr::addr_of_mut!(MENU),
        WIDGET_TYPE_WIDGET_TYPE_MENU_SCREEN,
        Widget_HandleEventMenuScreen,
        Widget_ReceiveFocusMenuScreen,
        MCD_DrawMenuScreen,
        Widget_HeightFullScreen
    );
    MENU.num_children = 6;
    MENU_WIDGETS = [
        ptr::addr_of_mut!(BUTTONS[1]).cast(),
        ptr::addr_of_mut!(TEXTS[0]).cast(),
        ptr::addr_of_mut!(BUTTONS[0]).cast(),
        ptr::addr_of_mut!(SLIDER).cast(),
        ptr::addr_of_mut!(TEXTS[1]).cast(),
        ptr::addr_of_mut!(BUTTONS[2]).cast(),
    ];
    MENU.child = ptr::addr_of_mut!(MENU_WIDGETS).cast();
    widget_header!(
        ptr::addr_of_mut!(SLIDER),
        WIDGET_TYPE_WIDGET_TYPE_SLIDER,
        Widget_HandleEventSlider,
        Widget_ReceiveFocusSimple,
        MCD_DrawSlider,
        Widget_HeightOneLine
    );
    SLIDER.draw_value = Some(Widget_Slider_DrawValue);
    SLIDER.min = 0;
    SLIDER.max = 9;
    SLIDER.step = 1;
    SLIDER.value = (*(*CURRENT_STATE).session).options_mut().inputDelay as c_int;
    SLIDER.category = game_string(NETMELEE_STRING_BASE as usize + 24);
    for entry in
        std::slice::from_raw_parts_mut(ptr::addr_of_mut!(TEXTS).cast::<WIDGET_TEXTENTRY>(), 2)
    {
        let entry = entry as *mut WIDGET_TEXTENTRY;
        widget_header!(
            entry,
            WIDGET_TYPE_WIDGET_TYPE_TEXTENTRY,
            Widget_HandleEventTextEntry,
            Widget_ReceiveFocusSimple,
            MCD_DrawTextEntry,
            Widget_HeightOneLine
        );
        (*entry).handleEventSelect = Some(OnTextEntryEvent);
        (*entry).maxlen = WIDGET_TEXTENTRY_WIDTH as c_int - 1;
        (*entry).state = WIDGET_TEXTENTRY_STATE_WTE_NORMAL;
        (*entry).cursor_pos = 0;
    }
    TEXTS[0].category = game_string(NETMELEE_STRING_BASE as usize + 22);
    TEXTS[1].category = game_string(NETMELEE_STRING_BASE as usize + 23);
    let peer =
        &(*(*CURRENT_STATE).session).options_mut().peer[(*CURRENT_STATE).which_side as usize];
    if TEXTS[0].value.as_ptr() != peer.host {
        libc::strncpy(
            TEXTS[0].value.as_mut_ptr(),
            peer.host,
            TEXTS[0].maxlen as usize,
        );
    }
    if TEXTS[1].value.as_ptr() != peer.port {
        libc::strncpy(
            TEXTS[1].value.as_mut_ptr(),
            peer.port,
            TEXTS[1].maxlen as usize,
        );
    }
    TEXTS[0].value[TEXTS[0].maxlen as usize] = 0;
    TEXTS[1].value[TEXTS[1].maxlen as usize] = 0;
    (MENU.receiveFocus.unwrap())(ptr::addr_of_mut!(MENU).cast(), WIDGET_EVENT_DOWN as c_int);
}
unsafe fn DrawConnectDialog() {
    let mut rectangle = RECT {
        extent: EXTENT {
            width: MCD_WIDTH as SIZE,
            height: MCD_HEIGHT as SIZE,
        },
        corner: POINT {
            x: ((ScreenWidth - MCD_WIDTH) >> 1) as COORD,
            y: ((ScreenHeight - MCD_HEIGHT) >> 1) as COORD,
        },
    };
    DrawShadowedBox(
        &mut rectangle,
        rgb15(0x14, 0x14, 0x14),
        rgb15(8, 8, 8),
        rgb15(0x10, 0x10, 0x10),
    );
    (MENU.draw.unwrap())(
        ptr::addr_of_mut!(MENU).cast(),
        rectangle.corner.x as c_int + 10,
        rectangle.corner.y as c_int + 10,
    );
}
unsafe extern "C" fn DoMeleeConnectDialog(state: *mut ConnectDialogState) -> BOOLEAN {
    GamePaused = 0;
    if (*state).Initialized == 0 {
        (*state).Initialized = 1;
        SetDefaultMenuRepeatDelay();
        (*state).NextTime = GetTimeCounter();
        CreateWidgets();
        DrawConnectDialog();
    }
    let mut changed = true;
    let mut event = None;
    for (key, value) in [
        (KEY_MENU_UP, WIDGET_EVENT_UP),
        (KEY_MENU_DOWN, WIDGET_EVENT_DOWN),
        (KEY_MENU_LEFT, WIDGET_EVENT_LEFT),
        (KEY_MENU_RIGHT, WIDGET_EVENT_RIGHT),
        (KEY_MENU_SELECT, WIDGET_EVENT_SELECT),
        (KEY_MENU_CANCEL, WIDGET_EVENT_CANCEL),
        (KEY_MENU_DELETE, WIDGET_EVENT_DELETE),
    ] {
        if PulsedInputState.menu[key as usize] != 0 {
            event = Some(value);
            break;
        }
    }
    if let Some(event) = event {
        Widget_Event(event as c_int);
    } else {
        changed = false;
    }
    if changed {
        DrawConnectDialog();
    }
    SleepThreadUntil((*state).NextTime.wrapping_add(MENU_FRAME_RATE));
    (*state).NextTime = GetTimeCounter();
    BOOLEAN::from(GlobData.Game_state.CurrentActivity & CHECK_ABORT as ACTIVITY == 0 && DONE == 0)
}

/// # Safety
/// Must run on the game thread with graphics, input, strings and netplay options
/// initialized. `side` must index one of the two netplay peer records.
pub unsafe fn melee_connect_dialog(
    session: &mut crate::supermelee::netplay::NetplaySession,
    side: c_int,
) -> BOOLEAN {
    PLAYER_FONT_HANDLE =
        LoadGraphicInstance(crate::navigation::abi::PLAYER_FONT.as_ptr().cast()).cast();
    let mut state: ConnectDialogState = mem::zeroed();
    state.Initialized = 0;
    state.which_side = side;
    state.session = session;
    state.InputFunc = Some(DoMeleeConnectDialog);
    state.confirmed = 1;
    CURRENT_STATE = &mut state;
    DoInput(ptr::addr_of_mut!(state).cast(), 1);
    CURRENT_STATE = ptr::null_mut();
    DestroyFont(PLAYER_FONT_HANDLE);
    state.confirmed as BOOLEAN
}
