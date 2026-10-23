//! Deterministic automation scene activation and dispatch observation.
//!
//! A start scene is declarative script data. It is consumed once, after game
//! structures and initial events are initialized but before the first activity
//! dispatch. Runtime setup then drives the normal game-loop encounter path;
//! it does not invoke communication recursively from an input callback.

use serde::Deserialize;
use std::fmt;
use std::sync::OnceLock;

/// Closed registry of deterministic automation scenes.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AutomationScene {
    /// Start the opening Sol encounter with the Ur-Quan probe.
    SolProbeEncounter,
    /// Start the first real Starbase commander communication.
    StarbaseCommander,
}

impl AutomationScene {
    /// Stable script-facing name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::SolProbeEncounter => "sol_probe_encounter",
            Self::StarbaseCommander => "starbase_commander",
        }
    }
}

/// Lifecycle boundaries at which scene activation might be attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneActivationBoundary {
    MainMenu,
    InputCallback,
    GameInitialized,
}

/// Where Sol sits in the universe map (`SOL_X`, `SOL_Y` in `hyper.h`), which is
/// where a new game starts.
pub const SOL_UNIVERSE_LOCATION: (i16, i16) = (1752, 1450);

/// Pure setup plan for a deterministic scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScenePlan {
    pub scene: AutomationScene,
    pub encounter_ship: u8,
    pub expected_encounter_conversation: u32,
    pub expected_dialogue_conversation: u32,
    pub current_activity: u16,
    /// Star the flagship is in when the scene starts. The encounter screen
    /// names this star, so a scene entered before the first solar-system visit
    /// must establish it as the real game does.
    pub star_location: (i16, i16),
    /// Named game-state values the scene's circumstances require.
    pub game_state: &'static [(&'static str, u32)],
}

/// Scene activation or verification failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneError {
    UnsafeBoundary {
        scene: AutomationScene,
        boundary: SceneActivationBoundary,
    },
    SetupFailed(&'static str),
    NoActiveScene,
    WrongActiveScene {
        expected: AutomationScene,
        actual: AutomationScene,
    },
    EncounterNotDispatched,
    WrongEncounterConversation {
        expected: u32,
        actual: u32,
    },
    DialogueNotDispatched,
    WrongDialogueConversation {
        expected: u32,
        actual: u32,
    },
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsafeBoundary { scene, boundary } => write!(
                f,
                "scene '{}' cannot activate at {boundary:?}; game initialization must be complete",
                scene.name()
            ),
            Self::SetupFailed(reason) => write!(f, "scene setup failed: {reason}"),
            Self::NoActiveScene => write!(f, "no automation scene is active"),
            Self::WrongActiveScene { expected, actual } => write!(
                f,
                "expected active scene '{}', found '{}'",
                expected.name(),
                actual.name()
            ),
            Self::EncounterNotDispatched => write!(f, "encounter dispatch has not been observed"),
            Self::WrongEncounterConversation { expected, actual } => write!(
                f,
                "expected encounter conversation {expected}, observed {actual}"
            ),
            Self::DialogueNotDispatched => write!(f, "dialogue dispatch has not been observed"),
            Self::WrongDialogueConversation { expected, actual } => write!(
                f,
                "expected dialogue conversation {expected}, observed {actual}"
            ),
        }
    }
}

impl std::error::Error for SceneError {}

/// Return the pure setup/verification contract for a scene.
pub fn scene_plan(
    scene: AutomationScene,
    boundary: SceneActivationBoundary,
) -> Result<ScenePlan, SceneError> {
    if boundary != SceneActivationBoundary::GameInitialized {
        return Err(SceneError::UnsafeBoundary { scene, boundary });
    }

    match scene {
        AutomationScene::SolProbeEncounter => Ok(ScenePlan {
            scene,
            encounter_ship: crate::comm::dispatch::ship::URQUAN_DRONE_SHIP,
            expected_encounter_conversation: crate::comm::dispatch::conv::URQUAN_DRONE,
            expected_dialogue_conversation: crate::comm::dispatch::conv::URQUAN,
            current_activity: crate::comm::dispatch::IN_ENCOUNTER
                | crate::comm::dispatch::START_ENCOUNTER,
            star_location: SOL_UNIVERSE_LOCATION,
            game_state: &[("PROBE_MESSAGE_DELIVERED", 0)],
        }),
        AutomationScene::StarbaseCommander => Ok(ScenePlan {
            scene,
            encounter_ship: 0,
            expected_encounter_conversation: crate::comm::dispatch::conv::COMMANDER,
            expected_dialogue_conversation: crate::comm::dispatch::conv::COMMANDER,
            current_activity: crate::comm::dispatch::IN_INTERPLANETARY
                | crate::comm::dispatch::START_ENCOUNTER,
            star_location: SOL_UNIVERSE_LOCATION,
            game_state: &[
                ("GLOBAL_FLAGS_AND_DATA", 0xFF),
                ("STARBASE_AVAILABLE", 1),
                ("PROBE_ILWRATH_ENCOUNTER", 0),
                // The visit goes straight into the Commander conversation only
                // when the moon base is on the ship (or Chmmr installed the
                // bomb); otherwise it falls through to the starbase menu.
                ("MOONBASE_ON_SHIP", 1),
            ],
        }),
    }
}

