//! Scoped live records and fixture phases. No record borrow crosses an engine call.

use super::*;
use crate::battle::element::ElementFlags;
use crate::engine::battle::access::fixture::{Element, Phoenix, Ship};
use crate::engine::battle::views::{Frame, Frames};
#[cfg(not(test))]
use crate::engine::battle::Cx;
use crate::engine::battle::{Concern, ElementId, Parent, ShipId};
use crate::graphics::native_abi::POINT as Point;
use crate::ships::native::abi::*;
use std::{mem, ptr};

#[path = "combat.rs"]
mod combat;
#[path = "corpus.rs"]
mod corpus;
#[path = "frames.rs"]
mod frames;
#[path = "production.rs"]
mod production;
#[path = "rebirth.rs"]
mod rebirth;
#[path = "star.rs"]
mod star;

#[cfg(not(test))]
pub(super) use production::execute;

struct World {
    elements: BTreeMap<ElementId, Element>,
    order: Vec<ElementId>,
    ships: BTreeMap<ShipId, Ship>,
    ship_order: Vec<ShipId>,
    activity: u16,
    counters: [u8; 2],
    star: usize,
}
impl World {
    fn new(activity: u16, counters: [u8; 2], star: usize) -> Self {
        Self {
            elements: BTreeMap::new(),
            order: Vec::new(),
            ships: BTreeMap::new(),
            ship_order: Vec::new(),
            activity,
            counters,
            star,
        }
    }
    fn owner(&self, element: ElementId) -> ShipId {
        match self.elements[&element].parent {
            Parent::Ship(id) => id,
            other => panic!("fixture element {element:?} has parent {other:?}"),
        }
    }
    fn ship(&self, element: ElementId) -> &Ship {
        &self.ships[&self.owner(element)]
    }
    fn ship_mut(&mut self, element: ElementId) -> &mut Ship {
        let owner = self.owner(element);
        self.ships
            .get_mut(&owner)
            .unwrap_or_else(|| panic!("fixture owner {owner:?} is not queued"))
    }
    fn fighters(&self) -> Result<[ElementId; 2], String> {
        let mut result = [None; 2];
        for (&id, element) in &self.elements {
            if element.is_player_ship() {
                let side = usize::try_from(element.player)
                    .map_err(|_| "live fixture player ship has a negative side")?;
                assert!(side < 2, "ship side");
                result[side] = Some(id);
            }
        }
        let [Some(a), Some(b)] = result else {
            return Err("parity fixture requires two loaded live battle ships".into());
        };
        if [a, b].iter().any(|id| !arrived(&self.elements[id])) {
            return Err("parity fixture requires completed ship arrivals".into());
        }
        Ok([a, b])
    }
    #[cfg(not(test))]
    fn commit(&self, cx: &mut Cx<'_>) {
        for (&id, element) in &self.elements {
            *cx.element_mut(id) = element.state;
        }
        for (&id, ship) in &self.ships {
            cx.apply_fixture_ship(id, ship);
        }
    }
}

fn arrived(element: &Element) -> bool {
    element.flags
        & (ElementFlags::APPEARING | ElementFlags::NONSOLID | ElementFlags::DISAPPEARING).bits()
        == 0
}
fn place(element: &mut Element, x: i16, y: i16) {
    element.current.location = Point { x, y };
    element.next.location = element.current.location;
    element.velocity.zero();
    element.flags |= ElementFlags::CHANGING.bits();
}

fn checkpoint() {
    crate::automation::coordinator::Coordinator::parity_checkpoint("fixture");
}

struct Hooks {
    blazer_collision: usize,
    satellite: crate::engine::battle::record::ElementProcessFunc,
    defense: usize,
    limpets: crate::engine::battle::record::ElementProcessFunc,
    fighters: crate::engine::battle::record::ElementProcessFunc,
    fighter_preprocess: usize,
    pkunk: Option<crate::engine::battle::race::ShipFn>,
    intercept: usize,
    rebirth: usize,
    ship_death: crate::engine::battle::record::ElementProcessFunc,
    star_collision: usize,
    unhyper: usize,
}

