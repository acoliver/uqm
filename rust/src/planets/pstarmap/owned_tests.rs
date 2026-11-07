use super::*;
use crate::engine::{
    battle::{input::InputContext, Battle},
    input::{frames::InputState, InputControls, InputRuntime},
    observe::{CampaignView, Observer, Presented, Unobserved},
    Game,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub(super) struct Driver {
    input: InputSession,
    ticks: Cell<u32>,
    delays: RefCell<Vec<u32>>,
    deadlines: RefCell<Vec<u32>>,
    updates: Cell<usize>,
    interrupt_after: Cell<Option<usize>>,
}
impl Driver {
    pub(super) fn press(&self, keys: &[usize]) {
        let mut state = InputState::default();
        for &key in keys {
            state.menu[key] = 1;
        }
        let ticks = self.ticks.get() + 1000;
        self.ticks.set(ticks);
        self.input.advance(state, state, ticks);
    }
}
impl InputRuntime for Driver {
    fn input(&self) -> &InputSession {
        &self.input
    }
    fn observer(&self) -> Box<dyn Observer<Option<CampaignView>>> {
        Box::new(Unobserved)
    }
    fn yield_poll(&self, _: &mut Cx<'_>) {}
    fn sleep(&self, _: &mut Cx<'_>, ticks: u32) {
        self.delays.borrow_mut().push(ticks);
    }
    fn sleep_until(&self, _: &mut Cx<'_>, deadline: u32) {
        self.deadlines.borrow_mut().push(deadline);
    }
    fn flush(&self) {}
    fn confirm_exit(&self, _: &mut Cx<'_>) -> bool {
        false
    }
    fn update(&self, _: &mut Cx<'_>) {
        let updates = self.updates.get() + 1;
        self.updates.set(updates);
        if self.interrupt_after.get() == Some(updates) {
            self.press(&[9]);
        }
    }
    fn battle_input(&self, _: &InputContext) -> u8 {
        0
    }
    fn run(&self, cx: &mut Cx<'_>, io: &mut Io<'_>, reset: bool, frame: &mut dyn Frame) {
        if reset {
            assert_eq!(self.input.repeat_config(), (70, 420, 70, false));
        }
        let cancel = if reset {
            crate::navigation::abi::KEY_MENU_EDIT_CANCEL
        } else {
            crate::navigation::abi::KEY_MENU_CANCEL
        } as usize;
        for keys in [&[][..], &[cancel][..]] {
            self.press(keys);
            frame.admit(cx, io, InputView::BeforeUpdate);
            if !frame.frame(cx, io) {
                return;
            }
        }
        panic!("cancel must finish the fixture's input loop");
    }
}
#[derive(Default)]
struct Recorded(Vec<Option<StarMapView>>);
impl Observer<Option<CampaignView>> for Recorded {
    fn input_admitted(&mut self, _: &InputView, _: &Option<CampaignView>) -> Injected {
        Injected::default()
    }
    fn checkpoint(&mut self, view: &CheckpointView<'_>) {
        if let CheckpointView::StarMap(view) = view {
            self.0.push(*view);
        }
    }
    fn presented(&mut self, _: &mut Presented) {}
}

pub(super) fn with_map(test: impl FnOnce(&mut StarMap, &mut Cx<'_>, &Rc<Driver>)) {
    crate::callback::ffi::Alarm_init();
    let mut game = Game::new();
    let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
    let misc = crate::graphics::drawable_abi::LoadedDrawable::create(8, 1, 1, 70).unwrap();
    game.kernel.misc_data_frame = misc.lease().frame(0).unwrap();
    game.kernel.space_context = game.kernel.off_screen_context;
    game.kernel.screen_context = game.kernel.off_screen_context;
    let context = ContextLease::require(game.kernel.space_context);
    context.set_frame(ContextFrame::retain(NonNull::new(game.kernel.screen)));
    context.set_clip(RECT {
        corner: POINT { x: 7, y: 13 },
        extent: EXTENT {
            width: 242,
            height: 227,
        },
    });
    game.world.story = Some(crate::engine::world::story::StoryState::new());
    game.world.set_catalog(std::borrow::Cow::Borrowed(
        &crate::planets::tables::STARMAP_ARRAY,
    ));
    game.world.current_star = Some(crate::engine::StarId(0));
    game.world.set_activity(4);
    game.world.data.game.autopilot = POINT { x: -1, y: -1 };
    let driver = Rc::new(Driver {
        input: InputSession::new(InputControls::default(), game.kernel.configuration.clone()),
        ticks: Cell::new(0),
        delays: RefCell::new(Vec::new()),
        deadlines: RefCell::new(Vec::new()),
        updates: Cell::new(0),
        interrupt_after: Cell::new(None),
    });
    game.kernel.install_input_runtime(driver.clone());
    {
        let mut battle = Battle::new();
        let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || 0);
        let mut map = StarMap::new(&cx);
        map.transition = false;
        test(&mut map, &mut cx, &driver);
    }
    game.kernel.sis_highlight.context.borrow_mut().take();
    // SAFETY: this serialized fixture owns the native drawing queue and resources.
    unsafe { crate::graphics::dcqueue_abi::TFB_FlushGraphics() };
    drop(context);
    drop(misc);
    display.finish(game);
    crate::callback::ffi::Alarm_uninit();
}

#[test]
#[serial_test::serial]
fn cursor_frames_publish_visit_lifecycle_zoom_and_autopilot() {
    with_map(|map, cx, driver| {
        let mut recorded = Recorded::default();
        let mut io = Io {
            observer: &mut recorded,
        };
        driver.press(&[10]);
        assert!(map.frame(cx, &mut io));
        assert!(map.initialized, "the first frame ignores cancel");
        let initial = map.state.cursor;
        driver.press(&[8]);
        assert!(map.frame(cx, &mut io));
        assert!(map.state.cursor.x > initial.x);
        driver.press(&[16, 6]);
        assert!(map.frame(cx, &mut io));
        assert_eq!(map.state.projection.zoom, 1);
        assert_eq!(*driver.delays.borrow(), [105]);
        assert_eq!(driver.deadlines.borrow().len(), 2);
        map.admit(cx, &mut io, InputView::BeforeUpdate);
        let selected = map.state.cursor;
        driver.press(&[9]);
        assert!(map.frame(cx, &mut io));
        assert_eq!(cx.world.data.game.autopilot, selected);
        driver.press(&[10]);
        assert!(!map.frame(cx, &mut io));
        assert!(!map.initialized);
        assert!(recorded
            .0
            .iter()
            .any(|view| view.is_some_and(|view| view.projection.zoom == 1)));
        assert_eq!(recorded.0.last(), Some(&None));
    });
}

#[test]
#[serial_test::serial]
fn portal_overrides_a_snapped_star_and_quasispace_rejects_search() {
    if !crate::campaign_display_test::own_graphics_process(concat!(
        module_path!(),
        "::portal_overrides_a_snapped_star_and_quasispace_rejects_search"
    )) {
        return;
    }
    with_map(|map, cx, driver| {
        cx.world.set_story_state(StoryKey::ARILOU_SPACE, 1);
        map.state.cursor = ARILOU_SPACE;
        map.update_cursor_info(cx.kernel, cx.world);
        assert_eq!(map.state.cursor, ARILOU_SPACE);
        assert_eq!(map.last_label.as_c_str(), cx.kernel.game_text(132));
        cx.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 2);
        map.state.cursor = QUASI_SPACE;
        map.update_cursor_info(cx.kernel, cx.world);
        assert_eq!(map.state.cursor, QUASI_SPACE);
        map.initialized = true;
        driver.press(&[21]);
        let mut observer = Unobserved;
        assert!(map.frame(
            cx,
            &mut Io {
                observer: &mut observer
            }
        ));
        assert!(driver.delays.borrow().is_empty());
        cx.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 0);
        map.state.cursor = cx.world.catalog()[0].star_pt;
        map.update_cursor_info(cx.kernel, cx.world);
        driver.input().configure_repeat(14, 105, 7, true, 0);
        driver.press(&[]);
        driver.press(&[21]);
        let saved_cursor = map.state.cursor;
        assert!(map.frame(
            cx,
            &mut Io {
                observer: &mut observer
            }
        ));
        assert_eq!(map.state.cursor, saved_cursor);
        assert_eq!(driver.input().menu_sounds(), (0, 0));
        assert!(cx.kernel.sis_highlight.context.borrow().is_none());
    });
}

