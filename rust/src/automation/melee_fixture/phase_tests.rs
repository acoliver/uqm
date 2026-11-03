//! Phase contracts use real borrowed records; engine execution is integration-tested.
use super::*;
fn frame_views(views: Frames) -> [FRAME; 3] {
    std::array::from_fn(|index| views.view(index).raw())
}

fn records() -> ([Element; 2], [ShipRecord; 2], [RaceDescriptor; 2]) {
    // SAFETY: these ABI ship records contain only integers, pointers and optional callbacks.
    let states = unsafe { mem::zeroed() };
    let mut races = [RaceDescriptor::EMPTY; 2];
    for race in &mut races {
        race.ship_info.crew_level = 10;
        race.ship_info.max_energy = 100;
        race.ship_info.energy_level = 50;
    }
    (std::array::from_fn(|_| Element::new()), states, races)
}

fn world(
    elements: &mut [Element; 2],
    states: &mut [ShipRecord; 2],
    races: &mut [RaceDescriptor; 2],
) -> World {
    let mut world = World::new((SUPER_MELEE | IN_BATTLE) as u16, [2; 2], 0);
    for (side, ((element, state), race)) in elements.iter_mut().zip(states).zip(races).enumerate() {
        let id = ElementId::fixture(side as u8, 0);
        let owner = crate::engine::battle::ShipId::fixture(side as u8, 0, 0);
        element.parent = Parent::Ship(owner);
        element.player = side as i16;
        element.flags = ElementFlags::PLAYER_SHIP.bits();
        element.life = 1;
        element.hits = 10;
        state.playerNr = side as i16;
        world.order.push(id);
        world.elements.insert(id, element.clone());
        world.ship_order.push(owner);
        world.ships.insert(owner, Ship::for_test(state, race, id));
    }
    world
}

fn run(world: &World, name: FixtureName) -> Battle {
    let mut run = Battle::new(name);
    [run.a, run.b] = world.fighters().unwrap().map(Some);
    run
}

#[test]
fn lethal_collision_setup_charges_a_live_weapon_and_reduces_only_the_enemy_crew() {
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::DeathSpawn);
    let effects = combat::death_spawn(&mut run, &mut world, &super::tests::test_hooks())
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Charge(a, 50), Effect::Crew(b, -9), Effect::Weapon(shooter)] if *a == run.a() && *b == run.b() && *shooter == run.a())
    );
    assert_eq!(world.ship(run.a()).state.weapon_counter, 0);
    assert_ne!(world.ship(run.a()).state.status & WEAPON as u16, 0);
    run.stage = 2;
    run.object = Some(run.a());
    let effects = frames::advance(&mut run, &mut world, &super::tests::test_hooks())
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Damage(b, 10), Effect::Invoke(callback, owner)] if *b == run.b() && *owner == run.b() && callback.is_none())
    );
}

#[test]
fn pool_cleanup_and_recovery_keep_the_original_handles_in_order() {
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::PoolFull);
    let hooks = super::tests::test_hooks();
    let effects = combat::pool(&mut run, &mut world, &hooks).unwrap().unwrap();
    assert!(matches!(effects.as_slice(), [Effect::Remove(handles)] if handles.is_empty()));
    run.stage = 3;
    run.before = world.order.iter().copied().collect();
    let effects = combat::pool(&mut run, &mut world, &hooks).unwrap().unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Remove(handles), Effect::ProbeRecovery] if handles.is_empty())
    );
    run.stage = 4;
    let effects = combat::pool(&mut run, &mut world, &hooks).unwrap().unwrap();
    assert!(matches!(effects.as_slice(), [Effect::Redraw(true)]));
}

