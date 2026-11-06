//! Observation ports specialized to the owned campaign view.

use super::{Kernel, World};
pub use uqm_engine::observe::{
    CheckpointView, DeviceObservation, Injected, InputSetup, InputView, Observer, PeerSetup,
    Presented, Unobserved,
};
pub type Io<'a> = uqm_engine::observe::ObservationIo<'a, Option<CampaignView>>;

/// Values copied for one observer call; this view never outlives its input boundary.
#[derive(Clone)]
pub struct CampaignView {
    pub(crate) activity: u16,
    pub(crate) flagship: crate::engine::world::SisState,
    pub(crate) velocity: (i32, i32),
    pub(crate) ship_position: (i16, i16),
    pub(crate) ship_facing: i32,
    pub(crate) flight_facing: Option<u8>,
    pub(crate) autopilot: (i16, i16),
    pub(crate) combat_flags: u8,
    pub(crate) date: (u8, u8, u16),
    pub(crate) story: Option<crate::engine::world::story::GameState>,
    pub(crate) current_star: Option<crate::engine::StarId>,
    pub(crate) contact_race: Option<u8>,
    pub(crate) escorts: Vec<(u8, u16)>,
    pub(crate) groups: Vec<(u8, u8, (i16, i16))>,
}

impl CampaignView {
    pub(crate) fn capture(world: &World, kernel: &Kernel) -> Self {
        let game = &world.data.game;
        let mut view = Self::from_world(world);
        view.flight_facing = crate::engine::save::hyper_save::read_flagship_facing(
            &kernel.races[0],
            &kernel.elements,
        )
        .ok()
        .flatten()
        .and_then(|facing| u8::try_from(facing).ok());
        if !game.ship_stamp.frame.is_null() {
            // SAFETY: the supplied world retains this game-thread graphics frame.
            view.ship_facing = i32::from(unsafe {
                crate::graphics::pixmap_abi::GetFrameIndex(game.ship_stamp.frame.cast())
            });
        }
        let npcs = world.npc_built_ships_ref();
        view.contact_race = npcs.head().map(|id| npcs.get(id).race_id);
        let ships = world.built_ships_ref();
        let mut next = ships.head();
        while let Some(id) = next {
            let ship = ships.get(id);
            view.escorts.push((ship.race_id, ship.crew_level));
            next = ships.succ(id);
        }
        let groups = world.ip_groups_ref();
        let mut next = groups.head();
        while let Some(id) = next {
            let group = groups.get(id);
            if group.in_system != 0 {
                view.groups
                    .push((group.race_id, group.sys_loc, (group.loc.x, group.loc.y)));
            }
            next = groups.succ(id);
        }
        view
    }

    pub(crate) fn for_input(view: InputView, capture: impl FnOnce() -> Self) -> Option<Self> {
        matches!(
            view,
            InputView::BeforeUpdate
                | InputView::Nested
                | InputView::PendingSpeech
                | InputView::CommunicationCompleted
        )
        .then(capture)
    }

    pub fn from_world(world: &World) -> Self {
        let game = &world.data.game;
        Self {
            activity: world.activity(),
            flagship: world.data.flagship,
            velocity: game.velocity.get_current_components(),
            ship_position: (game.ship_stamp.origin.x, game.ship_stamp.origin.y),
            ship_facing: 0,
            flight_facing: None,
            autopilot: (game.autopilot.x, game.autopilot.y),
            combat_flags: game.glob_flags,
            date: (
                game.game_clock.day_index,
                game.game_clock.month_index,
                game.game_clock.year_index,
            ),
            story: world.story.clone(),
            current_star: world.current_star,
            contact_race: None,
            escorts: Vec::new(),
            groups: Vec::new(),
        }
    }

    pub(crate) fn story_value(&self, key: crate::engine::StoryKey) -> Option<u8> {
        self.story
            .as_ref()
            .map(|story| story.get_state(key.start(), key.end()))
    }

    pub(crate) fn named_state_value(&self, name: &str) -> Option<u8> {
        self.story_value(crate::engine::StoryKey::from_name(name)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_reads_flagship_facing_from_its_unpublished_kernel() {
        let world = World::new();
        let mut kernel = crate::engine::Kernel::new();
        kernel.reset_ship_capacity(0, 1);
        kernel.reset_element_capacity(1);
        let element = kernel.elements.alloc().unwrap();
        let mut ships = kernel.ships(0);
        let link = ships.alloc().unwrap();
        let owner = ships.ship_id(link);
        ships.get_mut(link).hShip = Some(element);
        ships.get_mut(link).ship_facing = 9;
        ships.put(link);
        let record = kernel.elements.fixture_record_mut(element);
        record.state_flags = 1 << 2;
        record.parent = crate::engine::battle::Parent::Ship(owner);
        kernel.elements.put(element);
        let captured = CampaignView::capture(&world, &kernel);
        kernel.ships(0).get_mut(link).ship_facing = 11;
        assert_eq!(captured.flight_facing, Some(9));
        assert_eq!(
            CampaignView::capture(&world, &kernel).flight_facing,
            Some(11)
        );
    }

    #[test]
    fn capture_preserves_queue_order_and_freezes_the_supplied_world() {
        let mut world = World::new();
        let kernel = Kernel::new();
        world.data.flagship.fuel_on_board = 42;
        {
            let mut ships = world.built_ships();
            for (race, crew) in [(3, 12), (7, 19)] {
                let id = ships.alloc().unwrap();
                ships.get_mut(id).race_id = race;
                ships.get_mut(id).crew_level = crew;
                ships.put(id);
            }
        }
        {
            let mut ships = world.npc_built_ships();
            let id = ships.alloc().unwrap();
            ships.get_mut(id).race_id = 5;
            ships.put(id);
        }
        {
            let mut groups = world.ip_groups();
            for (race, in_system, location) in [(2, 1, 8), (4, 0, 9), (6, 1, 10)] {
                let id = groups.alloc().unwrap();
                let group = groups.get_mut(id);
                group.race_id = race;
                group.in_system = in_system;
                group.sys_loc = location;
                group.loc.x = i16::from(race);
                group.loc.y = -i16::from(race);
                groups.put(id);
            }
        }
        let captured = CampaignView::capture(&world, &kernel);
        world.data.flagship.fuel_on_board = 99;
        let first = world.built_ships_ref().head().unwrap();
        world.built_ships().get_mut(first).crew_level = 99;
        assert_eq!(captured.flagship.fuel_on_board, 42);
        assert_eq!(captured.contact_race, Some(5));
        assert_eq!(captured.escorts, vec![(3, 12), (7, 19)]);
        assert_eq!(captured.groups, vec![(2, 8, (2, -2)), (6, 10, (6, -6))]);
        assert_eq!(CampaignView::capture(&world, &kernel).escorts[0], (3, 99));
    }
}
