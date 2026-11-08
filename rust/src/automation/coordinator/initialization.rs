//! Initial scheduler state for an app-owned automation observer.

use super::*;

impl CoordInner {
    pub(super) fn new(
        now: Instant,
        pending_start_scene: PendingStartScene,
        consumed_communication_completions: u64,
    ) -> Self {
        Self {
            sched_state: SchedulerState::initial(),
            checkpoint_step: 0,
            native_publication_pending: false,
            #[cfg(feature = "debug-process")]
            native_playable_published: false,
            ready: false,
            last_progress: now,
            input_seen: 0,
            present_seen: 0,
            last_observed: now,
            accepted_player_inputs: 0,
            pending_player_input: None,
            held_repeat_key: None,
            verified_battle_frames: 0,
            native_battle_previous_ms: [None; 3],
            flight_position: FlightPositionTracker::default(),
            battle_loss: BattleLossTracker::default(),
            flagship_turn: FlagshipTurnTracker::default(),
            planet_minerals: PlanetMineralsTracker::default(),
            battle_fight: BattleFightTracker::default(),
            fleet_fight: crate::automation::fleet_fight::FleetFight::default(),
            fleet_retreat: crate::automation::fleet_fight::FleetRetreat::default(),
            samatra: crate::automation::samatra::Controller::default(),
            star_flight: crate::automation::star_flight::StarFlight::default(),
            star_encounter_active: false,
            patrol_response_pending: false,
            device: Default::default(),
            energy: Default::default(),
            finalized: false,
            terminal_class: None,
            pending_transitions: Vec::new(),
            quit_selected: false,
            pending_main_menu_ready: false,
            armed_capture_label: None,
            pending_start_scene,
            consumed_communication_completions,
            consumed_dispatch_generation: 0,
            observed_planet_side_phase: None,
            awaited_planet_side_generation: None,
            consumed_response_generation: 0,
            consumed_replay_generation: 0,
            consumed_planet_menu_generation: 0,
            consumed_ship_picker_generation: 0,
            release_semantic_select: false,
            replay_final_activity: 0,
            parity_fixture: None,
            save_capture: crate::automation::save_capture::Request::from_environment(),
            orbit: Default::default(),
            departure: Default::default(),
        }
    }
}