#[test]
fn outgoing_descriptor_is_retained_until_frame_users_retire_and_zoom_restores_scale() {
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::FrameSets);
    let hooks = super::tests::test_hooks();
    run.stage = 3;
    run.object = Some(run.a());
    run.owner = Some(world.owner(run.b()));
    world.ship_mut(run.b()).info.as_mut().unwrap().crew_level = 0;
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::RetainDescriptor(owner), Effect::Remove(handles)] if *owner == run.owner() && handles == &[run.b()])
    );
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(matches!(effects.as_slice(), [Effect::Spawn(owner)] if *owner == run.owner()));
    run.stage = 9;
    run.array = Some(Frames::shared(
        crate::engine::battle::frames::SharedFrames::Fixture(0),
        crate::engine::battle::frames::FrameArray::fixture([std::ptr::null_mut(); 3], ()),
    ));
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [Effect::FreeRetained, Effect::Scale(3)]
    ));
    run.replacement = Some(run.b());
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Intersect(a), Effect::Intersect(b), Effect::Redraw(true), Effect::ZoomProof(views, element, 0, 0)] if *a == run.a() && *b == run.b() && *element == run.a() && frame_views(views.clone()) == [ptr::null_mut(); 3])
    );
    run.wait = 3;
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [Effect::RestoreScale, Effect::Redraw(true)]
    ));
}

#[test]
fn arilou_threat_and_collision_operands_are_snapshots_of_the_real_borrowed_records() {
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::FrameSets);
    run.stage = 6;
    run.object = Some(run.b());
    run.replacement = Some(run.b());
    run.array = Some(world.elements[&run.object()].current.image.frames.clone());
    let hooks = super::tests::test_hooks();
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Intelligence(a, concerns)] if concerns.len() == 4 && *a == run.a() && concerns[ENEMY_WEAPON_INDEX as usize].object == Some(run.object()) && concerns[ENEMY_WEAPON_INDEX as usize].which_turn == 1)
    );
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Collide(weapon, target)] if *weapon == run.object() && *target == run.a())
    );
    place(world.elements.get_mut(&run.a()).unwrap(), 1000, 2000);
    let metadata = collision(&world.elements[&run.a()], &world.elements[&run.b()]);
    assert!(metadata.callback.is_none());
    assert_eq!(
        metadata.points[0],
        Point {
            x: 1000 >> ONE_SHIFT,
            y: 2000 >> ONE_SHIFT
        }
    );
    assert_eq!(metadata.parent, world.elements[&run.a()].parent);
}

#[test]
fn phoenix_install_invokes_the_native_preprocess_on_its_live_ship() {
    fn preprocess(_: &mut crate::engine::battle::Cx<'_>, _: crate::engine::battle::ShipId) {}
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::PkunkRebirth);
    let mut hooks = super::tests::test_hooks();
    hooks.intercept = 1;
    hooks.pkunk = Some(preprocess);
    let effects = rebirth::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::NativeShip(callback, ship)]
        if std::ptr::fn_addr_eq(*callback, preprocess as crate::engine::battle::race::ShipFn) && *ship == run.a())
    );
    assert!(world.elements[&run.a()].flags & ElementFlags::APPEARING.bits() != 0);
}

#[test]
fn phoenix_handoff_carries_the_actual_handle_and_saved_death_callback() {
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::PkunkRebirth);
    let hooks = super::tests::test_hooks();
    run.stage = 1;
    let phoenix = run.b();
    world.ship_mut(run.a()).phoenix = Some(Phoenix {
        handle: Some(phoenix),
        hooks: [0; 3],
    });
    let effects = rebirth::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::PhoenixFirst(h), Effect::Damage(a, 10)] if *h == phoenix && *a == run.a())
    );
    run.stage = 2;
    world.elements.get_mut(&run.a()).unwrap().life = 0;
    let effects = rebirth::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(matches!(effects.as_slice(), [Effect::Death(h)] if *h == run.a()));
}

#[test]
fn cyborg_effect_uses_both_live_spawned_elements_and_descriptor_crew() {
    let (mut elements, mut states, mut races) = records();
    states[1].index = 1;
    let mut world = world(&mut elements, &mut states, &mut races);
    let handles = world.order.clone();
    let catalog = BTreeMap::from([(0, 1), (1, 2)]);
    let mut run = corpus::Run::new(CorpusRequest { fleet: 0 }, vec![]).unwrap();
    assert!(matches!(
        run.advance(&mut world, &catalog)
            .unwrap()
            .unwrap()
            .as_slice(),
        [Effect::Reset]
    ));
    let effects = run.advance(&mut world, &catalog).unwrap().unwrap();
    assert_eq!(effects.len(), 2);
    for (side, element) in handles.iter().enumerate() {
        world.ship_mut(*element).element = Some(*element);
        world.ship_mut(*element).info.as_mut().unwrap().crew_level = 7 + side as u16;
    }
    run.advance(&mut world, &catalog).unwrap();
    let effects = run.advance(&mut world, &catalog).unwrap().unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Think(active), Effect::Redraw(false)]
        if *active == [(handles[0], 7), (handles[1], 8)])
    );
}