/// One-shot holder for a declarative start scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingStartScene {
    scene: Option<AutomationScene>,
    consumed: bool,
}

impl PendingStartScene {
    #[must_use]
    pub const fn new(scene: Option<AutomationScene>) -> Self {
        Self {
            scene,
            consumed: false,
        }
    }

    /// Consume the scene once at the safe game-initialized boundary.
    pub fn take(
        &mut self,
        boundary: SceneActivationBoundary,
    ) -> Result<Option<AutomationScene>, SceneError> {
        let Some(scene) = self.scene else {
            return Ok(None);
        };
        if self.consumed {
            return Ok(None);
        }
        scene_plan(scene, boundary)?;
        self.consumed = true;
        Ok(Some(scene))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SceneObservation {
    active: Option<AutomationScene>,
    encounter_conversation: Option<u32>,
    dialogue_conversation: Option<u32>,
    dispatch_generation: u64,
}

impl SceneObservation {
    fn begin(&mut self, scene: AutomationScene) {
        *self = Self {
            active: Some(scene),
            encounter_conversation: None,
            dialogue_conversation: None,
            dispatch_generation: 0,
        };
    }

    fn observe_encounter(&mut self, conversation: u32) {
        self.encounter_conversation = Some(conversation);
        self.dialogue_conversation = None;
    }

    fn observe_dialogue(&mut self, conversation: u32) {
        self.dialogue_conversation = Some(conversation);
        if self.encounter_conversation.is_some() {
            self.dispatch_generation = self.dispatch_generation.saturating_add(1);
        }
    }

    fn verify_dispatch(&self, encounter: u32, dialogue: u32) -> Result<(), SceneError> {
        let actual_encounter = self
            .encounter_conversation
            .ok_or(SceneError::EncounterNotDispatched)?;
        if actual_encounter != encounter {
            return Err(SceneError::WrongEncounterConversation {
                expected: encounter,
                actual: actual_encounter,
            });
        }
        let actual_dialogue = self
            .dialogue_conversation
            .ok_or(SceneError::DialogueNotDispatched)?;
        if actual_dialogue != dialogue {
            return Err(SceneError::WrongDialogueConversation {
                expected: dialogue,
                actual: actual_dialogue,
            });
        }
        Ok(())
    }

    fn verify(&self, scene: AutomationScene) -> Result<ScenePlan, SceneError> {
        let plan = scene_plan(scene, SceneActivationBoundary::GameInitialized)?;
        let actual_scene = self.active.ok_or(SceneError::NoActiveScene)?;
        if actual_scene != scene {
            return Err(SceneError::WrongActiveScene {
                expected: scene,
                actual: actual_scene,
            });
        }
        let encounter = self
            .encounter_conversation
            .ok_or(SceneError::EncounterNotDispatched)?;
        if encounter != plan.expected_encounter_conversation {
            return Err(SceneError::WrongEncounterConversation {
                expected: plan.expected_encounter_conversation,
                actual: encounter,
            });
        }
        let dialogue = self
            .dialogue_conversation
            .ok_or(SceneError::DialogueNotDispatched)?;
        if dialogue != plan.expected_dialogue_conversation {
            return Err(SceneError::WrongDialogueConversation {
                expected: plan.expected_dialogue_conversation,
                actual: dialogue,
            });
        }
        Ok(plan)
    }
}

fn observation() -> &'static parking_lot::Mutex<SceneObservation> {
    static OBSERVATION: OnceLock<parking_lot::Mutex<SceneObservation>> = OnceLock::new();
    OBSERVATION.get_or_init(|| parking_lot::Mutex::new(SceneObservation::default()))
}

/// Effects a scene has when it takes over from the new game's first
/// solar-system entry.
pub trait SceneEntryOps {
    fn set_last_activity(&mut self, activity: u16);
    fn set_current_activity(&mut self, activity: u16);
    /// Clear the screen context to black.
    fn clear_screen(&mut self);
    /// Queue the fade from the faded-out colormap back to full color.
    fn fade_screen_to_color(&mut self);
}

/// Switch the main loop to the scene's activity.
///
/// A new game leaves the screen faded to black, and the first solar-system
/// entry fades it back in (`solarsys.c`, "Starting a new game"). The scene
/// replaces that entry, so it performs the same clear and fade-in; without
/// it every frame of the scene is composited through a black colormap. The C
/// does not wait on the fade, so neither does the scene.
pub fn enter_scene(plan: &ScenePlan, ops: &mut impl SceneEntryOps) {
    ops.set_last_activity(crate::comm::dispatch::IN_ENCOUNTER);
    ops.set_current_activity(plan.current_activity);
    ops.clear_screen();
    ops.fade_screen_to_color();
}

#[cfg(not(test))]
struct LinkedSceneEntry;

#[cfg(not(test))]
impl SceneEntryOps for LinkedSceneEntry {
    fn set_last_activity(&mut self, activity: u16) {
        crate::mainloop::ffi::set_last_activity(crate::mainloop::types::ActivityValue(activity));
    }