#[test]
#[serial_test::serial]
#[cfg(feature = "debug-process")]
fn instant_select_uses_the_owned_option_and_campaign() {
    with_map(|map, cx, driver| {
        map.initialized = true;
        cx.kernel.instant_move = true;
        driver.press(&[9]);
        let mut observer = Unobserved;
        assert!(!map.frame(
            cx,
            &mut Io {
                observer: &mut observer
            }
        ));
        assert_ne!(cx.world.activity() & 0x0800, 0);
        assert_eq!(cx.world.data.game.autopilot, map.state.cursor);
        map.initialized = true;
        cx.world.set_activity(3);
        driver.press(&[]);
        driver.press(&[9]);
        assert!(!map.frame(
            cx,
            &mut Io {
                observer: &mut observer
            }
        ));
        assert_eq!(cx.world.data.game.autopilot, POINT { x: -1, y: -1 });
        assert_eq!(cx.world.current_star, Some(crate::engine::StarId(0)));
    });
}

fn install_fleets(world: &mut World) -> Vec<LinkId<Fleet>> {
    let labels = crate::strings::owned_table::LoadedTextTable::fixture(vec![b"A\0".to_vec(); 2]);
    let resources = crate::engine::battle::link_queue::ShipPickerResources::new(
        labels.entry(0),
        crate::engine::battle::frames::FrameSelection::ABSENT,
    );
    world.retain_fleet_resources(0, resources);
    let mut queue = world.avail_races();
    [(40, 80), (80, 40), (0, 20)]
        .into_iter()
        .map(|(known, actual)| {
            let id = queue.alloc().unwrap();
            *queue.get_mut(id) = FleetInfo {
                species_id: 0,
                known_strength: known,
                actual_strength: actual,
                known_loc: POINT { x: 3000, y: 4000 },
                loc: POINT { x: 3200, y: 4100 },
                ..FleetInfo::default()
            };
            queue.put(id);
            id
        })
        .collect()
}