/// Only operations that invalidate live borrows leave the safe phase logic.
enum Effect {
    RaceProcess(bool, ElementId),
    Invoke(crate::engine::battle::record::ElementProcessFunc, ElementId),
    Death(ElementId),
    Weapon(ElementId),
    Collide(ElementId, ElementId),
    Redraw(bool),
    Intersect(ElementId),
    Charge(ElementId, i16),
    Damage(ElementId, i16),
    Crew(ElementId, i16),
    Remove(Vec<ElementId>),
    PhoenixFirst(ElementId),
    ProbePool,
    ProbeRecovery,
    Spawn(ShipId),
    RetainDescriptor(ShipId),
    FreeRetained,
    Reset,
    Think([(ElementId, u16); 2]),
    Intelligence(Option<usize>, ElementId, [Concern; 4]),
    Scale(i32),
    RestoreScale,
    ZoomProof(Frames, ElementId, usize, u16),
    Captain(Frame),
    ClearStar,
}

struct Battle {
    proof: FixtureRun,
    stage: u16,
    a: Option<ElementId>,
    b: Option<ElementId>,
    object: Option<ElementId>,
    replacement: Option<ElementId>,
    owner: Option<ShipId>,
    species: u32,
    before: BTreeSet<ElementId>,
    array: Option<Frames>,
    saved_hook: usize,
    crew: u16,
    wait: u16,
    phoenix: Option<Phoenix>,
    star: usize,
    velocity: crate::battle::velocity::VelocityDesc,
    captain: Frame,
}

impl Battle {
    fn a(&self) -> ElementId {
        self.a
            .unwrap_or_else(|| panic!("fixture first fighter selected"))
    }
    fn b(&self) -> ElementId {
        self.b
            .unwrap_or_else(|| panic!("fixture second fighter selected"))
    }
    fn object(&self) -> ElementId {
        self.object
            .unwrap_or_else(|| panic!("fixture object selected"))
    }
    fn replacement(&self) -> ElementId {
        self.replacement
            .unwrap_or_else(|| panic!("fixture replacement selected"))
    }
    fn owner(&self) -> ShipId {
        self.owner
            .unwrap_or_else(|| panic!("fixture owner selected"))
    }
    fn array(&self) -> Frames {
        self.array
            .unwrap_or_else(|| panic!("fixture frame array selected"))
    }

    fn new(name: FixtureName) -> Self {
        Self {
            proof: FixtureRun::new(name),
            stage: 0,
            a: None,
            b: None,
            object: None,
            replacement: None,
            owner: None,
            species: 0,
            before: BTreeSet::new(),
            array: None,
            saved_hook: 0,
            crew: 0,
            wait: 0,
            phoenix: None,
            star: 0,
            velocity: crate::battle::velocity::VelocityDesc::new(),
            captain: Frame::ABSENT,
        }
    }

    fn start(&mut self, world: &mut World, species: [u32; 2]) -> Result<(), String> {
        if world.activity & IN_BATTLE as u16 == 0 {
            return Err("parity melee fixture requires a running production battle".into());
        }
        [self.a, self.b] = world.fighters()?.map(Some);
        for (handle, species) in [(self.a(), species[0]), (self.b(), species[1])] {
            let ship = world.ship_mut(handle);
            if ship.species != species {
                return Err(format!(
                    "{}: live opponents do not match fixture roster",
                    self.proof.fixture.name()
                ));
            }
            ship.state.input = 0;
        }
        Ok(())
    }

    fn advance(&mut self, world: &mut World, hooks: &Hooks) -> Result<Option<Vec<Effect>>, String> {
        match self.proof.fixture {
            FixtureName::AndrosynthBlazer => combat::blazer(self, world, hooks),
            FixtureName::PkunkRebirth => rebirth::advance(self, world, hooks),
            FixtureName::ChmmrSatellites => combat::satellites(self, world, hooks),
            FixtureName::DeathSpawn => combat::death_spawn(self, world, hooks),
            FixtureName::MixedCollision => combat::mixed(self, world, hooks),
            FixtureName::PoolFull => combat::pool(self, world, hooks),
            FixtureName::FrameSets => frames::advance(self, world, hooks),
            FixtureName::StarContact => unreachable!(),
        }
    }

    fn next(&mut self, effects: Vec<Effect>) -> Result<Option<Vec<Effect>>, String> {
        self.stage += 1;
        Ok(Some(effects))
    }

