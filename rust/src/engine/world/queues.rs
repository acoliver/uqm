//! Campaign records live in fixed-capacity pools; links belong to the pool, not the records.

use crate::collections::pool::OrderedPool;
use crate::engine::battle::link_queue::{queue_record, LinkId, LinkQueue, QueueRecord};
use crate::navigation::abi::{ENCOUNTER, IP_GROUP};
use crate::ships::native::abi::{FLEET_INFO, SHIP_FRAGMENT};

macro_rules! campaign_record {
    ($marker:ident, $record:ty) => {
        #[derive(Debug, PartialEq, Eq)]
        pub enum $marker {}
        impl queue_record::Sealed for $marker {}
        impl QueueRecord for $marker {
            type Record = $record;
        }
        impl<const N: usize> LinkQueue<'_, $marker, N> {
            pub fn get(&self, id: LinkId<$marker>) -> &$record {
                self.record(id)
            }
        }
    };
}
campaign_record!(Fleet, FLEET_INFO);
campaign_record!(Fragment, SHIP_FRAGMENT);
campaign_record!(Group, IP_GROUP);
campaign_record!(Encounter, ENCOUNTER);

pub struct CampaignQueues {
    pub(crate) avail_races: OrderedPool<FLEET_INFO, 24>,
    pub(crate) built_ships: OrderedPool<SHIP_FRAGMENT, 12>,
    pub(crate) npc_built_ships: OrderedPool<SHIP_FRAGMENT, 14>,
    pub(crate) ip_groups: OrderedPool<IP_GROUP, 32>,
    pub(crate) encounters: OrderedPool<ENCOUNTER, 16>,
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
        template: &FLEET_INFO,
        race: u8,
        crew: u16,
        mut pick_name: impl FnMut() -> u8,
    ) -> Option<LinkId<Fragment>> {
        let captain = if race == crate::ships::native::abi::SAMATRA_SHIP as u8 {
            0
        } else {
            loop {
                let name = pick_name();
                let mut current = self.head();
                let mut used = false;
                while let Some(id) = current {
                    let ship = self.get(id);
                    if ship.SpeciesID == template.SpeciesID && ship.captains_name_index == name {
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
        ship.SpeciesID = template.SpeciesID;
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