#[test]
#[serial_test::serial]
fn sphere_animation_updates_each_successor_and_preserves_unknown_fleets() {
    for interrupt in [false, true] {
        with_map(|map, cx, driver| {
            let ids = install_fleets(cx.world);
            driver.interrupt_after.set(interrupt.then_some(2));
            map.update_map(cx, driver.input());
            let queue = cx.world.avail_races_ref();
            assert_eq!(queue.count(), 3);
            assert_eq!(queue.succ(ids[0]), Some(ids[1]));
            for id in &ids[..2] {
                let fleet = queue.get(*id);
                assert_eq!(fleet.known_loc, fleet.loc);
                assert_eq!(fleet.known_strength, fleet.actual_strength);
            }
            assert_eq!(queue.get(ids[2]).known_strength, 0);
            assert_eq!(queue.get(ids[2]).known_loc, POINT { x: 3000, y: 4000 });
            if interrupt {
                assert_eq!(
                    driver.updates.get(),
                    2,
                    "interruption stops further button polls"
                );
            } else {
                assert!(driver.updates.get() > 2);
            }
        });
    }
}

#[test]
#[serial_test::serial]
fn visits_clear_observations_and_same_star_destinations_without_retaining_input() {
    with_map(|map, cx, driver| {
        cx.world.data.game.autopilot = map.universe;
        let mut recorded = Recorded::default();
        assert_eq!(
            star_map(
                cx,
                &mut Io {
                    observer: &mut recorded
                }
            ),
            0
        );
        assert_eq!(cx.world.data.game.autopilot, POINT { x: -1, y: -1 });
        assert!(recorded.0.iter().any(Option::is_some));
        assert_eq!(recorded.0.last(), Some(&None));
        assert_eq!(driver.input.menu_sounds(), (15, 16));
        let destination = POINT { x: 5000, y: 5000 };
        cx.world.data.game.autopilot = destination;
        assert_eq!(
            star_map(
                cx,
                &mut Io {
                    observer: &mut recorded
                }
            ),
            1
        );
        assert_eq!(cx.world.data.game.autopilot, destination);
        assert_eq!(recorded.0.last(), Some(&None));
    });
}

#[test]
#[serial_test::serial]
fn saved_location_names_use_normal_space_without_mutating_the_campaign() {
    with_map(|_, cx, _| {
        cx.world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 2);
        let star = &cx.world.catalog()[0];
        let (name, point) =
            super::super::starmap::naming::name_near(cx.kernel, cx.world.catalog(), star.star_pt)
                .unwrap();
        assert_eq!(point, star.star_pt);
        assert_eq!(
            name,
            super::super::starmap::naming::cluster_name(
                cx.kernel,
                star,
                crate::navigation::abi::STAR_NUMBER_BASE as usize,
            )
        );
        assert_eq!(cx.world.story_state(StoryKey::ARILOU_SPACE_SIDE), 2);
        assert!(super::super::starmap::naming::name_near(
            cx.kernel,
            cx.world.catalog(),
            POINT { x: 0, y: 0 },
        )
        .is_none());
    });
}