    fn traverse(&mut self) -> Result<Option<Vec<Effect>>, String> {
        assert!(self.proof.steps < 512, "bounded fixture traversal");
        self.proof.steps += 1;
        Ok(Some(vec![Effect::Redraw(true)]))
    }

    fn finish(&mut self, count: u16) -> Result<Option<Vec<Effect>>, String> {
        if self.wait == count {
            return Ok(None);
        }
        self.wait += 1;
        self.traverse()
    }

    fn prepare_weapon(&mut self, world: &mut World, handle: ElementId) -> Result<Effect, String> {
        self.before = world.elements.keys().copied().collect();
        let crew = world.elements[&handle].hits;
        let ship = world.ship_mut(handle);
        let info = ship
            .info
            .as_ref()
            .ok_or("fixture shooter descriptor is unavailable")?;
        assert!(info.crew_level > 0, "fixture shooter must be alive");
        assert_eq!(crew, info.crew_level, "fixture shooter crew union");
        ship.state.status = (ship.state.status & !(SPECIAL as u16)) | WEAPON as u16;
        ship.state.weapon_counter = 0;
        Ok(Effect::Charge(
            handle,
            i16::from(info.max_energy) - i16::from(info.energy_level),
        ))
    }

    fn weapon(&mut self, world: &mut World, shooter: ElementId) -> Result<ElementId, String> {
        world.ship_mut(shooter).state.status &= !(WEAPON as u16);
        let parent = world.elements[&shooter].parent;
        world
            .order
            .iter()
            .copied()
            .find(|h| !self.before.contains(h) && world.elements[h].parent == parent)
            .ok_or_else(|| "production weapon initializer did not spawn a weapon".into())
    }
}

enum Admission<'a> {
    Configure(&'a MeleeSetupRequest),
    Complete(Option<FixtureRun>),
    Task(Task<'a>),
}

fn admit<'a>(
    operation: Operation<'a>,
    activity: u16,
    mut position: SavedPosition,
    registry: Vec<(usize, &'static str)>,
) -> Result<Admission<'a>, String> {
    match operation {
        Operation::Configure(request) => {
            request.validate()?;
            Ok(Admission::Configure(request))
        }
        Operation::SaveWitness(proof, slot, loading) => {
            position.slot = slot;
            star::witness(proof, position, loading)?;
            Ok(Admission::Complete(None))
        }
        Operation::Fixture(FixtureName::StarContact) => {
            star::start(activity).map(|proof| Admission::Complete(Some(proof)))
        }
        other => Task::new(other, registry).map(Admission::Task),
    }
}

enum Task<'a> {
    Battle(Battle),
    Corpus(corpus::Run),
    Mutual(Battle),
    Star(&'a mut FixtureRun, Battle),
}

impl<'a> Task<'a> {
    fn new(operation: Operation<'a>, registry: Vec<(usize, &'static str)>) -> Result<Self, String> {
        Ok(match operation {
            Operation::Fixture(name) => Self::Battle(Battle::new(name)),
            Operation::Corpus(request) => Self::Corpus(corpus::Run::new(request, registry)?),
            Operation::MutualLoss => Self::Mutual(Battle::new(FixtureName::MixedCollision)),
            Operation::Advance(proof) => {
                let mut run = Battle::new(proof.fixture);
                mem::swap(&mut run.proof, proof);
                Self::Star(proof, run)
            }
            Operation::Configure(_) | Operation::SaveWitness(..) => unreachable!(),
        })
    }

    fn needed_species(&self) -> Vec<u8> {
        match self {
            Self::Battle(run) => run.proof.fixture.ships().to_vec(),
            Self::Corpus(run) => run.needed_species(),
            Self::Mutual(_) | Self::Star(..) => vec![],
        }
    }

    fn advance(
        &mut self,
        world: &mut World,
        hooks: &Hooks,
        catalog: &BTreeMap<u8, u32>,
    ) -> Result<Option<Vec<Effect>>, String> {
        match self {
            Self::Battle(run) => {
                if run.a.is_none() {
                    let required = run.proof.fixture.ships().map(|index| catalog[&index]);
                    run.start(world, required)?;
                }
                run.advance(world, hooks)
            }
            Self::Corpus(run) => run.advance(world, catalog),
            Self::Mutual(run) => combat::mutual_loss(run, world),
            Self::Star(_, run) => star::advance(run, world, hooks),
        }
    }

    fn reply(&mut self, reply: production::Reply) -> Result<(), String> {
        let run = match self {
            Self::Battle(run) | Self::Mutual(run) | Self::Star(_, run) => run,
            Self::Corpus(_) => return production::loaded(reply),
        };
        match reply {
            production::Reply::None => Ok(()),
            production::Reply::Loaded(value) => {
                production::loaded(production::Reply::Loaded(value))
            }
            production::Reply::Check(name, condition) => run.proof.require(condition, name),
            production::Reply::Captain(frame) => {
                run.captain = frame;
                Ok(())
            }
        }
    }

    fn finish(self) -> Result<Option<FixtureRun>, String> {
        match self {
            Self::Battle(run) => Ok(Some(run.proof)),
            Self::Star(target, mut run) => {
                mem::swap(target, &mut run.proof);
                Ok(None)
            }
            Self::Corpus(_) | Self::Mutual(_) => Ok(None),
        }
    }
}

