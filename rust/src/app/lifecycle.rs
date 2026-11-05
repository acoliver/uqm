use crate::engine::{Kernel, StoryKey, World};
use crate::mainloop::state_machine::{self, ActivityDecision, BreakAction, GameStateInfo};
use crate::mainloop::types::ActivityValue;

fn state(world: &World) -> GameStateInfo {
    GameStateInfo {
        chmmr_bomb_state: world.story_state(StoryKey::CHMMR_BOMB_STATE) as u8,
        starbase_available: world.story_state(StoryKey::STARBASE_AVAILABLE) as u8,
        global_flags_and_data: world.story_state(StoryKey::GLOBAL_FLAGS_AND_DATA) as u8,
        kohr_ah_killed_all: world.story_state(StoryKey::KOHR_AH_KILLED_ALL) as u8,
        crew_enlisted: world.data.flagship.crew_enlisted,
    }
}

pub(super) fn prepare_dispatch(world: &mut World) -> ActivityDecision {
    let current = ActivityValue::new(world.activity());
    let next = ActivityValue::new(world.next_activity);
    if state_machine::should_zero_velocity(current, next) {
        world.data.game.velocity.zero();
    } else {
        world.set_activity(state_machine::resolve_load_activity(current, next).0);
    }
    let activity = ActivityValue::new(world.activity());
    let decision = state_machine::evaluate(activity, &state(world));
    world.set_activity(state_machine::pre_dispatch_mutate(activity, decision).0);
    decision
}

pub(super) fn complete_encounter(world: &mut World, decision: ActivityDecision) {
    if matches!(
        decision,
        ActivityDecision::InstallBombAtEarth
            | ActivityDecision::VisitStarBase
            | ActivityDecision::RaceCommunication
    ) {
        world.set_activity(
            state_machine::post_encounter_clear(ActivityValue::new(world.activity())).0,
        );
    }
}

pub(super) fn finish_dispatch(world: &World, kernel: &mut Kernel) -> BreakAction {
    kernel.last_activity = world.activity();
    state_machine::check_break(ActivityValue::new(world.activity()), &state(world))
}

#[cfg(test)]
mod tests {
    use super::{complete_encounter, finish_dispatch, prepare_dispatch};
    use crate::engine::{world::story::GameState, Kernel, StoryKey, World};
    use crate::mainloop::state_machine::{ActivityDecision, BreakAction};
    use crate::mainloop::types::activity_flags::*;

    fn world(activity: u16) -> World {
        let mut world = World::new();
        world.story = Some(GameState::new());
        world.set_activity(activity);
        world
    }

