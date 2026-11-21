//! SIS conversation responses, feedback and paginated subtitle history.

use super::locdata::CColor;
use super::Comm;
use std::ffi::{CStr, CString};
use uqm_engine::graphics::{
    context::ContextLease, drawable::ContextFrame, font::FontLease, geometry::Color,
    geometry::Extent, geometry::Point, geometry::Rect,
};

pub(crate) struct ConversationResources {
    pub commander_name: CString,
    pub ship_name: CString,
    pub display: Option<ConversationDisplay>,
}

#[derive(Clone)]
pub(crate) struct ConversationDisplay {
    pub space: ContextLease,
    pub player_font: FontLease,
    pub history_font: FontLease,
    pub game_strings: uqm_engine::strings::owned_table::LoadedTextTable,
    pub subtitles: SubtitleCache,
}

#[derive(Clone)]
pub(crate) struct SubtitleCache {
    pub target: ContextLease,
    pub context: ContextLease,
    pub frame: ContextFrame,
    pub font: FontLease,
}

impl ConversationDisplay {
    pub(crate) fn new(
        comm: &Comm,
        assets: &super::assets::Assets,
        kernel: &uqm_engine::Kernel,
    ) -> Result<Self, &'static str> {
        let history = comm
            .presentation
            .tiny_font
            .ok_or("conversation history font")?;
        Ok(Self {
            space: ContextLease::require(comm.presentation.space_context),
            player_font: assets.player.lease(),
            history_font: FontLease::retain(history),
            game_strings: kernel.game_text_table(),
            subtitles: assets.subtitles(),
        })
    }
}

impl ConversationResources {
    pub(crate) fn new(world: &uqm_engine::World) -> Result<Self, std::ffi::FromBytesUntilNulError> {
        Ok(Self {
            commander_name: CStr::from_bytes_until_nul(&world.data.flagship.commander_name)?
                .to_owned(),
            ship_name: CStr::from_bytes_until_nul(&world.data.flagship.ship_name)?.to_owned(),
            display: None,
        })
    }
}

impl Comm {
    pub fn admit_conversation_names(
        &mut self,
        world: &uqm_engine::World,
    ) -> Result<(), std::ffi::FromBytesUntilNulError> {
        self.display.resources = Some(ConversationResources::new(world)?);
        Ok(())
    }

    pub(crate) fn conversation_resources(&self) -> &ConversationResources {
        self.display
            .resources
            .as_ref()
            .unwrap_or_else(|| panic!("conversation resources were not admitted"))
    }

    pub(crate) fn conversation_display(&self) -> &ConversationDisplay {
        self.conversation_resources()
            .display
            .as_ref()
            .unwrap_or_else(|| panic!("conversation display was not admitted"))
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Text<'a> {
    pub text: &'a CStr,
    pub baseline: Point,
    pub align: i32,
    pub char_count: u16,
}
impl<'a> Text<'a> {
    pub fn new(text: &'a CStr, baseline: Point, align: i32, char_count: u16) -> Self {
        Self {
            text,
            baseline,
            align,
            char_count,
        }
    }

    pub fn fit(
        &mut self,
        font: &FontLease,
        width: i16,
        max_chars: u16,
    ) -> super::text_layout::FittedLine {
        let fitted = super::text_layout::fit_text_line(
            self.text,
            self.char_count,
            width,
            max_chars,
            |count| {
                let measured = font.measure(
                    self.text,
                    self.baseline,
                    uqm_engine::graphics::geometry::TextAlign::from_c(self.align),
                    count,
                );
                (measured.rect.extent.width, measured.characters)
            },
        );
        self.char_count = fitted.characters;
        fitted
    }

