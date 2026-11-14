use super::live::discovery_report;
use crate::engine::{
    battle::{input::InputContext, Battle, Cx},
    input::{frames::InputState, Frame, InputControls, InputRuntime, InputSession},
    observe::{
        CampaignView, CheckpointView, Injected, InputView, Io, Observer, Presented, Unobserved,
    },
    strings::owned_table::LoadedTextTable,
    world::generation::ReportSlot,
    Game,
};
use crate::graphics::{
    batch_depth::depth,
    context_abi::{ContextFrame, ContextLease},
    drawable_abi::LoadedDrawable,
    native_abi::{Color, POINT, TEXT_ALIGN_ALIGN_LEFT},
};
use crate::planet_side::report_adapter::{DiscoveryReport, DiscoveryReportPort, ReportDisposition};
use std::{
    cell::{Cell, RefCell},
    ptr::NonNull,
    rc::Rc,
};

struct Driver {
    input: InputSession,
    context: ContextLease,
    frame: ContextFrame,
    point: POINT,
    updates: Cell<usize>,
    flushes: Cell<usize>,
    pixels: RefCell<Vec<Color>>,
}

impl InputRuntime for Driver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {
        panic!("unexpected report yield")
    }
    fn sleep(&self, _: &mut Cx<'_>, ticks: u32) {
        assert_eq!(ticks, 840 / 40);
    }
    fn sleep_until(&self, _: &mut Cx<'_>, _: u32) {
        panic!("unexpected report deadline")
    }
    fn flush(&self) {
        self.flushes.set(self.flushes.get() + 1);
        self.input.clear_menu_state();
    }
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        panic!("unexpected exit confirmation")
    }
    fn update(&self, _: &mut Cx<'_>) {
        let index = self.updates.get();
        let pressed = [false, false, true, false, true, false][index];
        self.updates.set(index + 1);
        assert_eq!(depth(&self.context), 0, "report waited under a batch");
        self.context.flush();
        self.pixels.borrow_mut().push(self.frame.pixel(self.point));
        let mut state = InputState::default();
        state.menu[9] = i32::from(pressed);
        self.input.advance(state, state, (index as u32 + 1) * 1000);
    }
    fn run(&self, _: &mut Cx<'_>, _: &mut Io<'_>, _: bool, _: &mut dyn Frame) {
        panic!("report uses its supplied nested wait, not a second input loop");
    }
    fn battle_input(&self, _: &InputContext) -> u8 {
        panic!("unexpected battle input")
    }
}

#[derive(Default)]
struct Recording {
    active: bool,
    visibility: Vec<bool>,
    nested: usize,
}
impl Observer<Option<CampaignView>> for Recording {
    fn input_admitted(&mut self, view: &InputView, _: &Option<CampaignView>) -> Injected {
        assert!(self.active, "report input escaped its visibility scope");
        assert!(matches!(view, InputView::Nested));
        self.nested += 1;
        Injected::default()
    }
    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        if let CheckpointView::PlanetSideReport(active) = view {
            self.active = *active;
            self.visibility.push(*active);
        }
    }
    fn presented(&mut self, _: &mut Presented) {}
}

#[test]
#[serial_test::serial]
fn real_report_presents_before_nested_input_and_restores_retained_background() {
    for style in [0, 2] {
        let mut game = Game::new();
        crate::state::ffi::rust_init_game_state(&mut game.world);
        game.world.set_catalog(std::borrow::Cow::Borrowed(
            &crate::planets::tables::STARMAP_ARRAY,
        ));
        game.kernel
            .configuration
            .update_preferences(|preferences| preferences.fonts = style);
        let reports = super::typography::ReportResources::default();
        let display = crate::campaign_display_test::CampaignDisplay::install_with_resources(
            &mut game,
            || reports.load(),
        )
        .unwrap();
        game.kernel.space_context = game.kernel.off_screen_context;
        let context = ContextLease::require(game.kernel.space_context);
        let frame = ContextFrame::retain(NonNull::new(game.kernel.screen));
        context.set_frame(frame.clone());
        let typography = reports.snapshot().unwrap();
        let point = typography
            .font()
            .measure(c"A", POINT { x: 0, y: 2 }, TEXT_ALIGN_ALIGN_LEFT, 1)
            .rect
            .corner;
        let background = Color {
            r: 41,
            g: 83,
            b: 125,
            a: 255,
        };
        context.fill_rectangle(background, frame.rect());
        context.flush();
        let junk = LoadedDrawable::create(8, 1, 1, 22).unwrap();
        let junk_frame = junk.lease().context_frame(0);
        drop(junk);
        let driver = Rc::new(Driver {
            input: InputSession::new(InputControls::default(), game.kernel.configuration.clone()),
            context: context.clone(),
            frame: frame.clone(),
            point,
            updates: Cell::new(0),
            flushes: Cell::new(0),
            pixels: RefCell::default(),
        });
        game.kernel.install_input_runtime(driver.clone());
        let table = LoadedTextTable::new(vec![c"A A".to_owned()]);
        let mut slot = ReportSlot::new(table.entry(0));
        drop(table);
        let mut recording = Recording::default();
        {
            let mut battle = Battle::new();
            let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
            let io = RefCell::new(Io {
                observer: &mut recording,
            });
            let mut report = DiscoveryReport::new(&mut slot, &io, |text| {
                discovery_report(
                    &mut cx,
                    &mut io.borrow_mut(),
                    text,
                    Some(&context),
                    &typography,
                    junk_frame.clone(),
                );
            });
            assert!(report.present(ReportDisposition::Once).unwrap());
        }
        assert!(slot.current().is_none());
        assert_eq!(recording.visibility, [true, false]);
        assert_eq!(recording.nested, 1);
        assert!(!recording.active);
        assert_eq!(driver.updates.get(), 6);
        assert_eq!(driver.flushes.get(), 2);
        let pixels = driver.pixels.borrow();
        let expected = if style == 0 {
            Color {
                r: 0,
                g: 255,
                b: 0,
                a: 255,
            }
        } else {
            Color {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            }
        };
        assert_eq!(
            (pixels[0].r, pixels[0].g, pixels[0].b),
            (expected.r, expected.g, expected.b)
        );
        assert_eq!(
            (pixels[5].r, pixels[5].g, pixels[5].b),
            (background.r, background.g, background.b)
        );
        assert_eq!(depth(&context), 0);
        drop(pixels);
        drop((
            driver, context, frame, junk_frame, typography, reports, slot,
        ));
        display.finish(game);
    }
}