    #[test]
    fn starbase_load_returns_to_solar_exploration_on_the_same_world() {
        let mut world = world(START_ENCOUNTER);
        let mut kernel = Kernel::new();
        world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, u8::MAX.into());
        let decision = prepare_dispatch(&mut world);
        assert_eq!(decision, ActivityDecision::VisitStarBase);
        world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
        world.set_activity(CHECK_LOAD);
        world.next_activity = 4 | START_INTERPLANETARY;
        complete_encounter(&mut world, decision);
        assert_eq!(finish_dispatch(&world, &mut kernel), BreakAction::Continue);
        assert_eq!(kernel.last_activity, CHECK_LOAD);
        assert_eq!(
            prepare_dispatch(&mut world),
            ActivityDecision::ExploreSolarSystem
        );
        assert_eq!(world.activity(), 4);
    }

    #[test]
    fn restart_break_records_activity_before_the_caller_clears_restart() {
        let mut world = world(3 | CHECK_RESTART);
        let mut kernel = Kernel::new();
        world.data.flagship.crew_enlisted = u16::MAX;
        assert_eq!(
            finish_dispatch(&world, &mut kernel),
            BreakAction::ClearRestart
        );
        assert_eq!(world.activity(), 3 | CHECK_RESTART);
        world.set_activity(world.activity() & !CHECK_RESTART);
        assert_eq!(kernel.last_activity, 3 | CHECK_RESTART);
    }

    #[test]
    fn pending_activity_belongs_to_its_world_and_survives_the_value_reset() {
        let mut first = world(0);
        let second = world(0);
        first.next_activity = CHECK_LOAD | START_INTERPLANETARY;
        first.reset_game_data();
        assert_eq!(first.next_activity, CHECK_LOAD | START_INTERPLANETARY);
        assert_eq!(second.next_activity, 0);
    }

    #[test]
    fn ordinary_dispatch_zeroes_the_supplied_world_velocity() {
        let mut world = world(START_INTERPLANETARY);
        world.data.game.velocity.set_components(72, -31);
        assert_eq!(
            prepare_dispatch(&mut world),
            ActivityDecision::ExploreSolarSystem
        );
        assert!(world.data.game.velocity.is_zero());
        assert_eq!(world.activity(), 4);
    }

    #[test]
    fn load_preserves_velocity_and_dispatches_pending_activity() {
        let mut world = world(CHECK_LOAD);
        world.next_activity = START_INTERPLANETARY;
        world.data.game.velocity.set_components(72, -31);
        let saved_velocity = world.data.game.velocity;
        assert_eq!(
            prepare_dispatch(&mut world),
            ActivityDecision::ExploreSolarSystem
        );
        assert_eq!(world.data.game.velocity, saved_velocity);
        assert_eq!(world.activity(), 4);
    }

    #[test]
    fn pending_load_alone_preserves_velocity_without_replacing_current_activity() {
        let mut world = world(START_ENCOUNTER);
        world.next_activity = CHECK_LOAD | START_INTERPLANETARY;
        world.data.game.velocity.set_components(72, -31);
        let saved_velocity = world.data.game.velocity;
        assert_eq!(
            prepare_dispatch(&mut world),
            ActivityDecision::RaceCommunication
        );
        assert_eq!(world.data.game.velocity, saved_velocity);
        assert_eq!(world.activity(), START_ENCOUNTER);
    }

    #[test]
    fn dispatch_reads_story_state_from_the_supplied_world() {
        let mut first = world(0);
        let mut second = world(0);
        first.set_story_state(StoryKey::CHMMR_BOMB_STATE, 2);
        second.set_story_state(StoryKey::CHMMR_BOMB_STATE, 2);
        second.set_story_state(StoryKey::STARBASE_AVAILABLE, 1);
        assert_eq!(
            prepare_dispatch(&mut first),
            ActivityDecision::InstallBombAtEarth
        );
        assert_eq!(
            prepare_dispatch(&mut second),
            ActivityDecision::VisitStarBase
        );
        assert_eq!(first.activity(), 0);
        assert_eq!(second.activity(), START_ENCOUNTER);
    }

    #[test]
    fn encounter_completion_reads_the_returned_activity_and_preserves_stop_flags() {
        for flags in [0, CHECK_ABORT, CHECK_LOAD] {
            let mut world = world(4 | START_ENCOUNTER | flags);
            complete_encounter(&mut world, ActivityDecision::RaceCommunication);
            assert_eq!(
                world.activity(),
                if flags == 0 {
                    4 | START_INTERPLANETARY
                } else {
                    4 | START_ENCOUNTER | flags
                }
            );
        }
    }

    #[test]
    fn solar_and_battle_returns_do_not_clear_new_encounters() {
        for decision in [
            ActivityDecision::ExploreSolarSystem,
            ActivityDecision::Battle,
        ] {
            let mut world = world(4 | START_ENCOUNTER);
            complete_encounter(&mut world, decision);
            assert_eq!(world.activity(), 4 | START_ENCOUNTER);
        }
    }

    #[test]
    fn dispatch_finish_records_the_returned_activity_on_the_supplied_kernel() {
        let world = world(4 | CHECK_ABORT);
        let mut kernel = Kernel::new();
        kernel.last_activity = 3;
        assert_eq!(finish_dispatch(&world, &mut kernel), BreakAction::Continue);
        assert_eq!(kernel.last_activity, 4 | CHECK_ABORT);
    }

    #[test]
    fn break_check_reads_post_dispatch_crew_and_story_state() {
        let mut world = world(3);
        let mut kernel = Kernel::new();
        assert_eq!(finish_dispatch(&world, &mut kernel), BreakAction::Continue);
        world.data.flagship.crew_enlisted = u16::MAX;
        assert_eq!(finish_dispatch(&world, &mut kernel), BreakAction::JustBreak);
        world.set_story_state(StoryKey::KOHR_AH_KILLED_ALL, 1);
        assert_eq!(
            finish_dispatch(&world, &mut kernel),
            BreakAction::InitBlackUrqCommunication
        );
        world.set_activity(3 | CHECK_LOAD);
        assert_eq!(finish_dispatch(&world, &mut kernel), BreakAction::Continue);
        assert_eq!(kernel.last_activity, 3 | CHECK_LOAD);
    }
}