#[test]
fn mutual_loss_captain_effect_comes_from_the_live_shofixti_descriptor() {
    let (mut elements, mut states, mut races) = records();
    states[0].SpeciesID = SPECIES_ID_SHOFIXTI_ID;
    races[1].ship_info.crew_level = 6;
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::MixedCollision);
    let captain = world
        .ship(run.a())
        .data
        .as_ref()
        .unwrap()
        .captain_special
        .clone();
    let effects = combat::mutual_loss(&mut run, &mut world).unwrap().unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Intersect(a), Effect::Intersect(b), Effect::Captain(frame)]
        if *a == run.a() && *b == run.b() && *frame == captain)
    );
    assert_ne!(world.ship(run.a()).state.status & SPECIAL as u16, 0);
    assert_eq!(
        world.elements[&run.a()].current.location,
        Point { x: 4000, y: 4000 }
    );
    assert_eq!(
        world.elements[&run.b()].current.location,
        Point { x: 4160, y: 4000 }
    );
}

#[test]
fn blazer_setup_dispatches_pre_then_post_for_the_same_ship() {
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::AndrosynthBlazer);
    run.stage = 1;
    let effects = combat::blazer(&mut run, &mut world, &super::tests::test_hooks())
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(),[Effect::RaceProcess(false,a),Effect::RaceProcess(true,b)] if *a==run.a() && *b==run.a())
    );
    assert_eq!(world.ship(run.a()).info.as_ref().unwrap().energy_level, 100);
    assert_ne!(world.ship(run.a()).state.status & SPECIAL as u16, 0);
}

fn special_records() -> ([Element; 2], [ShipRecord; 2], [RaceDescriptor; 2]) {
    let (elements, states, mut races) = records();
    races[0].characteristics.special_wait = 30;
    races[0].characteristics.special_energy_cost = 16;
    (elements, states, races)
}

fn neutral_junk(world: &mut World, mass: u8) -> ElementId {
    let id = ElementId::fixture(2, 0);
    let mut junk = Element::new();
    junk.player = crate::ships::native::abi::NEUTRAL_PLAYER_NUM as i16;
    junk.mass = mass;
    junk.life = 1;
    world.order.push(id);
    world.elements.insert(id, junk);
    id
}

fn failure(result: Result<Option<Vec<Effect>>, String>) -> String {
    result.err().expect("the fixture stage should fail")
}

type Stage = fn(&mut Battle, &mut World) -> Result<Option<Vec<Effect>>, String>;

fn drive(stage: Stage, run: &mut Battle, world: &mut World) {
    while stage(run, world).unwrap().is_some() {}
}

#[test]
fn druuge_furnace_holds_special_until_the_race_arms_then_checks_the_burn() {
    let (mut elements, mut states, mut races) = special_records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::DruugeFurnace);
    let (a, b) = (run.a(), run.b());
    specials::furnace(&mut run, &mut world).unwrap();
    assert_eq!(
        world.ship(a).state.control,
        crate::battle::ai::HUMAN_CONTROL
    );
    assert_eq!(world.ship(b).state.input, 0);
    assert_eq!(world.ship(a).info.as_ref().unwrap().energy_level, 0);
    assert!(specials::furnace(&mut run, &mut world).unwrap().is_some());
    assert_eq!(world.ship(a).state.input, SPECIAL as u8);
    world.ship_mut(a).state.special_counter = 29;
    world.ship_mut(a).info.as_mut().unwrap().crew_level = 9;
    world.ship_mut(a).info.as_mut().unwrap().energy_level = 16;
    specials::furnace(&mut run, &mut world).unwrap();
    assert_eq!(world.ship(a).state.input, 0);
    assert_eq!(run.stage, 2);
    drive(specials::furnace, &mut run, &mut world);
    run.proof.verify().unwrap();
}

