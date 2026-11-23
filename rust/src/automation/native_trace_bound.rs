//! Upper bound on the native trace a scenario can write, by record kind.
//!
//! A native run reserves one inventory member for its trace, so a scenario
//! whose trace cannot fit must be rejected before launch. The kinds that cost
//! bytes per input tick are `input_tick`, `presentation`, `native_graphics`
//! (written only when the sampled graphics state changes) and `native_battle`
//! (written only while a battle is sampled). Each kind is charged its widest
//! serialized record times the rate it can be written at. Battle samples are
//! charged only to scenarios whose script can reach combat.
//!
//! Rates are bounded by the code where it can be and by measurement where it
//! cannot. Measurements come from `campaign-s06c-utwig-bomb` (39,771 ticks,
//! the most combat of any scenario) and `campaign-s07c-war-cleared` (101,050
//! ticks, the longest scenario) in run 1011-s07c-s06c-03eb14915.

use super::script::{Action, ValidatedScript};

/// `input_tick` records are written once per admitted input tick.
const INPUT_TICK_RECORD_BYTES: u64 = 136;

/// Measured 1.998 presentations per input tick, so two is the bound.
const PRESENTATIONS_PER_TICK: u64 = 2;
const PRESENTATION_RECORD_BYTES: u64 = 208;

/// A graphics record lists the display events and readbacks of one frame, so
/// its width varies; the widest of 2,491 measured records was 838 bytes.
const GRAPHICS_RECORD_BYTES: u64 = 1024;
/// One graphics record per this many ticks at most. Measured 1 per 15.7 in
/// s06c and 1 per 665 in s07c.
const GRAPHICS_CHANGE_INTERVAL_TICKS: u64 = 12;

/// Each combat tick writes an input sample and an advance sample, and a
/// replacement gate adds a third. 5,656 combat ticks in s06c wrote 5,601 with
/// two records, 34 with one and 21 with three.
const BATTLE_RECORDS_PER_COMBAT_TICK: u64 = 3;
/// The widest of 11,299 measured battle records was 784 bytes; the bound is
/// the width of a sample with both ships, a gate and eight-digit counters.
const BATTLE_RECORD_BYTES: u64 = 928;
/// Combat ticks are at most this share of a scenario that can reach combat
/// (measured 5,656 of 39,771, or 14%, in s06c).
const COMBAT_TICK_SHARE_DENOMINATOR: u64 = 4;

/// What a scenario's script can make the native trace contain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeTraceProfile {
    may_enter_combat: bool,
}

impl NativeTraceProfile {
    /// A scenario that never reaches combat writes no battle records.
    #[cfg(test)]
    pub const WITHOUT_COMBAT: Self = Self {
        may_enter_combat: false,
    };
    #[cfg(test)]
    pub const WITH_COMBAT: Self = Self {
        may_enter_combat: true,
    };

    #[must_use]
    pub fn of_script(script: &ValidatedScript) -> Self {
        Self {
            may_enter_combat: script.steps().iter().any(can_enter_combat),
        }
    }

    /// The most bytes the trace of `input_ticks` ticks can occupy, or `None`
    /// when that overflows.
    #[must_use]
    pub fn max_trace_bytes(self, input_ticks: u64) -> Option<u64> {
        let per_tick = INPUT_TICK_RECORD_BYTES + PRESENTATIONS_PER_TICK * PRESENTATION_RECORD_BYTES;
        let graphics_records = input_ticks.div_ceil(GRAPHICS_CHANGE_INTERVAL_TICKS);
        let battle_bytes = if self.may_enter_combat {
            let combat_ticks = input_ticks.div_ceil(COMBAT_TICK_SHARE_DENOMINATOR);
            combat_ticks
                .checked_mul(BATTLE_RECORDS_PER_COMBAT_TICK)?
                .checked_mul(BATTLE_RECORD_BYTES)?
        } else {
            0
        };
        input_ticks
            .checked_mul(per_tick)?
            .checked_add(graphics_records.checked_mul(GRAPHICS_RECORD_BYTES)?)?
            .checked_add(battle_bytes)
    }
}