    fn set_current_activity(&mut self, activity: u16) {
        // SAFETY: writes the current-activity global on the main thread.
        unsafe { crate::mainloop::c_extern::set_current_activity(activity) };
    }

    fn clear_screen(&mut self) {
        use crate::mainloop::restart_menu::c_extern;
        // SAFETY: the screen context exists once game initialization is
        // complete, and scene activation runs on the main thread.
        unsafe {
            c_extern::SetContext(c_extern::ScreenContext);
            c_extern::ClearDrawable();
        }
    }

    fn fade_screen_to_color(&mut self) {
        use crate::mainloop::restart_menu::c_extern;
        // SAFETY: FadeScreen is called from the main thread; the returned
        // completion time is deliberately not awaited, as in the C entry.
        unsafe {
            c_extern::FadeScreen(
                c_extern::FADE_ALL_TO_COLOR,
                (c_extern::ONE_SECOND / 2) as i16,
            );
        }
    }
}

/// Activate a scene at the lifecycle-owned safe boundary.
///
/// This prepares the real NPC queue and sets `START_ENCOUNTER`; the normal
/// main-loop state machine subsequently calls `rust_race_communication`.
pub fn activate(
    scene: AutomationScene,
    boundary: SceneActivationBoundary,
) -> Result<ScenePlan, SceneError> {
    let plan = scene_plan(scene, boundary)?;
    observation().lock().begin(scene);

    #[cfg(not(test))]
    unsafe {
        if scene == AutomationScene::SolProbeEncounter {
            if let Err(reason) = crate::comm::dispatch::prepare_automation_sol_probe_encounter() {
                *observation().lock() = SceneObservation::default();
                return Err(SceneError::SetupFailed(reason));
            }
        }
        if let Err(reason) = crate::comm::dispatch::place_flagship_at_star(plan.star_location) {
            *observation().lock() = SceneObservation::default();
            return Err(SceneError::SetupFailed(reason));
        }
        for (name, value) in plan.game_state {
            crate::state::game_state_keys::set_game_state(name, *value);
        }
        enter_scene(&plan, &mut LinkedSceneEntry);
    }

    #[cfg(test)]
    {
        let _ = plan;
        *observation().lock() = SceneObservation::default();
        return Err(SceneError::SetupFailed(
            "runtime scene activation is unavailable in isolated unit tests",
        ));
    }

    #[allow(unreachable_code)]
    Ok(plan)
}

/// Record the normal RaceCommunication mapping selected from the NPC queue.
pub fn observe_encounter_dispatch(conversation: u32) {
    observation().lock().observe_encounter(conversation);
}

/// Record the race dialogue selected by `rust_init_race_dispatch`.
pub fn observe_dialogue_dispatch(conversation: u32) {
    observation().lock().observe_dialogue(conversation);
}

/// Return the scene currently active in this process, if any.
#[must_use]
pub fn active_scene() -> Option<AutomationScene> {
    observation().lock().active
}

/// Verify that a scene reached its expected encounter and dialogue dispatches.
pub fn verify(scene: AutomationScene) -> Result<ScenePlan, SceneError> {
    observation().lock().verify(scene)
}

/// Verify the most recently observed normal encounter/dialogue dispatch IDs.
pub fn verify_dispatch(encounter: u32, dialogue: u32) -> Result<(), SceneError> {
    observation().lock().verify_dispatch(encounter, dialogue)
}

/// Return the latest complete dispatch generation and IDs.
#[must_use]
pub fn dispatch_observation() -> (u64, Option<u32>, Option<u32>) {
    let observed = observation().lock();
    (
        observed.dispatch_generation,
        observed.encounter_conversation,
        observed.dialogue_conversation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_scene_is_consumed_once() {
        let mut pending = PendingStartScene::new(Some(AutomationScene::SolProbeEncounter));
        assert_eq!(
            pending
                .take(SceneActivationBoundary::GameInitialized)
                .unwrap(),
            Some(AutomationScene::SolProbeEncounter)
        );
        assert_eq!(
            pending
                .take(SceneActivationBoundary::GameInitialized)
                .unwrap(),
            None
        );
    }

    #[test]
    fn unsafe_boundary_does_not_consume_start_scene() {
        let mut pending = PendingStartScene::new(Some(AutomationScene::SolProbeEncounter));
        assert!(matches!(
            pending.take(SceneActivationBoundary::InputCallback),
            Err(SceneError::UnsafeBoundary { .. })
        ));
        assert_eq!(
            pending
                .take(SceneActivationBoundary::GameInitialized)
                .unwrap(),
            Some(AutomationScene::SolProbeEncounter)
        );
    }

    #[test]
    fn sol_probe_plan_uses_real_probe_and_dispatch_ids() {
        let plan = scene_plan(
            AutomationScene::SolProbeEncounter,
            SceneActivationBoundary::GameInitialized,
        )
        .unwrap();
        assert_eq!(plan.encounter_ship, 23);
        assert_eq!(plan.expected_encounter_conversation, 24);
        assert_eq!(plan.expected_dialogue_conversation, 18);
        assert_eq!(plan.current_activity, 0x0402);
    }

    #[test]
    fn scenes_start_in_sol_where_a_new_game_begins() {
        for scene in [
            AutomationScene::SolProbeEncounter,
            AutomationScene::StarbaseCommander,
        ] {
            let plan = scene_plan(scene, SceneActivationBoundary::GameInitialized).unwrap();
            assert_eq!(plan.star_location, (1752, 1450), "{}", scene.name());
        }
    }

    #[test]
    fn the_starbase_scene_reaches_the_commander_without_the_starbase_menu() {
        let plan = scene_plan(
            AutomationScene::StarbaseCommander,
            SceneActivationBoundary::GameInitialized,
        )
        .unwrap();
        let state = |name: &str| {
            plan.game_state
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| *value)
        };
        assert_eq!(state("STARBASE_AVAILABLE"), Some(1));
        assert_eq!(state("MOONBASE_ON_SHIP"), Some(1));
        assert_eq!(state("PROBE_ILWRATH_ENCOUNTER"), Some(0));
    }

    #[test]
    fn the_probe_scene_does_not_claim_to_choose_the_segue() {
        // Ur-Quan initialization sets a hostile segue, so the player always
        // answers the encounter screen; the scene must not pretend otherwise.
        let plan = scene_plan(
            AutomationScene::SolProbeEncounter,
            SceneActivationBoundary::GameInitialized,
        )
        .unwrap();
        assert!(plan
            .game_state
            .iter()
            .all(|(key, _)| *key != "BATTLE_SEGUE"));
    }

    #[derive(Default)]
    struct RecordingOps {
        calls: Vec<String>,
    }

    impl SceneEntryOps for RecordingOps {
        fn set_last_activity(&mut self, activity: u16) {
            self.calls.push(format!("last_activity:{activity:#x}"));
        }
        fn set_current_activity(&mut self, activity: u16) {
            self.calls.push(format!("current_activity:{activity:#x}"));
        }
        fn clear_screen(&mut self) {
            self.calls.push("clear_screen".into());
        }
        fn fade_screen_to_color(&mut self) {
            self.calls.push("fade_to_color".into());
        }
    }

    #[test]
    fn entering_a_scene_clears_then_fades_the_screen_back_to_color() {
        for scene in [
            AutomationScene::SolProbeEncounter,
            AutomationScene::StarbaseCommander,
        ] {
            let plan = scene_plan(scene, SceneActivationBoundary::GameInitialized).unwrap();
            let mut ops = RecordingOps::default();
            enter_scene(&plan, &mut ops);
            assert_eq!(
                ops.calls,
                [
                    format!("last_activity:{:#x}", crate::comm::dispatch::IN_ENCOUNTER),
                    format!("current_activity:{:#x}", plan.current_activity),
                    "clear_screen".to_string(),
                    "fade_to_color".to_string(),
                ],
                "{}",
                scene.name()
            );
        }
    }

    #[test]
    fn scene_verification_rejects_arilou_dialogue() {
        let mut observed = SceneObservation::default();
        observed.begin(AutomationScene::SolProbeEncounter);
        observed.observe_encounter(24);
        observed.observe_dialogue(0);
        assert!(matches!(
            observed.verify(AutomationScene::SolProbeEncounter),
            Err(SceneError::WrongDialogueConversation {
                expected: 18,
                actual: 0
            })
        ));
    }

    #[test]
    fn scene_verification_accepts_expected_dispatch_chain() {
        let mut observed = SceneObservation::default();
        observed.begin(AutomationScene::SolProbeEncounter);
        observed.observe_encounter(24);
        observed.observe_dialogue(18);
        assert!(observed.verify(AutomationScene::SolProbeEncounter).is_ok());
    }
}