#[test]
fn druuge_furnace_fails_when_the_crew_is_not_burned() {
    let (mut elements, mut states, mut races) = special_records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::DruugeFurnace);
    let a = run.a();
    specials::furnace(&mut run, &mut world).unwrap();
    world.ship_mut(a).state.special_counter = 29;
    world.ship_mut(a).info.as_mut().unwrap().energy_level = 16;
    let error = failure(specials::furnace(&mut run, &mut world));
    assert!(error.contains("crew-burned"), "{error}");
}

#[test]
fn druuge_furnace_fails_when_the_held_special_never_arms_the_race() {
    let (mut elements, mut states, mut races) = special_records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::DruugeFurnace);
    specials::furnace(&mut run, &mut world).unwrap();
    for _ in 0..specials::HOLD_FRAMES {
        specials::furnace(&mut run, &mut world).unwrap();
    }
    let error = failure(specials::furnace(&mut run, &mut world));
    assert!(error.contains("special did not fire"), "{error}");
}

#[test]
fn druuge_furnace_needs_a_crew_to_burn() {
    let (mut elements, mut states, mut races) = special_records();
    races[0].ship_info.crew_level = 1;
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::DruugeFurnace);
    let error = failure(specials::furnace(&mut run, &mut world));
    assert!(error.contains("at least two crew"), "{error}");
}

#[test]
fn slylandro_harvest_places_junk_in_range_holds_special_then_checks_the_harvest() {
    let (mut elements, mut states, mut races) = special_records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let junk = neutral_junk(&mut world, 3);
    let mut run = run(&world, FixtureName::SlylandroHarvest);
    let (a, b) = (run.a(), run.b());
    specials::harvest(&mut run, &mut world).unwrap();
    assert_eq!(run.object(), junk);
    let ship_x = i32::from(world.elements[&a].current.location.x);
    assert_eq!(
        i32::from(world.elements[&junk].current.location.x) - ship_x,
        specials::JUNK_OFFSET << ONE_SHIFT
    );
    assert_eq!(world.ship(a).info.as_ref().unwrap().energy_level, 0);
    assert_eq!(world.ship(b).state.input, 0);
    assert!(specials::harvest(&mut run, &mut world).unwrap().is_some());
    assert_eq!(world.ship(a).state.input, SPECIAL as u8);
    world.ship_mut(a).state.special_counter = 29;
    world.ship_mut(a).info.as_mut().unwrap().energy_level = 100;
    world.elements.get_mut(&junk).unwrap().life = 0;
    specials::harvest(&mut run, &mut world).unwrap();
    assert_eq!(run.stage, 2);
    drive(specials::harvest, &mut run, &mut world);
    run.proof.verify().unwrap();
}

#[test]
fn slylandro_harvest_has_no_junk_to_place_without_arrived_neutral_junk_in_mass_range() {
    let (mut elements, mut states, mut races) = special_records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::SlylandroHarvest);
    let error = failure(specials::harvest(&mut run, &mut world));
    assert!(error.contains("neutral space junk"), "{error}");
    neutral_junk(&mut world, 200);
    let error = failure(specials::harvest(&mut run, &mut world));
    assert!(error.contains("neutral space junk"), "{error}");
}

#[test]
fn slylandro_harvest_fails_when_the_junk_survives_the_held_special() {
    let (mut elements, mut states, mut races) = special_records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let junk = neutral_junk(&mut world, 3);
    let mut run = run(&world, FixtureName::SlylandroHarvest);
    let a = run.a();
    specials::harvest(&mut run, &mut world).unwrap();
    world.ship_mut(a).state.special_counter = 29;
    world.ship_mut(a).info.as_mut().unwrap().energy_level = 100;
    let error = failure(specials::harvest(&mut run, &mut world));
    assert!(error.contains("junk-harvested"), "{error}");
    assert_eq!(world.elements[&junk].life, 1);
}
