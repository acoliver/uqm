//! Scoped live records and fixture phases. No record borrow crosses an engine call.

use super::*;
use crate::battle::element::{Element, ElementFlags, HElement, Point};
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

struct Ship<'a> {
    #[cfg(not(test))]
    callback_addresses: [usize; 5],
    state: &'a mut STARSHIP,
    info: Option<&'a mut SHIP_INFO>,
    data: Option<&'a mut DATA_STUFF>,
    characteristics: Option<&'a mut CHARACTERISTIC_STUFF>,
    preprocess: PREPROCESS_FUNC,
    postprocess: POSTPROCESS_FUNC,
    intelligence: IntelligenceFunc,
    phoenix: Option<Phoenix>,
}

impl<'a> Ship<'a> {
    fn descriptor(
        &mut self,
        info: &'a mut SHIP_INFO,
        data: &'a mut DATA_STUFF,
        characteristics: &'a mut CHARACTERISTIC_STUFF,
        hooks: (PREPROCESS_FUNC, POSTPROCESS_FUNC, IntelligenceFunc),
    ) {
        self.info = Some(info);
        self.data = Some(data);
        self.characteristics = Some(characteristics);
        (self.preprocess, self.postprocess, self.intelligence) = hooks;
    }
}

#[derive(Clone, Copy)]
struct Phoenix {
    handle: HElement,
    hooks: [usize; 3],
}

struct World<'a> {
    elements: BTreeMap<HElement, &'a mut Element>,
    order: Vec<HElement>,
    #[cfg(not(test))]
    descriptors: BTreeMap<HLINK, *mut *mut RACE_DESC>,
    ships: BTreeMap<HLINK, Ship<'a>>,
    #[cfg(not(test))]
    ship_order: Vec<HLINK>,
    activity: u16,
    counters: [u8; 2],
    star: usize,
}

impl<'a> World<'a> {
    fn new(activity: u16, counters: [u8; 2], star: usize) -> Self {
        Self {
            elements: BTreeMap::new(),
            order: Vec::new(),
            ships: BTreeMap::new(),
            #[cfg(not(test))]
            ship_order: Vec::new(),
            #[cfg(not(test))]
            descriptors: BTreeMap::new(),
            activity,
            counters,
            star,
        }
    }

    fn insert_element(&mut self, record: &'a mut Element) -> HElement {
        let handle = ptr::from_mut(record);
        let next = record.succ;
        self.order.push(handle);
        self.elements.insert(handle, record);
        next
    }

    #[cfg(not(test))]
    fn insert_ship(&mut self, ship: Ship<'a>) -> HLINK {
        let owner = ptr::from_mut(ship.state).cast();
        let next = ship.state.succ;
        self.descriptors
            .insert(owner, &raw mut ship.state.RaceDescPtr);
        self.ship_order.push(owner);
        self.ships.insert(owner, ship);
        next
    }

    fn ship(&self, handle: HElement) -> &Ship<'a> {
        &self.ships[&self.elements[&handle].p_parent]
    }

    fn ship_mut(&mut self, handle: HElement) -> &mut Ship<'a> {
        match self.ships.get_mut(&self.elements[&handle].p_parent) {
            Some(ship) => ship,
            None => panic!("live fixture element {handle:p} has no queued owner"),
        }
    }

    fn fighters(&self) -> Result<[HElement; 2], String> {
        let mut result = [ptr::null_mut(); 2];
        for (&handle, element) in &self.elements {
            if element.is_player_ship() {
                let side = usize::try_from(element.player_nr)
                    .map_err(|_| "live fixture player ship has a negative side")?;
                assert!(side < 2, "ship side");
                result[side] = handle;
            }
        }
        if result.iter().any(|h| h.is_null()) {
            return Err("parity fixture requires two loaded live battle ships".into());
        }
        if result.iter().any(|h| !arrived(self.elements[h])) {
            return Err("parity fixture requires completed ship arrivals".into());
        }
        Ok(result)
    }
}

fn arrived(element: &Element) -> bool {
    !element
        .state_flags
        .intersects(ElementFlags::APPEARING | ElementFlags::NONSOLID | ElementFlags::DISAPPEARING)
}

fn place(element: &mut Element, x: i16, y: i16) {
    element.current.location = Point::new(x, y);
    element.next.location = element.current.location;
    element.velocity.zero();
    element.state_flags.insert(ElementFlags::CHANGING);
}

fn checkpoint() {
    crate::automation::coordinator::Coordinator::parity_checkpoint("fixture");
}

struct Hooks {
    blazer_collision: usize,
    satellite: PREPROCESS_FUNC,
    defense: usize,
    limpets: crate::engine::battle::record::ElementProcessFunc,
    fighters: PREPROCESS_FUNC,
    fighter_preprocess: usize,
    pkunk: PREPROCESS_FUNC,
    intercept: usize,
    rebirth: usize,
    ship_death: crate::engine::battle::record::ElementProcessFunc,
    star_collision: usize,
    unhyper: usize,
}

