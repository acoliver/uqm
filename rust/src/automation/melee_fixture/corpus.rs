//! Fourteen bounded production engagements for each seven-ship fleet.

use super::*;
use crate::collections::observation::{inspect_pending, Event};

pub(super) struct Run {
    request: CorpusRequest,
    pairs: Vec<[u8; 2]>,
    pair: usize,
    stage: u8,
    wait: u16,
    active: [HLINK; 2],
    selected: BTreeSet<u32>,
    observations: Observations,
}

struct Observations {
    coverage: CorpusCoverage,
    parents: BTreeMap<usize, usize>,
    registry: Vec<(usize, &'static str)>,
    seen_pending: usize,
}

impl Observations {
    fn observe(&mut self) -> Result<(), String> {
        inspect_pending(|events| -> Result<(), String> {
            for event in &events[self.seen_pending..] {
                match event {
                    Event::Spawn {
                        address,
                        side,
                        species,
                    } => {
                        self.coverage.spawn(*side, *species)?;
                        self.parents.insert(*address, *side as usize);
                    }
                    Event::Hook { parent, callback } => {
                        if let Some(side) = self.parents.get(parent) {
                            self.coverage.hook(
                                *side,
                                crate::ships::hook_registry::name(&self.registry, *callback),
                            );
                        }
                    }
                    Event::Allocate(_)
                    | Event::Drawable(_)
                    | Event::Free(_)
                    | Event::ResetPool
                    | Event::SpaceActive(_)
                    | Event::Frame { .. }
                    | Event::FrameSet { .. }
                    | Event::ImageFrame { .. }
                    | Event::ImageSet { .. }
                    | Event::ImageCopy { .. }
                    | Event::DrawableFree(_)
                    | Event::ResourcesFree { .. } => {}
                }
            }
            Ok(())
        })?;
        checkpoint();
        self.seen_pending = inspect_pending(<[_]>::len);
        Ok(())
    }
}

impl Run {
    pub(super) fn new(
        request: CorpusRequest,
        registry: Vec<(usize, &'static str)>,
    ) -> Result<Self, String> {
        Ok(Self {
            request,
            pairs: request.bouts()?,
            pair: 0,
            stage: 0,
            wait: 0,
            active: [ptr::null_mut(); 2],
            selected: BTreeSet::new(),
            observations: Observations {
                coverage: CorpusCoverage::default(),
                parents: BTreeMap::new(),
                registry,
                seen_pending: 0,
            },
        })
    }

    pub(super) fn needed_species(&self) -> Vec<u8> {
        FLEETS[usize::from(self.request.fleet)].to_vec()
    }

    pub(super) fn advance(
        &mut self,
        world: &mut World<'_>,
        catalog: &BTreeMap<u8, u32>,
    ) -> Result<Option<Vec<Effect>>, String> {
        match self.stage {
            0 => {
                world.fighters()?;
                self.stage = 1;
                Ok(Some(vec![Effect::Reset]))
            }
            1 => {
                reset(world, &FLEETS[usize::from(self.request.fleet)], catalog);
                let pair = self.pairs[self.pair];
                for side in 0..2 {
                    let species = catalog[&pair[side]];
                    self.selected.insert(species);
                    self.active[side] = *world
                        .ships
                        .iter()
                        .find(|(_, s)| {
                            s.state.playerNr == side as i16 && s.state.SpeciesID == species
                        })
                        .ok_or("requested species missing from loaded fleet")?
                        .0;
                    world
                        .ships
                        .get_mut(&self.active[side])
                        .ok_or("corpus requested owner is no longer queued")?
                        .state
                        .control = 2 | 64;
                }
                self.stage = 2;
                Ok(Some(self.active.into_iter().map(Effect::Spawn).collect()))
            }
            2 => {
                self.observations.observe()?;
                self.wait = 0;
                self.stage = 3;
                Ok(Some(vec![Effect::Redraw(false)]))
            }
            3 => self.arrival(world),
            4 => self.engage(world),
            _ => {
                self.observations
                    .coverage
                    .verify(&self.selected.iter().copied().collect::<Vec<_>>())?;
                Ok(None)
            }
        }
    }

    fn arrival(&mut self, world: &World<'_>) -> Result<Option<Vec<Effect>>, String> {
        self.observations.observe()?;
        if self.ended(world)? {
            return Err("production corpus ship died during arrival".into());
        }
        if self
            .active
            .iter()
            .all(|h| arrived(world.elements[&world.ships[h].state.hShip.cast()]))
        {
            self.stage = 4;
            self.wait = 0;
            return Ok(Some(vec![self.think(world)?, Effect::Redraw(false)]));
        }
        self.wait += 1;
        if self.wait == 96 {
            return Err("production corpus arrivals did not complete".into());
        }
        Ok(Some(vec![Effect::Redraw(false)]))
    }

    fn engage(&mut self, world: &World<'_>) -> Result<Option<Vec<Effect>>, String> {
        self.observations.observe()?;
        self.wait += 1;
        if self.ended(world)? || self.wait == 96 {
            self.pair += 1;
            self.stage = if self.pair == self.pairs.len() { 5 } else { 1 };
            return Ok(Some(if self.stage == 1 {
                vec![Effect::Reset]
            } else {
                vec![]
            }));
        }
        Ok(Some(vec![self.think(world)?, Effect::Redraw(false)]))
    }

    fn think(&self, world: &World<'_>) -> Result<Effect, String> {
        let mut active = [(ptr::null_mut(), 0); 2];
        for (slot, h) in active.iter_mut().zip(self.active) {
            let ship = &world.ships[&h];
            *slot = (
                ship.state.hShip.cast(),
                ship.info
                    .as_ref()
                    .ok_or("corpus descriptor is unavailable")?
                    .crew_level,
            );
        }
        Ok(Effect::Think(active))
    }

    fn ended(&self, world: &World<'_>) -> Result<bool, String> {
        for h in &self.active {
            let ship = &world.ships[h];
            assert!(!ship.state.hShip.is_null(), "active corpus element");
            if ship
                .info
                .as_ref()
                .ok_or("active corpus descriptor is unavailable")?
                .crew_level
                == 0
                || ship.state.SpeciesID == SPECIES_ID_NO_ID
            {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn reset(world: &mut World<'_>, fleet: &[u8; 7], catalog: &BTreeMap<u8, u32>) {
    for ship in world.ships.values_mut() {
        let slot = usize::from(ship.state.index);
        assert!(slot < fleet.len(), "configured corpus fleet slot");
        ship.state.hShip = ptr::null_mut();
        ship.state.SpeciesID = catalog[&fleet[slot]];
        ship.state.crew_level = 0;
        ship.state.ship_input_state = 0;
        ship.state.cur_status_flags = 0;
        ship.state.old_status_flags = 0;
    }
}
