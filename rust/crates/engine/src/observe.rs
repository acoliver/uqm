//! Observation ports specialized to the owned campaign view.

pub use super::observe_port::{
    CheckpointView, DeviceObservation, Injected, InputSetup, InputView, Observer, PeerSetup,
    Presented, StarMapView, Unobserved,
};
pub mod battle;
use super::{Kernel, World};
pub type Io<'a> = super::observe_port::ObservationIo<'a, Option<CampaignView>>;

/// Values copied for one observer call; this view never outlives its input boundary.
#[derive(Clone)]
pub struct CampaignView {
    pub activity: u16,
    pub flagship: super::world::SisState,
    pub velocity: (i32, i32),
    pub ship_position: (i16, i16),
    pub ship_facing: i32,
    pub flight_facing: Option<u8>,
    pub autopilot: (i16, i16),
    pub combat_flags: u8,
    pub instant_move: bool,
    pub date: (u8, u8, u16),
    pub story: Option<super::world::story::StoryState>,
    pub current_star: Option<super::StarId>,
    pub contact_race: Option<u8>,
    pub escorts: Vec<(u8, u16)>,
    pub player_ships: Vec<crate::battle::link_queue::PickShip>,
    pub groups: Vec<(u8, u8, (i16, i16))>,
}

impl CampaignView {
    pub fn capture(world: &World, kernel: &Kernel) -> Self {
        let game = &world.data.game;
        let mut view = Self::from_world(world);
        view.instant_move = kernel.instant_move;
        view.flight_facing =
            super::save::hyper_save::read_flagship_facing(&kernel.races[0], &kernel.elements)
                .ok()
                .flatten()
                .and_then(|facing| u8::try_from(facing).ok());
        if !game.ship_stamp.frame.is_null() {
            // SAFETY: the supplied world retains this game-thread graphics frame.
            view.ship_facing = i32::from(unsafe {
                crate::graphics::pixmap_abi::GetFrameIndex(game.ship_stamp.frame.cast())
            });
        }
        let ships = kernel.ships_ref(0);
        let mut next = ships.head();
        while let Some(id) = next {
            view.player_ships.push(ships.get(id));
            next = ships.succ(id);
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

    pub fn for_input(view: InputView, capture: impl FnOnce() -> Self) -> Option<Self> {
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
            instant_move: false,
            contact_race: None,
            escorts: Vec::new(),
            player_ships: Vec::new(),
            groups: Vec::new(),
        }
    }

    pub fn story_value(&self, key: super::StoryKey) -> Option<u8> {
        self.story
            .as_ref()
            .map(|story| story.get_state(key.start(), key.end()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_reads_flagship_facing_from_its_unpublished_kernel() {
        let mut world = World::new();
        let mut kernel = super::super::Kernel::new();
        kernel.reset_ship_capacity(0, 1);
        kernel.reset_element_capacity(1);
        let element = kernel.elements.alloc().unwrap();
        let mut ships = kernel.ships(0);
        let link = ships.alloc().unwrap();
        let owner = ships.ship_id(link);
        ships.put(link);
        let pool = kernel.races[0].fixture_pool();
        let key = pool.id_at(usize::from(owner.slot())).unwrap();
        pool.get_mut(key).hShip = Some(element);
        let mut battle = crate::battle::Battle::new();
        crate::battle::Cx::new(&mut battle, &mut world, &mut kernel, crate::math::random)
            .ship_state_mut(owner)
            .facing = 9;
        let record = kernel.elements.fixture_record_mut(element);
        record.state_flags = 1 << 2;
        record.parent = super::super::battle::Parent::Ship(owner);
        kernel.elements.put(element);
        let captured = CampaignView::capture(&world, &kernel);
        crate::battle::Cx::new(&mut battle, &mut world, &mut kernel, crate::math::random)
            .ship_state_mut(owner)
            .facing = 11;
        assert_eq!(captured.flight_facing, Some(9));
        assert_eq!(
            CampaignView::capture(&world, &kernel).flight_facing,
            Some(11)
        );
    }

    #[test]
    fn capture_freezes_player_ship_values_in_link_order_without_publication() {
        let world = World::new();
        let mut kernel = Kernel::new();
        kernel.reset_ship_capacity(0, 3);
        for (species, index, crew) in [(4, 7, 9), (0, 2, 0), (1, 0, 42)] {
            let mut ships = kernel.ships(0);
            let id = ships.alloc().unwrap();
            ships.edit(id, |ship| {
                ship.species = species;
                ship.index = index;
                ship.crew = crew;
            });
            ships.put(id);
        }
        let captured = CampaignView::capture(&world, &kernel);
        let first = kernel.ships_ref(0).head().unwrap();
        kernel.ships(0).edit(first, |ship| ship.crew = 1);
        assert_eq!(
            captured
                .player_ships
                .iter()
                .map(|ship| (ship.species, ship.index, ship.crew))
                .collect::<Vec<_>>(),
            [(4, 7, 9), (0, 2, 0), (1, 0, 42)],
        );
        assert_eq!(
            CampaignView::capture(&world, &kernel).player_ships[0].crew,
            1
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