    pub fn draw(&self, context: &ContextLease, font: &FontLease, color: Color) {
        context.draw_text_bounded(
            font,
            self.text,
            self.baseline,
            uqm_engine::graphics::geometry::TextAlign::from_c(self.align),
            color,
            self.char_count,
        );
    }
}

pub(crate) fn suffix(text: &CStr, offset: usize) -> &CStr {
    &text[offset..]
}

const WON_LAST_BATTLE: u8 = 5;
const SLIDER_Y: i16 = 107;
const SLIDER_HEIGHT: i16 = 15;
const SIS_SCREEN_WIDTH: i16 = 242;
const SIS_SCREEN_HEIGHT: i16 = 227;
const TEXT_X_OFFS: i16 = 1;
const PLAYER_TEXT_WIDTH: i16 = SIS_SCREEN_WIDTH - 8 - (TEXT_X_OFFS << 2);
const DELTA_Y_SUMMARY: i16 = 8;
const MAX_SUMM_ROWS: i16 = (SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT) / DELTA_Y_SUMMARY - 1;
const CHECK_ABORT: u16 = 0x4000;
const ONE_SECOND: u32 = 840;
pub(super) const FEEDBACK_STRING_BASE: u16 = crate::starbase::ids::FEEDBACK_STRING_BASE as u16;
const KEY_MENU_SELECT: usize = 9;
const KEY_MENU_CANCEL: usize = 10;
const KEY_MENU_RIGHT: usize = 8;

const fn make_rgb15(r: u8, g: u8, b: u8) -> CColor {
    CColor {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 0xff,
    }
}
const COMM_PLAYER_TEXT_NORMAL_COLOR: CColor = make_rgb15(0x00, 0x14, 0x14);
const COMM_PLAYER_TEXT_HIGHLIGHT_COLOR: CColor = make_rgb15(0x1a, 0x1a, 0x1a);
const COMM_PLAYER_BACKGROUND_COLOR: CColor = make_rgb15(0x00, 0x00, 0x14);
const COMM_RESPONSE_INTRO_TEXT_COLOR: CColor = make_rgb15(0x0a, 0x0c, 0x1f);
const COMM_FEEDBACK_TEXT_COLOR: CColor = make_rgb15(0x12, 0x14, 0x4f);
const COMM_HISTORY_BACKGROUND_COLOR: CColor = make_rgb15(0x00, 0x05, 0x00);
const COMM_HISTORY_TEXT_COLOR: CColor = make_rgb15(0x00, 0x10, 0x00);
const COMM_MORE_TEXT_COLOR: CColor = make_rgb15(0x00, 0x17, 0x00);

fn window_rect() -> Rect {
    Rect {
        corner: Point {
            x: 0,
            y: SLIDER_Y + SLIDER_HEIGHT,
        },
        extent: Extent {
            width: SIS_SCREEN_WIDTH,
            height: SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT,
        },
    }
}

fn space_context(comm: &Comm) -> ContextLease {
    comm.conversation_display().space.clone()
}

fn player_font(comm: &Comm) -> FontLease {
    comm.conversation_display().player_font.clone()
}

fn game_string(comm: &Comm, index: u16) -> &CStr {
    comm.conversation_display().game_strings.label(index).0
}

fn with_player_font<R>(context: &ContextLease, font: &FontLease, draw: impl FnOnce() -> R) -> R {
    context.with_selected(|| {
        let (result, foreground) = context.with_text_state(|| {
            context.set_font(font);
            let result = draw();
            (result, context.foreground())
        });
        // Native feedback restores the font but leaves its last foreground color selected.
        context.set_foreground(foreground);
        result
    })
}

pub(crate) fn draw_sis_com_window(comm: &Comm, activity: u16) {
    if activity as u8 != WON_LAST_BATTLE {
        let context = space_context(comm);
        context
            .with_selected(|| context.fill_rectangle(COMM_PLAYER_BACKGROUND_COLOR, window_rect()));
    }
}

fn draw_wrapped(
    context: &ContextLease,
    font: &FontLease,
    text: &mut Text<'_>,
    color: Color,
) -> i16 {
    let mut remaining = u16::MAX;
    let mut input = text.text;
    let leading = font.leading();
    text.baseline.y -= leading;
    loop {
        text.text = input;
        text.baseline.y += leading;
        let fitted = text.fit(font, PLAYER_TEXT_WIDTH, remaining);
        remaining = remaining.wrapping_sub(text.char_count).saturating_sub(1);
        input = suffix(input, fitted.next);
        if text.baseline.y < SIS_SCREEN_HEIGHT {
            text.draw(context, font, color);
        }
        if fitted.end_of_line || remaining == 0 {
            break;
        }
    }
    text.baseline.y
}

pub fn feedback_player_phrase(comm: &Comm, activity: u16, text: Option<&CStr>) {
    let context = space_context(comm);
    context.with_selected(|| {
        context.with_batch(|| {
            draw_sis_com_window(comm, activity);
            if let Some(input) = text.filter(|text| !text.is_empty()) {
                let font = player_font(comm);
                with_player_font(&context, &font, || {
                    let mut text = Text::new(
                        game_string(comm, FEEDBACK_STRING_BASE),
                        Point {
                            x: SIS_SCREEN_WIDTH >> 1,
                            y: SLIDER_Y + SLIDER_HEIGHT + 13,
                        },
                        1,
                        u16::MAX,
                    );
                    text.draw(&context, &font, COMM_RESPONSE_INTRO_TEXT_COLOR);
                    text.text = input;
                    text.baseline.y += 16 + font.leading();
                    draw_wrapped(&context, &font, &mut text, COMM_FEEDBACK_TEXT_COLOR);
                });
            }
        })
    });
}

fn response_page_text(text: &str) -> CString {
    CString::from(
        text.bytes()
            .map_while(std::num::NonZeroU8::new)
            .take(1023)
            .collect::<Vec<_>>(),
    )
}

fn scroll_indicator(context: &ContextLease, comm: &Comm, index: u16, origin_y: i16) {
    if let Some(frame) = comm
        .presentation
        .activity_frame
        .non_null()
        .map(ContextFrame::retain)
    {
        let frame = frame.wrapped_frame(index);
        context.draw_stamp(&frame, scroll_indicator_origin(&frame, origin_y));
    }
}

fn scroll_indicator_origin(frame: &ContextFrame, origin_y: i16) -> Point {
    Point {
        x: SIS_SCREEN_WIDTH - frame.rect().extent.width - 1,
        y: origin_y,
    }
}

pub fn refresh_responses(comm: &mut Comm, activity: u16, top: u8, count: u8, selected: u8) {
    comm.response_display = [top, count, selected];
    let context = space_context(comm);
    let font = player_font(comm);
    with_player_font(&context, &font, || {
        context.with_batch(|| {
            draw_sis_com_window(comm, activity);
            let mut y = SLIDER_Y + SLIDER_HEIGHT + 1;
            for index in top..count {
                let Some(entry) = comm.responses().get(index as usize) else {
                    continue;
                };
                let input = response_page_text(&entry.response_text);
                let color = if index == selected {
                    COMM_PLAYER_TEXT_HIGHLIGHT_COLOR
                } else {
                    COMM_PLAYER_TEXT_NORMAL_COLOR
                };
                let mut text = Text::new(
                    &input,
                    Point {
                        x: TEXT_X_OFFS + 8,
                        y: y + font.leading(),
                    },
                    0,
                    u16::MAX,
                );
                let bullet = Text {
                    text: c"•",
                    baseline: Point {
                        x: text.baseline.x - 8,
                        y: text.baseline.y,
                    },
                    ..text
                };
                bullet.draw(&context, &font, color);
                y = draw_wrapped(&context, &font, &mut text, color);
            }
            if top != 0 {
                scroll_indicator(&context, comm, 6, SLIDER_Y + SLIDER_HEIGHT + 1);
            } else if y > SIS_SCREEN_HEIGHT {
                scroll_indicator(&context, comm, 7, SIS_SCREEN_HEIGHT - 2);
            }
        })
    });
}

#[derive(Default)]
struct SummaryLoopState {
    print_next: bool,
    history: Vec<uqm_engine::sound::trackplayer::SubtitleSnapshot>,
    next_sub: usize,
    left_over: Option<usize>,
}
impl SummaryLoopState {
    fn has_next(&self) -> bool {
        self.next_sub < self.history.len()
    }
}

fn copy_prompt_bytes(source: &[u8], buffer: &mut [u8; 80], offset: usize) -> usize {
    if offset >= buffer.len() {
        return offset;
    }
    let length = source.len().min(buffer.len() - offset - 1);
    buffer[offset..offset + length].copy_from_slice(&source[..length]);
    offset + length
}

fn more_prompt(more: &CStr) -> CString {
    let mut buffer = [0; 80];
    let bullet = "·".as_bytes();
    let blen = copy_prompt_bytes(bullet, &mut buffer, 0);
    let mlen = copy_prompt_bytes(more.to_bytes(), &mut buffer, blen);
    copy_prompt_bytes(bullet, &mut buffer, blen + mlen);
    CString::from(
        buffer
            .into_iter()
            .map_while(std::num::NonZeroU8::new)
            .collect::<Vec<_>>(),
    )
}

fn draw_summary_page_contents(comm: &Comm, state: &mut SummaryLoopState) {
    let context = space_context(comm);
    let font = comm.conversation_display().history_font.clone();
    context.with_selected(|| {
        context.fill_rectangle(COMM_HISTORY_BACKGROUND_COLOR, window_rect());
        context.set_foreground(COMM_HISTORY_TEXT_COLOR);
        context.set_font(&font);
        let mut text = Text::new(
            c"",
            Point {
                x: 2,
                y: SLIDER_Y + SLIDER_HEIGHT + DELTA_Y_SUMMARY,
            },
            0,
            0,
        );
        let mut row = 0;
        while row < MAX_SUMM_ROWS && state.has_next() {
            let page = state.history[state.next_sub].text.as_c_str();
            let mut offset = state.left_over.take().unwrap_or(0);
            text.text = suffix(page, offset);
            text.char_count = u16::MAX;
            while row < MAX_SUMM_ROWS {
                let fitted = text.fit(&font, SIS_SCREEN_WIDTH - 4, u16::MAX);
                if fitted.end_of_line {
                    break;
                }
                text.draw(&context, &font, COMM_HISTORY_TEXT_COLOR);
                text.baseline.y += DELTA_Y_SUMMARY;
                row += 1;
                offset += fitted.next;
                text.text = suffix(page, offset);
                text.char_count = u16::MAX;
            }
            if row >= MAX_SUMM_ROWS {
                state.left_over = Some(offset);
                break;
            }
            text.draw(&context, &font, COMM_HISTORY_TEXT_COLOR);
            text.baseline.y += DELTA_Y_SUMMARY;
            row += 1;
            state.next_sub += 1;
        }
        if row >= MAX_SUMM_ROWS && (state.has_next() || state.left_over.is_some()) {
            let prompt = more_prompt(game_string(comm, FEEDBACK_STRING_BASE + 1));
            Text::new(
                &prompt,
                Point {
                    x: SIS_SCREEN_WIDTH >> 1,
                    y: text.baseline.y,
                },
                1,
                0,
            )
            .draw(&context, &font, COMM_MORE_TEXT_COLOR);
        }
    });
}

fn do_summary_page_frame(
    comm: &Comm,
    input: &uqm_engine::input::InputSession,
    state: &mut SummaryLoopState,
    activity: u16,
) -> bool {
    if activity & CHECK_ABORT != 0 {
        return false;
    }
    let keys = input.pulsed();
    if [KEY_MENU_SELECT, KEY_MENU_CANCEL, KEY_MENU_RIGHT]
        .into_iter()
        .any(|key| keys.menu[key] != 0)
    {
        if state.has_next() {
            state.print_next = true;
        } else {
            return false;
        }
    } else if state.print_next {
        draw_summary_page_contents(comm, state);
        state.print_next = false;
    } else {
        uqm_engine::threading::pacing::GameThreadPacer::new(space_context(comm))
            .sleep_for(ONE_SECOND / 20);
    }
    true
}

pub fn select_conversation_summary(
    comm: &mut Comm,
    cx: &mut uqm_engine::battle::Cx<'_>,
    io: &mut uqm_engine::observe::Io<'_>,
) {
    if comm.response_display[1] > 0 {
        if let Some(entry) = comm.responses().get(comm.response_display[2] as usize) {
            let text = response_page_text(&entry.response_text);
            feedback_player_phrase(comm, cx.world.activity(), Some(&text));
        }
    }
    let mut state = SummaryLoopState {
        print_next: true,
        history: uqm_engine::sound::trackplayer::subtitle_history(),
        ..Default::default()
    };
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    comm.run_input(cx, io, false, |comm, cx, _| {
        do_summary_page_frame(comm, input, &mut state, cx.world.activity())
    });
    if comm.response_display[1] > 0 {
        let [top, count, selected] = comm.response_display;
        refresh_responses(comm, cx.world.activity(), top, count, selected);
    }
    super::comm_abi::clear_subtitles(comm);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_names_are_admitted_once_and_reject_unterminated_buffers() {
        let mut game = uqm_engine::Game::new();
        game.world.data.flagship.commander_name[..5].copy_from_slice("Zoë\0".as_bytes());
        game.world.data.flagship.ship_name[..4].copy_from_slice("船\0".as_bytes());
        let mut comm = Comm::new();
        comm.admit_conversation_names(&game.world).unwrap();
        game.world.data.flagship.commander_name.fill(b'x');
        game.world.data.flagship.ship_name.fill(b'x');
        assert_eq!(comm.conversation_resources().commander_name, c"Zoë");
        assert_eq!(comm.conversation_resources().ship_name, c"船");
        assert!(Comm::new().admit_conversation_names(&game.world).is_err());
    }

    #[test]
    #[should_panic(expected = "conversation resources were not admitted")]
    fn names_cannot_be_used_without_session_admission() {
        let comm = Comm::new();
        comm.conversation_resources();
    }

    #[test]
    fn response_display_preserves_its_native_byte_limit_and_nul_boundary() {
        assert_eq!(response_page_text("ab\0cd"), c"ab");
        assert_eq!(
            response_page_text(&"x".repeat(2048)).as_bytes(),
            vec![b'x'; 1023]
        );
        assert_eq!(response_page_text("").as_c_str(), c"");
    }

    #[test]
    fn history_prompt_preserves_legacy_offsets_and_byte_bounds() {
        assert_eq!(more_prompt(c"MORE").as_c_str(), c"·MORE");
        assert_eq!(
            more_prompt(&CString::new("x".repeat(100)).unwrap())
                .as_bytes()
                .len(),
            79
        );
    }

    #[test]
    fn history_starts_without_a_page_or_leftover() {
        let state = SummaryLoopState::default();
        assert!(!state.print_next);
        assert!(!state.has_next());
        assert_eq!(state.left_over, None);
    }
    #[test]
    #[serial_test::serial]
    fn history_pages_and_scroll_indicators_use_real_retained_resources() {
        super::super::sis_graphics_font_tests::with_display("comm::sis_graphics::tests::history_pages_and_scroll_indicators_use_real_retained_resources", |comm, context| {
            use uqm_engine::sound::trackplayer;
            trackplayer::stop_track().unwrap();
            for _ in 0..20 {
                trackplayer::splice_track(Some("history"), Some("A"), None, None, vec![]).unwrap();
            }
            let mut state = SummaryLoopState { history: trackplayer::subtitle_history(), print_next: true, ..Default::default() };
            assert_eq!(state.history.len(), 20);
            trackplayer::stop_track().unwrap();
            draw_summary_page_contents(comm, &mut state);
            assert_eq!(state.next_sub, MAX_SUMM_ROWS as usize);
            assert!(state.has_next());
            context.flush();
            assert!((122..227).any(|y| context.target().pixel(Point { x: 2, y }).g != 0));
            draw_summary_page_contents(comm, &mut state);
            assert_eq!(state.next_sub, 20);
            assert!(!state.has_next());
            let input = uqm_engine::input::InputSession::default();
            assert!(!do_summary_page_frame(comm, &input, &mut state, CHECK_ABORT));
            state.print_next = false;
            assert!(do_summary_page_frame(comm, &input, &mut state, 0));
            let frames = uqm_engine::graphics::drawable::LoadedDrawable::create(8, 13, 5, 8).unwrap();
            let frame = frames.lease().context_frame(0);
            assert_eq!(scroll_indicator_origin(&frame, 124), Point { x: 228, y: 124 });
            comm.presentation.activity_frame = frame.frame();
            scroll_indicator(context, comm, 6, 123);
            scroll_indicator(context, comm, 7, 225);
            context.flush();
            comm.presentation.activity_frame = uqm_engine::graphics::drawable::FrameId::NULL;
        });
    }
    #[test]
    #[serial_test::serial]
    fn text_wraps_at_the_measured_word_boundary_with_the_real_font() {
        super::super::sis_graphics_font_tests::with_display(
            "comm::sis_graphics::tests::text_wraps_at_the_measured_word_boundary_with_the_real_font",
            |comm, context| {
                let font = player_font(comm);
                let baseline = Point { x: 10, y: 135 };
                let mut text = Text::new(c"AAA AAA", baseline, 0, u16::MAX);
                let width = font.measure(text.text, baseline, uqm_engine::graphics::geometry::TextAlign::Left, u16::MAX).rect.extent.width;
                let line = text.fit(&font, width, u16::MAX);
                assert!(!line.end_of_line);
                assert_eq!((line.characters, line.next), (3, 4));
                text.draw(context, &font, Color { r: 255, g: 255, b: 255, a: 255 });
                context.flush();
                let frame = context.target();
                assert!((125..145).any(|y| (0..100).any(|x| {
                    let pixel = frame.pixel(Point { x, y });
                    pixel.r == 255 && pixel.g == 255 && pixel.b == 255
                })));
                text.text = suffix(text.text, line.next);
                text.char_count = u16::MAX;
                let last = text.fit(&font, width, u16::MAX);
                assert!(last.end_of_line);
                assert_eq!((last.characters, last.next), (3, 3));
            },
        );
    }
}