/// Only operations that invalidate live borrows leave the safe phase logic.
enum Effect {
    RaceProcess(bool, HElement),
    Invoke(crate::engine::battle::record::ElementProcessFunc, HElement),
    Death(HElement),
    Weapon(HElement),
    Collide(HElement, HElement),
    Redraw(bool),
    Intersect(HElement),
    Charge(HElement, i16),
    Damage(HElement, i16),
    Crew(HElement, i16),
    Remove(Vec<HElement>),
    PhoenixFirst(HElement),
    ProbePool,
    ProbeRecovery,
    Spawn(HLINK),
    RetainDescriptor(HLINK),
    FreeRetained,
    Reset,
    Think([(HElement, u16); 2]),
    Intelligence(IntelligenceFunc, HElement, [EVALUATE_DESC; 10]),
    Scale(i32),
    RestoreScale,
    ZoomProof([FRAME; 3], HElement, usize, u16),
    Captain(FRAME),
    ClearStar,
}

struct Battle {
    proof: FixtureRun,
    stage: u16,
    a: HElement,
    b: HElement,
    object: HElement,
    replacement: HElement,
    owner: HLINK,
    species: u32,
    before: BTreeSet<HElement>,
    array: *mut crate::battle::element::FrameHandle,
    saved_hook: usize,
    crew: u16,
    wait: u16,
    phoenix: Option<Phoenix>,
    star: usize,
    velocity: crate::battle::velocity::VelocityDesc,
    captain: FRAME,
}

impl Battle {
    fn new(name: FixtureName) -> Self {
        Self {
            proof: FixtureRun::new(name),
            stage: 0,
            a: ptr::null_mut(),
            b: ptr::null_mut(),
            object: ptr::null_mut(),
            replacement: ptr::null_mut(),
            owner: ptr::null_mut(),
            species: 0,
            before: BTreeSet::new(),
            array: ptr::null_mut(),
            saved_hook: 0,
            crew: 0,
            wait: 0,
            phoenix: None,
            star: 0,
            velocity: crate::battle::velocity::VelocityDesc::new(),
            captain: ptr::null_mut(),
        }
    }

    fn start(&mut self, world: &mut World<'_>, species: [u32; 2]) -> Result<(), String> {
        if world.activity & IN_BATTLE as u16 == 0 {
            return Err("parity melee fixture requires a running production battle".into());
        }
        [self.a, self.b] = world.fighters()?;
        for (handle, species) in [(self.a, species[0]), (self.b, species[1])] {
            let ship = world.ship_mut(handle);
            if ship.state.SpeciesID != species {
                return Err(format!(
                    "{}: live opponents do not match fixture roster",
                    self.proof.fixture.name()
                ));
            }
            ship.state.ship_input_state = 0;
        }
        Ok(())
    }

    fn advance(
        &mut self,
        world: &mut World<'_>,
        hooks: &Hooks,
    ) -> Result<Option<Vec<Effect>>, String> {
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

    fn prepare_weapon(
        &mut self,
        world: &mut World<'_>,
        handle: HElement,
    ) -> Result<Effect, String> {
        self.before = world.elements.keys().copied().collect();
        let crew = world.elements[&handle].crew_or_hp;
        let ship = world.ship_mut(handle);
        let info = ship
            .info
            .as_ref()
            .ok_or("fixture shooter descriptor is unavailable")?;
        assert!(info.crew_level > 0, "fixture shooter must be alive");
        assert_eq!(crew, info.crew_level, "fixture shooter crew union");
        ship.state.cur_status_flags =
            (ship.state.cur_status_flags & !(SPECIAL as u16)) | WEAPON as u16;
        ship.state.weapon_counter = 0;
        Ok(Effect::Charge(
            handle,
            i16::from(info.max_energy) - i16::from(info.energy_level),
        ))
    }

    fn weapon(&mut self, world: &mut World<'_>, shooter: HElement) -> Result<HElement, String> {
        world.ship_mut(shooter).state.cur_status_flags &= !(WEAPON as u16);
        let parent = world.elements[&shooter].p_parent;
        world
            .order
            .iter()
            .copied()
            .find(|h| !self.before.contains(h) && world.elements[h].p_parent == parent)
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
        world: &mut World<'_>,
        hooks: &Hooks,
        catalog: &BTreeMap<u8, u32>,
    ) -> Result<Option<Vec<Effect>>, String> {
        match self {
            Self::Battle(run) => {
                if run.a.is_null() {
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

struct Collision {
    callback: crate::battle::element::ElementCollisionFunc,
    points: [Point; 2],
    parent: usize,
}

fn collision(a: &Element, b: &Element) -> Collision {
    let point = |e: &Element| {
        Point::new(
            e.next.location.x >> ONE_SHIFT,
            e.next.location.y >> ONE_SHIFT,
        )
    };
    Collision {
        callback: a.collision_func,
        points: [point(a), point(b)],
        parent: a.p_parent as usize,
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
        task.reply(production::Reply::Captain(ptr::null_mut()))
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
        element.life_span = 31;
        element.crew_or_hp = 7;
        element.set_appearing();
        element.velocity.set_components(100, -90);
        place(&mut element, 4000, 4160);
        assert_eq!(element.current.location, Point::new(4000, 4160));
        assert_eq!(element.next.location, element.current.location);
        assert!(element.velocity.is_zero());
        assert!(element.state_flags.contains(ElementFlags::CHANGING));
        assert!(!arrived(&element));
        element.clear_appearing();
        assert!(arrived(&element));
        assert_eq!((element.life_span, element.crew_or_hp), (31, 7));
    }
}

#[cfg(test)]
#[path = "phase_tests.rs"]
mod phase_tests;
