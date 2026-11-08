//! Native battle diagnostics copied at input and simulation boundaries.

use crate::{battle::Cx, observe::Io};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BattleTick {
    pub phase: BattlePhase,
    pub activity: u16,
    pub keys: [[i32; crate::input_port::frames::FLIGHT_KEYS]; 2],
    pub pulsed: [[i32; crate::input_port::frames::FLIGHT_KEYS]; 2],
    pub menu: [i32; crate::input_port::frames::MENU_KEYS],
    pub menu_pulsed: [i32; crate::input_port::frames::MENU_KEYS],
    pub rng: u32,
    pub ditty: u32,
    pub ships: [Option<BattleShip>; 2],
    pub counters: [u8; 2],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BattlePhase {
    Input,
    Advanced,
}

/// Fixed-width ship sample: no handles, pointers, resources or element copies.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BattleShip {
    pub position: [i16; 2],
    pub velocity: [i32; 2],
    pub facing: u16,
    pub crew: u16,
    pub energy: u8,
    pub input: u8,
    pub status: u16,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReplacementGate {
    pub ditty_before: u32,
    /// None means the gate short-circuited without querying the audio worker.
    pub track_playing: Option<bool>,
    pub ditty_after: u32,
    pub blocked: bool,
}

pub fn replacement_gate(
    ditty: &mut u32,
    track_playing: impl FnOnce() -> bool,
    io: &mut Io<'_>,
) -> bool {
    let before = *ditty;
    let playing = (before != 0).then(track_playing);
    if let Some(playing) = playing {
        *ditty = u32::from(playing);
    }
    let blocked = *ditty != 0;
    io.observer.replacement_gate(&ReplacementGate {
        ditty_before: before,
        track_playing: playing,
        ditty_after: *ditty,
        blocked,
    });
    blocked
}

pub fn observe_tick(cx: &Cx<'_>, io: &mut Io<'_>, phase: BattlePhase) {
    if !io.observer.battle_diagnostics() || !is_combat_sample(cx.world.activity(), phase) {
        return;
    }
    io.observer
        .battle_tick(&capture(cx, cx.kernel.input_controls(), phase));
}

fn is_combat_sample(activity: u16, phase: BattlePhase) -> bool {
    // Hyperspace raises IN_BATTLE for travel; interplanetary combat also uses it.
    (phase == BattlePhase::Advanced || activity & (1 << 9) != 0) && activity as u8 != 3
}

fn capture(
    cx: &Cx<'_>,
    input: &crate::input_port::controls::InputControls,
    phase: BattlePhase,
) -> BattleTick {
    let current = input.current();
    let pulsed = input.pulsed();
    let templates = input.player_controls();
    let mut ships = [None; 2];
    let mut next = cx.elements().head();
    while let Some(id) = next {
        next = cx.elements().succ(id);
        let element = cx.element(id);
        // PLAYER_SHIP identifies fighters, rather than their projectiles or crew.
        if element.flags & (1 << 2) == 0 {
            continue;
        }
        if let crate::battle::Parent::Ship(owner) = cx.elements()[id].parent {
            let side = usize::from(owner.side());
            let state = cx.ship_state(owner);
            let (vx, vy) = element.velocity.get_current_components();
            ships[side] = Some(BattleShip {
                position: [element.current.location.x, element.current.location.y],
                velocity: [vx, vy],
                facing: state.facing,
                crew: cx.race_stats(owner).ship_info.crew_level,
                energy: cx.race_stats(owner).ship_info.energy_level,
                input: state.input,
                status: state.status,
            });
        }
    }
    BattleTick {
        phase,
        activity: cx.world.activity(),
        keys: std::array::from_fn(|side| current.key[templates[side] as usize]),
        pulsed: std::array::from_fn(|side| pulsed.key[templates[side] as usize]),
        menu: current.menu,
        menu_pulsed: pulsed.menu,
        rng: crate::math::random_state(),
        ditty: cx.kernel.ditty_playing,
        ships,
        counters: cx.b.counters,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observe::{CheckpointView, Injected, InputView, Observer, Presented};

    #[derive(Default)]
    struct Recorded(Vec<ReplacementGate>);
    impl Observer<Option<crate::observe::CampaignView>> for Recorded {
        fn input_admitted(
            &mut self,
            _: &InputView,
            _: &Option<crate::observe::CampaignView>,
        ) -> Injected {
            Injected::default()
        }
        fn checkpoint(&mut self, _: &CheckpointView<'_>) {}
        fn presented(&mut self, _: &mut Presented) {}
        fn replacement_gate(&mut self, gate: &ReplacementGate) {
            self.0.push(*gate);
        }
    }

    #[test]
    fn capture_reads_both_applied_templates_and_rng_without_consuming_it() {
        const CHILD: &str = "UQM_BATTLE_SNAPSHOT_TEST";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "observe::battle::tests::capture_reads_both_applied_templates_and_rng_without_consuming_it"])
                .env(CHILD, "1").status().unwrap();
            assert!(status.success());
            return;
        }
        let mut world = crate::World::new();
        let mut kernel = crate::Kernel::new();
        let mut battle = crate::battle::Battle::new();
        battle.counters = [3, 4];
        kernel.ditty_playing = 1;
        let input = crate::input_port::controls::InputControls::default();
        input.set_player_control(0, 4);
        input.set_player_control(1, 2);
        input.set_player_key(0, 3, true);
        input.set_player_key(1, 5, true);
        input.set_menu_key(9, true);
        input.advance(input.immediate(), input.immediate(), 42);
        let cx = crate::battle::Cx::new(&mut battle, &mut world, &mut kernel, crate::math::random);
        let tick = capture(&cx, &input, BattlePhase::Input);
        assert_eq!(tick.keys[0][3], 1);
        assert_eq!(tick.keys[1][5], 1);
        assert_eq!(tick.menu[9], 1);
        assert_eq!(tick.menu_pulsed[9], 1);
        assert_eq!(tick.pulsed[1][5], 1);
        assert_eq!(tick.counters, [3, 4]);
        assert_eq!(tick.ditty, 1);
        assert_eq!(tick.ships, [None; 2]);
        assert_eq!(tick.rng, crate::math::random_state());
        assert_eq!(tick, capture(&cx, &input, BattlePhase::Input));
        assert_eq!(
            serde_json::from_slice::<BattleTick>(&serde_json::to_vec(&tick).unwrap()).unwrap(),
            tick
        );
    }

    #[test]
    fn capture_reads_live_fighters_for_both_sides_and_ignores_projectiles() {
        let mut world = crate::World::new();
        let mut kernel = crate::Kernel::new();
        let mut battle = crate::battle::Battle::new();
        kernel.reset_element_capacity(3);
        for side in 0..2 {
            kernel.reset_ship_capacity(side, 1);
            let element = kernel.elements.alloc().unwrap();
            let mut ships = kernel.ships(side);
            let link = ships.alloc().unwrap();
            let owner = ships.ship_id(link);
            ships.put(link);
            let pool = kernel.races[side].fixture_pool();
            let key = pool.id_at(usize::from(owner.slot())).unwrap();
            let ship = pool.get_mut(key);
            let mut descriptor = crate::battle::record::RaceDescriptor::EMPTY;
            descriptor.ship_info.crew_level = 8 + side as u16;
            descriptor.ship_info.energy_level = 12 + side as u8;
            ship.descriptor = Some(Box::new(descriptor));
            ship.hShip = Some(element);
            ship.ship_facing = 3 + side as u16;
            ship.ship_input_state = 1 << side;
            ship.cur_status_flags = 17;
            let record = kernel.elements.fixture_record_mut(element);
            record.state_flags = 1 << 2;
            record.parent = crate::battle::Parent::Ship(owner);
            record.current.location.x = 100 + side as i16;
            record.current.location.y = -50;
            record.velocity.set_components(256, -512);
            kernel.elements.put(element);
        }
        let projectile = kernel.elements.alloc().unwrap();
        kernel.elements.fixture_record_mut(projectile).state_flags = 0;
        kernel.elements.put(projectile);
        let cx = crate::battle::Cx::new(&mut battle, &mut world, &mut kernel, crate::math::random);
        let tick = capture(&cx, &Default::default(), BattlePhase::Advanced);
        for (side, ship) in tick.ships.iter().enumerate() {
            assert_eq!(
                *ship,
                Some(BattleShip {
                    position: [100 + side as i16, -50],
                    velocity: [256, -512],
                    facing: 3 + side as u16,
                    crew: 8 + side as u16,
                    energy: 12 + side as u8,
                    input: 1 << side,
                    status: 17,
                })
            );
        }
    }

    #[test]
    fn input_scope_accepts_interplanetary_combat_but_not_hyperspace_flight() {
        for activity in [512, 513, 514, 516] {
            for phase in [BattlePhase::Input, BattlePhase::Advanced] {
                assert!(
                    is_combat_sample(activity, phase),
                    "activity={activity}, phase={phase:?}"
                );
            }
        }
        assert!(!is_combat_sample(0, BattlePhase::Input));
        assert!(!is_combat_sample(515, BattlePhase::Input));
        assert!(!is_combat_sample(515, BattlePhase::Advanced));
        assert!(is_combat_sample(0, BattlePhase::Advanced));
    }

    #[test]
    fn replacement_records_the_single_consumed_audio_result_and_short_circuit() {
        let mut observer = Recorded::default();
        let mut ditty = 0;
        let mut io = Io {
            observer: &mut observer,
        };
        assert!(!replacement_gate(
            &mut ditty,
            || panic!("short circuit"),
            &mut io
        ));
        ditty = 1;
        assert!(replacement_gate(&mut ditty, || true, &mut io));
        assert!(!replacement_gate(&mut ditty, || false, &mut io));
        assert_eq!(ditty, 0);
        assert_eq!(
            observer.0,
            [
                ReplacementGate {
                    ditty_before: 0,
                    track_playing: None,
                    ditty_after: 0,
                    blocked: false
                },
                ReplacementGate {
                    ditty_before: 1,
                    track_playing: Some(true),
                    ditty_after: 1,
                    blocked: true
                },
                ReplacementGate {
                    ditty_before: 1,
                    track_playing: Some(false),
                    ditty_after: 0,
                    blocked: false
                },
            ]
        );
    }
}
