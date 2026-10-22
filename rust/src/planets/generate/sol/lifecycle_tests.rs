use super::*;
use crate::c_bindings::global_state::IP_GROUP;
use crate::planets::generate::default::Npcs;
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

pub(super) type Trace = Rc<RefCell<Vec<String>>>;

#[derive(Default)]
pub(super) struct State {
    pub flags: BTreeMap<&'static str, u32>,
    pub words: BTreeMap<&'static str, u32>,
    pub trace: Trace,
}

impl Story for State {
    fn flag(&self, name: &'static str) -> u32 {
        self.trace.borrow_mut().push(format!("read:{name}"));
        self.flags.get(name).copied().unwrap_or(0)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        self.trace.borrow_mut().push(format!("flag:{name}:{value}"));
        self.flags.insert(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        self.trace.borrow_mut().push(format!("read-word:{name}"));
        self.words.get(name).copied().unwrap_or(0)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        self.trace.borrow_mut().push(format!("word:{name}:{value}"));
        self.words.insert(name, value);
    }
}

#[derive(Default)]
pub(super) struct QueueHost {
    pub trace: Trace,
    pub reference: u32,
    pub groups: Vec<IP_GROUP>,
    pub restored: Option<Vec<IP_GROUP>>,
    pub init: VecDeque<bool>,
    pub built: u16,
    pub rebellion: bool,
    pub race: i16,
}

impl QueueHost {
    pub fn call(&self, call: impl Into<String>) {
        self.trace.borrow_mut().push(call.into());
    }
}

impl Npcs for QueueHost {
    fn init_ip_groups(&mut self) -> bool {
        self.call("init");
        self.init.pop_front().expect("unexpected GROUP_INIT_IP")
    }
    fn clear_battle_group_ref(&mut self) {
        self.call("clear-ref");
        self.reference = 0;
    }
    fn build_groups(&mut self) {
        self.call("build");
    }
    fn load_ip_groups(&mut self) {
        self.call("load");
        if let Some(groups) = self.restored.take() {
            self.groups = groups;
        }
    }
    fn save_ip_groups(&mut self) {
        self.call("save");
    }
    fn reinit_npc_built_ship_q(&mut self) {
        self.call("reset-built");
        self.built = 0;
    }
    fn reinit_ip_group_q(&mut self) {
        self.call("reset-ip");
        self.groups.clear();
    }
    fn yehat_civil_war(&self) -> bool {
        self.call("civil-war");
        self.rebellion
    }
    fn encounter_race(&self) -> i16 {
        self.call("encounter-race");
        self.race
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        self.call("visit");
        for group in &mut self.groups {
            visit(group);
        }
    }
}

impl FleetGroups for QueueHost {
    fn battle_group_ref(&self) -> u32 {
        self.call("get-ref");
        self.reference
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        self.call(format!("ref:{value}"));
        self.reference = value;
    }
    fn clone_ship_into_built_queue(&mut self, ship: u8) {
        self.call(format!("clone:{ship}"));
        self.built += 1;
    }
    fn add_new_group(&mut self) -> u32 {
        self.call("add");
        assert_eq!(self.built, 1);
        0x12345678
    }
    fn reinit_built_queue(&mut self) {
        self.reinit_npc_built_ship_q();
    }
    fn head_ip_group(&mut self) -> Option<&mut IP_GROUP> {
        self.call("head");
        self.groups.first_mut()
    }
}

impl orbital::Queues for QueueHost {
    fn built_ship_count(&self) -> u16 {
        self.call("count-built");
        self.built
    }
}

pub(super) fn group(race: u8, task: u8) -> IP_GROUP {
    let mut group: IP_GROUP = unsafe { std::mem::zeroed() };
    group.race_id = race;
    group.task = task;
    group.sys_loc = 9;
    group.dest_loc = 8;
    group.loc.x = 77;
    group.loc.y = -91;
    group.group_counter = 233;
    group.in_system = 19;
    group.orbit_pos = 21;
    group.group_id = 25;
    group
}

pub(super) fn pair() -> (State, QueueHost) {
    let state = State::default();
    let host = QueueHost {
        trace: state.trace.clone(),
        ..QueueHost::default()
    };
    (state, host)
}

#[test]
fn probe_creation_persists_reference_and_initial_orbit_changes_only_c_fields() {
    for saved in [0, 0xabcdef01] {
        let (mut story, mut host) = pair();
        story.words.insert("URQUAN_PROBE_GRPOFFS0", saved);
        story.words.insert("unrelated", 42);
        host.init.push_back(true);
        host.groups = vec![group(23, 0xff), group(11, 0xfe)];
        let mut expected = host.groups.clone();
        expected[0].task = 0;
        expected[0].sys_loc = 3;
        expected[0].dest_loc = 3;
        expected[0].loc.x = 0;
        expected[0].loc.y = 0;
        expected[0].group_counter = 0;
        assert!(init_npcs(&mut story, &mut host));
        assert_eq!(format!("{:?}", host.groups), format!("{expected:?}"));
        let reference = if saved == 0 { 0x12345678 } else { saved };
        assert_eq!(host.reference, reference);
        assert_eq!(story.words["URQUAN_PROBE_GRPOFFS0"], reference);
        assert_eq!(story.words["unrelated"], 42);
        assert_eq!(host.built, 0);
        let mut trace = vec![
            "read-word:URQUAN_PROBE_GRPOFFS0".to_owned(),
            format!("ref:{saved}"),
            "get-ref".into(),
        ];
        if saved == 0 {
            trace.extend(
                [
                    "clone:23",
                    "add",
                    "ref:305419896",
                    "reset-built",
                    "word:URQUAN_PROBE_GRPOFFS0:305419896",
                ]
                .map(str::to_owned),
            );
        }
        trace.extend(["read:PROBE_MESSAGE_DELIVERED", "init", "head"].map(str::to_owned));
        assert_eq!(*story.trace.borrow(), trace);
    }
}

#[test]
fn probe_init_short_circuits_and_repeats_default_group_load_as_c_does() {
    for delivered in [0, 1] {
        for probe_load in [false, true] {
            for head in [false, true] {
                for default_load in [false, true] {
                    if delivered == 0 && probe_load && head {
                        continue;
                    }
                    let (mut story, mut host) = pair();
                    story.words.insert("URQUAN_PROBE_GRPOFFS0", 71);
                    story.flags.insert("PROBE_MESSAGE_DELIVERED", delivered);
                    if head {
                        host.groups.push(group(23, 0xfe));
                    }
                    if delivered == 0 {
                        host.init.push_back(probe_load);
                    }
                    host.init.push_back(default_load);
                    let before = format!("{:?}", host.groups);
                    assert!(init_npcs(&mut story, &mut host));
                    assert_eq!(format!("{:?}", host.groups), before);
                    assert!(host.init.is_empty());
                    assert_eq!(host.reference, if default_load { 71 } else { 0 });
                    assert_eq!(story.words["URQUAN_PROBE_GRPOFFS0"], 71);
                    let mut trace = vec![
                        "read-word:URQUAN_PROBE_GRPOFFS0",
                        "ref:71",
                        "get-ref",
                        "read:PROBE_MESSAGE_DELIVERED",
                    ];
                    if delivered == 0 {
                        trace.push("init");
                        if probe_load {
                            trace.push("head");
                        }
                    }
                    trace.push("init");
                    if !default_load {
                        trace.extend(["clear-ref", "build"]);
                    }
                    assert_eq!(*story.trace.borrow(), trace);
                }
            }
        }
    }
}

#[test]
fn probe_reinit_restores_first_and_changes_only_a_reforming_head_probe() {
    for reference in [0, 99] {
        for head in [false, true] {
            for race in [11, 23] {
                for task in [0, 2, 4, 8, 0xff] {
                    let (mut story, mut host) = pair();
                    story.flags.insert("CHMMR_BOMB_STATE", 2);
                    host.reference = reference;
                    let restored = if head {
                        vec![group(race, task), group(23, 8)]
                    } else {
                        vec![]
                    };
                    let mut expected = restored.clone();
                    if reference != 0 && head && race == 23 && task & 8 != 0 {
                        expected[0].task = 6;
                        expected[0].dest_loc = 0;
                    }
                    host.restored = Some(restored);
                    assert!(reinit_npcs(&story, &mut host));
                    assert_eq!(format!("{:?}", host.groups), format!("{expected:?}"));
                    let mut trace = vec!["read:CHMMR_BOMB_STATE", "load", "civil-war", "get-ref"];
                    if reference != 0 {
                        trace.push("head");
                    }
                    assert_eq!(*story.trace.borrow(), trace);
                    assert_eq!(host.reference, reference);
                    assert!(story.words.is_empty());
                }
            }
        }
    }
}

#[test]
fn reinit_keeps_default_yehat_rebellion_before_sol_probe_check() {
    let (story, mut host) = pair();
    host.reference = 99;
    host.rebellion = true;
    host.race = -1;
    host.groups = vec![group(23, 8), group(11, 4)];
    assert!(reinit_npcs(&story, &mut host));
    assert_eq!(host.groups[0].task, 6);
    assert_eq!(host.groups[1].task, 6);
    assert_eq!(host.groups[1].dest_loc, 0);
    assert_eq!(
        *story.trace.borrow(),
        [
            "read:CHMMR_BOMB_STATE",
            "load",
            "civil-war",
            "encounter-race",
            "visit",
            "get-ref",
            "head"
        ]
    );
}

#[test]
fn bomb_state_three_resets_only_reference_and_ip_queue_and_asserts_built_empty() {
    for bomb in [0, 1, 2, 3, 4] {
        for built in [0, 1] {
            let (mut story, mut host) = pair();
            story.flags.insert("CHMMR_BOMB_STATE", bomb);
            story.words.insert("URQUAN_PROBE_GRPOFFS0", 321);
            host.reference = 99;
            host.built = built;
            host.groups = vec![group(23, 8)];
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                reinit_npcs(&story, &mut host)
            }));
            assert_eq!(result.is_err(), bomb == 3 && built != 0);
            assert_eq!(host.built, built);
            assert_eq!(story.words["URQUAN_PROBE_GRPOFFS0"], 321);
            if bomb == 3 {
                assert!(host.groups.is_empty());
                assert_eq!(host.reference, 0);
                assert_eq!(
                    *story.trace.borrow(),
                    [
                        "read:CHMMR_BOMB_STATE",
                        "clear-ref",
                        "reset-ip",
                        "count-built"
                    ]
                );
            } else {
                assert_eq!(host.reference, 99);
                assert_eq!(host.groups[0].task, 6);
            }
        }
    }
}

#[test]
fn delivered_message_still_creates_and_persists_missing_probe_before_default_init() {
    let (mut story, mut host) = pair();
    story.flags.insert("PROBE_MESSAGE_DELIVERED", 1);
    host.init.push_back(true);
    host.groups.push(group(11, 0xfe));
    let before = format!("{:?}", host.groups);
    assert!(init_npcs(&mut story, &mut host));
    assert_eq!(host.reference, 0x12345678);
    assert_eq!(story.words["URQUAN_PROBE_GRPOFFS0"], 0x12345678);
    assert_eq!(format!("{:?}", host.groups), before);
    assert!(host.init.is_empty());
    assert_eq!(
        *story.trace.borrow(),
        [
            "read-word:URQUAN_PROBE_GRPOFFS0",
            "ref:0",
            "get-ref",
            "clone:23",
            "add",
            "ref:305419896",
            "reset-built",
            "word:URQUAN_PROBE_GRPOFFS0:305419896",
            "read:PROBE_MESSAGE_DELIVERED",
            "init"
        ]
    );
}
