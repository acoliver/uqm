//! Campaign records live in fixed-capacity pools; links belong to the pool, not the records.

use super::super::battle::link_queue::{
    queue_record, LinkId, LinkQueue, MutableQueueRecord, QueueRecord,
};
use crate::collections::pool::OrderedPool;
use crate::graphics::native_abi::{FRAME, POINT, STRING};

pub const SAMATRA_RACE: u8 = 23;

#[derive(Debug, Copy, Clone, Default)]
pub struct ShipFragment {
    pub species_id: u32,
    pub captains_name_index: u8,
    pub race_id: u8,
    pub index: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub energy_level: u8,
    pub max_energy: u8,
    pub race_strings: STRING,
    pub icons: FRAME,
    pub melee_icon: FRAME,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct FleetInfo {
    pub species_id: u32,
    pub allied_state: u16,
    pub days_left: u8,
    pub growth_fract: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub growth: u8,
    pub max_energy: u8,
    pub loc: POINT,
    pub race_strings: STRING,
    pub icons: FRAME,
    pub melee_icon: FRAME,
    pub actual_strength: u16,
    pub known_strength: u16,
    pub known_loc: POINT,
    pub growth_err_term: u8,
    pub func_index: u8,
    pub dest_loc: POINT,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct BriefShipInfo {
    pub race_id: u8,
    pub crew_level: u16,
    pub max_crew: u16,
    pub max_energy: u8,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct EncounterRecord {
    pub h_element: *mut std::ffi::c_void,
    pub transition_state: i16,
    pub origin: POINT,
    pub radius: u16,
    pub race_id: u8,
    pub num_ships: u8,
    pub flags: u8,
    pub loc_pt: POINT,
    pub ship_list: [BriefShipInfo; 7usize],
    pub log_x: i32,
    pub log_y: i32,
}

#[derive(Debug, Copy, Clone, Default)]
pub struct IpGroup {
    pub group_counter: u16,
    pub race_id: u8,
    pub sys_loc: u8,
    pub task: u8,
    pub in_system: u8,
    pub dest_loc: u8,
    pub orbit_pos: u8,
    pub group_id: u8,
    pub loc: POINT,
    pub melee_icon: FRAME,
}

impl IpGroup {
    pub const IN_ORBIT: u8 = 0;
    pub const EXPLORE: u8 = 1;
    pub const FLEE: u8 = 2;
    pub const ON_STATION: u8 = 3;
    pub const IGNORE_FLAGSHIP: u8 = 4;
    pub const REFORM_GROUP: u8 = 8;
}

#[derive(Debug, PartialEq, Eq)]
pub enum Fleet {}
impl queue_record::Sealed for Fleet {}
impl MutableQueueRecord for Fleet {}
impl QueueRecord for Fleet {
    type Record = FleetInfo;
}
impl<const N: usize> LinkQueue<'_, Fleet, N> {
    pub fn get(&self, id: LinkId<Fleet>) -> &FleetInfo {
        self.record(id)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Fragment {}
impl queue_record::Sealed for Fragment {}
impl MutableQueueRecord for Fragment {}
impl QueueRecord for Fragment {
    type Record = ShipFragment;
}
impl<const N: usize> LinkQueue<'_, Fragment, N> {
    pub fn get(&self, id: LinkId<Fragment>) -> &ShipFragment {
        self.record(id)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Group {}
impl queue_record::Sealed for Group {}
impl MutableQueueRecord for Group {}
impl QueueRecord for Group {
    type Record = IpGroup;
}
impl<const N: usize> LinkQueue<'_, Group, N> {
    pub fn get(&self, id: LinkId<Group>) -> &IpGroup {
        self.record(id)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Encounter {}
impl queue_record::Sealed for Encounter {}
impl MutableQueueRecord for Encounter {}
impl QueueRecord for Encounter {
    type Record = EncounterRecord;
}
impl<const N: usize> LinkQueue<'_, Encounter, N> {
    pub fn get(&self, id: LinkId<Encounter>) -> &EncounterRecord {
        self.record(id)
    }
}

pub struct CampaignQueues {
    pub avail_races: OrderedPool<FleetInfo, 24>,
    pub built_ships: OrderedPool<ShipFragment, 12>,
    pub npc_built_ships: OrderedPool<ShipFragment, 14>,
    pub ip_groups: OrderedPool<IpGroup, 32>,
    pub encounters: OrderedPool<EncounterRecord, 16>,
}

impl CampaignQueues {
    pub fn new() -> Self {
        Self {
            avail_races: OrderedPool::new(),
            built_ships: OrderedPool::new(),
            npc_built_ships: OrderedPool::new(),
            ip_groups: OrderedPool::new(),
            encounters: OrderedPool::new(),
        }
    }

    pub fn clear(&mut self) {
        self.avail_races.reinit();
        self.built_ships.reinit();
        self.npc_built_ships.reinit();
        self.ip_groups.reinit();
        self.encounters.reinit();
    }
}

impl Default for CampaignQueues {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> LinkQueue<'_, Fragment, N> {
    pub fn clone_fragment(
        &mut self,
        template: &FleetInfo,
        race: u8,
        crew: u16,
        mut pick_name: impl FnMut() -> u8,
    ) -> Option<LinkId<Fragment>> {
        let captain = if race == SAMATRA_RACE {
            0
        } else {
            loop {
                let name = pick_name();
                let mut current = self.head();
                let mut used = false;
                while let Some(id) = current {
                    let ship = self.get(id);
                    if ship.species_id == template.species_id && ship.captains_name_index == name {
                        used = true;
                        break;
                    }
                    current = self.succ(id);
                }
                if !used {
                    break name;
                }
            }
        };
        let id = self.alloc()?;
        let ship = self.get_mut(id);
        ship.species_id = template.species_id;
        ship.captains_name_index = captain;
        ship.race_id = race;
        ship.crew_level = if crew == 0 { template.crew_level } else { crew };
        ship.max_crew = template.max_crew;
        ship.max_energy = template.max_energy;
        ship.race_strings = template.race_strings;
        ship.icons = template.icons;
        ship.melee_icon = template.melee_icon;
        self.put(id);
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_tasks_preserve_the_saved_nibble_and_independent_flags() {
        assert_eq!(
            [
                IpGroup::IN_ORBIT,
                IpGroup::EXPLORE,
                IpGroup::FLEE,
                IpGroup::ON_STATION
            ],
            [0, 1, 2, 3]
        );
        assert_eq!((IpGroup::IGNORE_FLAGSHIP, IpGroup::REFORM_GROUP), (4, 8));
        for task in [
            IpGroup::IN_ORBIT,
            IpGroup::EXPLORE,
            IpGroup::FLEE,
            IpGroup::ON_STATION,
        ] {
            let marked = task | IpGroup::IGNORE_FLAGSHIP | IpGroup::REFORM_GROUP;
            assert_eq!(
                marked & !(IpGroup::IGNORE_FLAGSHIP | IpGroup::REFORM_GROUP),
                task
            );
            assert_eq!(marked & 0xf0, 0);
        }
    }

    #[test]
    fn fragment_clones_preserve_template_fields_and_same_species_name_uniqueness() {
        let template = FleetInfo {
            species_id: 7,
            crew_level: 10,
            max_crew: 20,
            max_energy: 30,
            ..Default::default()
        };
        let mut pool = OrderedPool::<ShipFragment, 3>::new();
        let mut ships = LinkQueue::<Fragment, 3>::from_pool(&mut pool);
        let first = ships.clone_fragment(&template, 6, 0, || 5).unwrap();
        let mut candidates = [5, 6].into_iter();
        let second = ships
            .clone_fragment(&template, 6, 1, || candidates.next().unwrap())
            .unwrap();
        assert_eq!(ships.get(first).crew_level, 10);
        assert_eq!(ships.get(first).max_crew, 20);
        assert_eq!(ships.get(first).max_energy, 30);
        assert_eq!(ships.get(first).energy_level, 0);
        assert_eq!(ships.get(first).race_id, 6);
        assert_eq!(ships.get(second).crew_level, 1);
        assert_eq!(ships.get(second).captains_name_index, 6);
        assert_eq!(ships.succ(first), Some(second));
        let other = FleetInfo {
            species_id: 8,
            ..template
        };
        let third = ships.clone_fragment(&other, 7, 0, || 5).unwrap();
        assert_eq!(ships.get(third).captains_name_index, 5);
        assert_eq!(ships.tail(), Some(third));
    }

    #[test]
    fn full_fragment_pool_still_draws_a_name_before_allocation_fails() {
        let template = FleetInfo::default();
        let mut pool = OrderedPool::<ShipFragment, 1>::new();
        let mut ships = LinkQueue::<Fragment, 1>::from_pool(&mut pool);
        ships.clone_fragment(&template, 0, 0, || 5).unwrap();
        let mut draws = 0;
        assert!(ships
            .clone_fragment(&template, 0, 0, || {
                draws += 1;
                6
            })
            .is_none());
        assert_eq!(draws, 1);
        assert_eq!(ships.count(), 1);
    }

    #[test]
    fn samatra_captain_does_not_draw_a_name() {
        let template = FleetInfo::default();
        let mut pool = OrderedPool::<ShipFragment, 1>::new();
        let mut ships = LinkQueue::<Fragment, 1>::from_pool(&mut pool);
        let id = ships
            .clone_fragment(&template, SAMATRA_RACE, 0, || {
                panic!("Sa-Matra has a fixed captain")
            })
            .unwrap();
        assert_eq!(ships.get(id).captains_name_index, 0);
    }
}