/// Whether an action can run a flagship through space or a battle, where
/// combat starts. Every action is listed so that a new one has to be
/// classified.
fn can_enter_combat(action: &Action) -> bool {
    match action {
        Action::SetPlayerKey(_)
        | Action::TapPlayerKey(_)
        | Action::NavigateToPlanet(_)
        | Action::NavigateToContact(_)
        | Action::NavigateToMoon(_)
        | Action::NavigateToOrbit(_)
        | Action::NavigateToHyperspace(_)
        | Action::FlyToStar(_)
        | Action::TurnFlagship(_)
        | Action::FightBattle(_)
        | Action::FightEncounter(_)
        | Action::RetreatEncounter(_)
        | Action::FightSamatra(_)
        | Action::PickBattleShip(_)
        | Action::AssertBattleFrames(_)
        | Action::WaitForBattleFrames(_)
        | Action::WaitForBattleShipLoss(_)
        | Action::WaitForPkunkRebirth(_)
        | Action::SetupMeleeMutualLossFixture(_)
        | Action::ConnectNetplay(_)
        | Action::ConfigureParityMelee(_)
        | Action::RunParityFixture(_)
        | Action::RunParityCorpus(_)
        | Action::AssertParityFixture => true,
        Action::WaitInputTicks(_)
        | Action::WaitPresentations(_)
        | Action::WaitForMainMenuReady(_)
        | Action::SetMenuKey(_)
        | Action::TapMenuKey(_)
        | Action::UseDevice(_)
        | Action::CollectEnergyNode(_)
        | Action::CollectPlanetMinerals(_)
        | Action::WaitForPlanetSideStart(_)
        | Action::WaitForPlanetSideEnd(_)
        | Action::SelectPlanetMenu(_)
        | Action::Capture(_)
        | Action::AssertActivity(_)
        | Action::AssertScene(_)
        | Action::AssertMode(_)
        | Action::AssertDispatch(_)
        | Action::WaitForDispatch(_)
        | Action::AssertGameOptions(_)
        | Action::AssertCommunicationResponses(_)
        | Action::RecordHyperspacePosition(_)
        | Action::AssertHyperspaceDisplacement(_)
        | Action::AssertHyperspaceStationary(_)
        | Action::RecordSavedHyperspacePosition(_)
        | Action::AssertHyperspacePositionRestored(_)
        | Action::AssertShipResources(_)
        | Action::AssertState(_)
        | Action::WaitForState(_)
        | Action::AssertShipElement(_)
        | Action::AssertPlanetSideCollisions(_)
        | Action::SelectCommunicationResponse(_)
        | Action::SelectResponse(_)
        | Action::WaitForCommunicationResponses(_)
        | Action::WaitForCommunicationEnd(_)
        | Action::WaitForCommunicationReplay(_)
        | Action::AssertMainMenuTransition(_)
        | Action::SetupPlanetSideCollisionFixture(_)
        | Action::SetupPlanetSideMineralGallery(_)
        | Action::ExpectQuit(_)
        | Action::Finish => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::capture::capture_trace_record;
    use crate::automation::scheduler::CaptureGeneration;
    use crate::automation::trace::{
        NativeBattleEvidence, PresentationEvidence, RecordKind, TraceRecord,
    };
    use crate::automation::{parse_script, validate_script};
    use uqm_engine::observe::battle::{BattlePhase, BattleShip, BattleTick, ReplacementGate};

    const MEMBER_BYTES: u64 = 64 * 1024 * 1024;
    const S06C_INPUT_TICKS: u64 = 39_771;
    const S07C_INPUT_TICKS: u64 = 101_050;
    const WIDE: u64 = 99_999_999;

    fn serialized_len(kind: RecordKind, edit: impl FnOnce(&mut TraceRecord)) -> u64 {
        let mut record = capture_trace_record(WIDE, WIDE, CaptureGeneration(0), "");
        record.run = 1;
        record.input_seen = WIDE;
        record.present_seen = WIDE;
        record.label = None;
        record.kind = kind;
        edit(&mut record);
        record.to_jsonl().unwrap().len() as u64
    }

    fn wide_ship() -> BattleShip {
        BattleShip {
            identity: [9, 99],
            species: 17,
            position: [i16::MIN, i16::MIN],
            velocity: [-99_999, -99_999],
            facing: 15,
            crew: 999,
            energy: u8::MAX,
            input: u8::MAX,
            status: u16::MAX,
        }
    }

    fn profile_of(action: &str) -> NativeTraceProfile {
        let source = format!(
            r#"{{"version":1,"name":"probe","steps":[{action},{{"action":"finish"}}],"budgets":{{"max_input_ticks":1000,"max_presentations":100,"max_wallclock_seconds":60}}}}"#
        );
        let document = parse_script(source.as_bytes(), "probe.json").unwrap();
        NativeTraceProfile::of_script(&validate_script(document, "probe.json").unwrap())
    }

    #[test]
    fn the_record_sizes_cover_the_widest_records_of_their_kind() {
        let presentation = serialized_len(RecordKind::Presentation, |record| {
            record.presentation = Some(PresentationEvidence {
                count: WIDE,
                generation: 99,
                width: 1280,
                height: 960,
            });
        });
        assert!(presentation <= PRESENTATION_RECORD_BYTES, "{presentation}");
        let input_tick = serialized_len(RecordKind::InputTick, |_| {});
        assert!(input_tick <= INPUT_TICK_RECORD_BYTES, "{input_tick}");
        // Both ships present, with a replacement gate and the widest values a
        // battle reaches: positions at the type minimum, 5-digit velocities and
        // every key and menu entry set.
        let battle = serialized_len(RecordKind::NativeBattle, |record| {
            record.schema = TraceRecord::NATIVE_SCHEMA;
            record.native_battle = Some(NativeBattleEvidence {
                frame: WIDE,
                tick: Some(BattleTick {
                    phase: BattlePhase::Advanced,
                    activity: 514,
                    keys: [[1; 7]; 2],
                    pulsed: [[1; 7]; 2],
                    menu: [1; 24],
                    menu_pulsed: [1; 24],
                    rng: u32::MAX,
                    ditty: 1,
                    ships: [Some(wide_ship()), Some(wide_ship())],
                    counters: [u8::MAX; 2],
                }),
                gate: Some(ReplacementGate {
                    ditty_before: 1,
                    track_playing: Some(true),
                    ditty_after: 1,
                    blocked: true,
                }),
                picker_active: true,
                picker_generation: 99,
                delta_ms: Some(999_999),
            });
        });
        assert!(battle <= BATTLE_RECORD_BYTES, "{battle}");
    }

    #[test]
    fn the_measured_scenarios_fit_one_member() {
        let combat = NativeTraceProfile::WITH_COMBAT.max_trace_bytes(S06C_INPUT_TICKS);
        assert!(combat.unwrap() < MEMBER_BYTES);
        // s06c wrote 29.6 MB and s07c 51.4 MB, so each bound covers its run.
        assert!(combat.unwrap() > 29_561_415);
        let long = NativeTraceProfile::WITHOUT_COMBAT.max_trace_bytes(S07C_INPUT_TICKS);
        assert!(long.unwrap() < MEMBER_BYTES);
        assert!(long.unwrap() > 51_355_354);
    }

    #[test]
    fn a_battle_heavy_profile_over_the_member_is_charged_for_its_battle_records() {
        // At 89,000 ticks the old 600-byte estimate admitted a combat scenario
        // at the s06c rate of 755 bytes per tick, which needs 67.2 MB.
        let ticks = 89_000;
        assert!(ticks * 600 < MEMBER_BYTES);
        assert!(ticks * 755 > MEMBER_BYTES);
        let with_combat = NativeTraceProfile::WITH_COMBAT
            .max_trace_bytes(ticks)
            .unwrap();
        assert!(with_combat > MEMBER_BYTES, "{with_combat}");
        // The same ticks with no way to reach combat still fit.
        let without_combat = NativeTraceProfile::WITHOUT_COMBAT
            .max_trace_bytes(ticks)
            .unwrap();
        assert!(without_combat < MEMBER_BYTES, "{without_combat}");
    }

    #[test]
    fn the_bound_reports_overflow() {
        assert_eq!(
            NativeTraceProfile::WITH_COMBAT.max_trace_bytes(u64::MAX),
            None
        );
    }

    #[test]
    fn only_scripts_that_can_reach_combat_are_charged_for_battle_records() {
        assert_eq!(
            profile_of(r#"{"action":"fight_encounter","max_ticks":100}"#),
            NativeTraceProfile::WITH_COMBAT
        );
        assert_eq!(
            profile_of(r#"{"action":"wait_input_ticks","count":5}"#),
            NativeTraceProfile::WITHOUT_COMBAT
        );
    }
}