#[cfg(test)]
struct Collision {
    callback: crate::engine::battle::record::ElementCollisionFunc,
    points: [Point; 2],
    parent: Parent,
}
#[cfg(test)]
fn collision(a: &Element, b: &Element) -> Collision {
    let point = |e: &Element| Point {
        x: e.next.location.x >> ONE_SHIFT,
        y: e.next.location.y >> ONE_SHIFT,
    };
    Collision {
        callback: a.collide,
        points: [point(a), point(b)],
        parent: a.parent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admitted_tasks_validate_before_touching_a_loaded_world() {
        let position = SavedPosition {
            slot: 0,
            log_x: 0,
            log_y: 0,
            star: None,
        };
        let setup = MeleeSetupRequest {
            ships: [[0; 7]; 2],
            cyborg: true,
        };
        let Admission::Configure(request) =
            admit(Operation::Configure(&setup), 0, position, vec![]).unwrap()
        else {
            panic!("configure admission");
        };
        assert_eq!(request, &setup);
        let Admission::Complete(Some(proof)) = admit(
            Operation::Fixture(FixtureName::StarContact),
            IN_INTERPLANETARY as u16,
            position,
            vec![],
        )
        .unwrap() else {
            panic!("star admission");
        };
        assert_eq!(proof.star.phase, 1);
        let Admission::Task(mut task) = admit(
            Operation::Corpus(CorpusRequest { fleet: 0 }),
            0,
            position,
            vec![],
        )
        .unwrap() else {
            panic!("corpus admission");
        };
        assert_eq!(task.needed_species(), FLEETS[0]);
        assert!(task
            .advance(
                &mut World::new(0, [0; 2], 0),
                &test_hooks(),
                &BTreeMap::new()
            )
            .is_err());
        task.reply(production::Reply::None).unwrap();
        assert!(task.finish().unwrap().is_none());
        let mut task = Task::new(Operation::MutualLoss, vec![]).unwrap();
        assert!(task.reply(production::Reply::Loaded(false)).is_err());
        assert!(task
            .reply(production::Reply::Check("not-observed", false))
            .is_err());
        task.reply(production::Reply::Captain(Frame::ABSENT))
            .unwrap();
    }

    pub(super) fn test_hooks() -> Hooks {
        Hooks {
            blazer_collision: 0,
            satellite: None,
            defense: 0,
            limpets: None,
            fighters: None,
            fighter_preprocess: 0,
            pkunk: None,
            intercept: 0,
            rebirth: 0,
            ship_death: None,
            star_collision: 0,
            unhyper: 0,
        }
    }

    #[test]
    fn placement_updates_the_union_free_live_record_without_changing_ownership_or_life() {
        let mut element = Element::new();
        element.life = 31;
        element.hits = 7;
        element.set_appearing();
        element.velocity.set_components(100, -90);
        place(&mut element, 4000, 4160);
        assert_eq!(element.current.location, Point { x: 4000, y: 4160 });
        assert_eq!(element.next.location, element.current.location);
        assert!(element.velocity.is_zero());
        assert!(element.flags & ElementFlags::CHANGING.bits() != 0);
        assert!(!arrived(&element));
        element.clear_appearing();
        assert!(arrived(&element));
        assert_eq!((element.life, element.hits), (31, 7));
    }
}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod phase_tests;
