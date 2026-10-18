//! Story gating and host effects of `GenerateChmmr_generateOrbital`, checked
//! against a recording site and a fake game state, and unleashed or sealed.

use std::collections::HashMap;

use super::{enter_orbit, ChmmrSite, CHMMR_CONTACT_FLAGS, ILWRATH_AMBUSH_FLAGS};
use crate::comm::dispatch::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::planets::generate::special::{Body, Story};

const IN_INTERPLANETARY: u16 = 4;
const HOME: Body = Body::Planet(1);
const STARBASE: Body = Body::Moon { planet: 1, moon: 0 };

#[derive(Default)]
struct FakeStory(HashMap<&'static str, u32>);

impl FakeStory {
    fn with(flags: &[(&'static str, u32)]) -> Self {
        Self(flags.iter().copied().collect())
    }
}

impl Story for FakeStory {
    fn flag(&self, name: &'static str) -> u32 {
        self.0.get(name).copied().unwrap_or(0)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        self.0.insert(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        self.flag(name)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        self.set_flag(name, value);
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Call {
    TalkToChmmr,
    StartSphereTracking,
    SaveIpGroups,
    ReinitIpGroupQ,
    CountBuiltShips,
    CloneIlwrathFleet,
    SetActivity(u16),
    SetGlobalFlagsAndData(u8),
    TalkToIlwrath,
    ReinitBuiltShipQ,
    LoadIpGroups,
    VisitStarbase,
}

struct RecordingSite {
    tracking: bool,
    built_ships: u16,
    activity: u16,
    activity_after_talk: Option<u16>,
    calls: std::cell::RefCell<Vec<Call>>,
}

impl RecordingSite {
    fn new(tracking: bool) -> Self {
        Self {
            tracking,
            built_ships: 0,
            activity: IN_INTERPLANETARY,
            activity_after_talk: None,
            calls: std::cell::RefCell::default(),
        }
    }

    fn record(&self, call: Call) {
        self.calls.borrow_mut().push(call);
    }
}

impl ChmmrSite for RecordingSite {
    fn talk_to_chmmr(&mut self) {
        self.record(Call::TalkToChmmr);
    }
    fn start_ilwrath_sphere_tracking(&mut self) -> bool {
        self.record(Call::StartSphereTracking);
        self.tracking
    }
    fn save_ip_groups(&mut self) {
        self.record(Call::SaveIpGroups);
    }
    fn reinit_ip_group_q(&mut self) {
        self.record(Call::ReinitIpGroupQ);
    }
    fn built_ship_count(&self) -> u16 {
        self.record(Call::CountBuiltShips);
        self.built_ships
    }
    fn clone_ilwrath_fleet_into_built_queue(&mut self) {
        self.record(Call::CloneIlwrathFleet);
    }
    fn current_activity(&self) -> u16 {
        self.activity
    }
    fn set_current_activity(&mut self, activity: u16) {
        self.record(Call::SetActivity(activity));
        self.activity = activity;
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.record(Call::SetGlobalFlagsAndData(value));
    }
    fn talk_to_ilwrath(&mut self) {
        self.record(Call::TalkToIlwrath);
        if let Some(after) = self.activity_after_talk {
            self.activity = after;
        }
    }
    fn reinit_built_ship_q(&mut self) {
        self.record(Call::ReinitBuiltShipQ);
    }
    fn load_ip_groups(&mut self) {
        self.record(Call::LoadIpGroups);
    }
    fn visit_starbase(&mut self) {
        self.record(Call::VisitStarbase);
    }
}

#[test]
fn unleashed_chmmr_talk_without_the_bomb_leaves_the_activity_alone() {
    for bomb in [0, 1, 3] {
        let story = FakeStory::with(&[("CHMMR_UNLEASHED", 1), ("CHMMR_BOMB_STATE", bomb)]);
        let mut site = RecordingSite::new(true);
        assert!(enter_orbit(HOME, &story, &mut site));
        assert_eq!(
            *site.calls.borrow(),
            [
                Call::SetGlobalFlagsAndData(CHMMR_CONTACT_FLAGS),
                Call::TalkToChmmr
            ],
            "bomb state {bomb}"
        );
        assert_eq!(site.activity, IN_INTERPLANETARY);
    }
}

#[test]
fn unleashed_chmmr_talk_with_the_bomb_delivered_ends_the_interplanetary_visit() {
    let story = FakeStory::with(&[("CHMMR_UNLEASHED", 1), ("CHMMR_BOMB_STATE", 2)]);
    let mut site = RecordingSite::new(true);
    assert!(enter_orbit(HOME, &story, &mut site));
    assert_eq!(
        *site.calls.borrow(),
        [
            Call::SetGlobalFlagsAndData(CHMMR_CONTACT_FLAGS),
            Call::TalkToChmmr,
            Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY)
        ]
    );
}

#[test]
fn unleashed_chmmr_never_meet_the_ilwrath() {
    let story = FakeStory::with(&[("CHMMR_UNLEASHED", 1), ("SUN_DEVICE_ON_SHIP", 1)]);
    let mut site = RecordingSite::new(true);
    assert!(enter_orbit(HOME, &story, &mut site));
    assert!(!site.calls.borrow().contains(&Call::StartSphereTracking));
}

/// Everything up to and including the Ilwrath conversation.
fn ambush_prefix() -> Vec<Call> {
    vec![
        Call::StartSphereTracking,
        Call::SaveIpGroups,
        Call::ReinitIpGroupQ,
        Call::CountBuiltShips,
        Call::CloneIlwrathFleet,
        Call::SetGlobalFlagsAndData(ILWRATH_AMBUSH_FLAGS),
        Call::SetActivity(IN_INTERPLANETARY | START_INTERPLANETARY),
        Call::TalkToIlwrath,
    ]
}

fn ambush_story() -> FakeStory {
    FakeStory::with(&[("SUN_DEVICE_ON_SHIP", 1)])
}

#[test]
fn the_ilwrath_ambush_that_ends_normally_puts_the_interplanetary_world_back() {
    let mut site = RecordingSite::new(true);
    assert!(enter_orbit(HOME, &ambush_story(), &mut site));

    let mut expected = ambush_prefix();
    expected.extend([
        Call::SetActivity(IN_INTERPLANETARY),
        Call::ReinitBuiltShipQ,
        Call::LoadIpGroups,
    ]);
    assert_eq!(*site.calls.borrow(), expected);
}

#[test]
fn the_ilwrath_ambush_keeps_the_conversation_state_when_the_game_aborts_or_loads() {
    for flag in [CHECK_ABORT, CHECK_LOAD, CHECK_ABORT | CHECK_LOAD] {
        let mut site = RecordingSite::new(true);
        let during = IN_INTERPLANETARY | START_INTERPLANETARY | flag;
        site.activity_after_talk = Some(during);
        assert!(enter_orbit(HOME, &ambush_story(), &mut site));
        assert_eq!(*site.calls.borrow(), ambush_prefix(), "flag {flag:#x}");
        assert_eq!(site.activity, during);
    }
}

#[test]
fn the_ilwrath_ambush_needs_the_built_ship_queue_to_be_empty() {
    let mut site = RecordingSite::new(true);
    site.built_ships = 1;
    let visit = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        enter_orbit(HOME, &ambush_story(), &mut site)
    }));
    assert!(visit.is_err());
}

#[test]
fn the_ambush_needs_the_sun_device_the_unfooled_ilwrath_and_a_trackable_sphere() {
    type Flags = &'static [(&'static str, u32)];
    let cases: [(Flags, bool, bool); 5] = [
        (&[], true, false),
        (
            &[("SUN_DEVICE_ON_SHIP", 1), ("ILWRATH_DECEIVED", 1)],
            true,
            false,
        ),
        (&[("ILWRATH_DECEIVED", 1)], true, false),
        (&[("SUN_DEVICE_ON_SHIP", 1)], false, false),
        (&[("SUN_DEVICE_ON_SHIP", 1)], true, true),
    ];
    for (flags, tracking, ambushed) in cases {
        let mut site = RecordingSite::new(tracking);
        assert_eq!(
            enter_orbit(HOME, &FakeStory::with(flags), &mut site),
            ambushed,
            "{flags:?} tracking {tracking}"
        );
        let tracked = site.calls.borrow().contains(&Call::StartSphereTracking);
        let expect_tracked =
            flags.contains(&("SUN_DEVICE_ON_SHIP", 1)) && !flags.contains(&("ILWRATH_DECEIVED", 1));
        assert_eq!(tracked, expect_tracked, "{flags:?} tracking {tracking}");
        if !ambushed {
            assert_eq!(site.activity, IN_INTERPLANETARY);
        }
    }
}

#[test]
fn the_starbase_moon_is_a_report_and_nothing_else() {
    let mut site = RecordingSite::new(true);
    assert!(enter_orbit(STARBASE, &ambush_story(), &mut site));
    assert_eq!(*site.calls.borrow(), [Call::VisitStarbase]);
}

#[test]
fn other_worlds_touch_nothing() {
    let story = FakeStory::with(&[("CHMMR_UNLEASHED", 1), ("SUN_DEVICE_ON_SHIP", 1)]);
    for body in [
        Body::Planet(0),
        Body::Planet(2),
        Body::Moon { planet: 1, moon: 1 },
        Body::Moon { planet: 0, moon: 0 },
    ] {
        let mut site = RecordingSite::new(true);
        assert!(!enter_orbit(body, &story, &mut site), "{body:?}");
        assert!(site.calls.borrow().is_empty());
    }
}
