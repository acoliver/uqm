//! Phase contracts use real borrowed records; engine execution is integration-tested.
use super::*;

fn records() -> ([Element; 2], [STARSHIP; 2], [race_desc; 2]) {
    // SAFETY: these ABI ship records contain only integers, pointers and optional callbacks.
    let states = unsafe { mem::zeroed() };
    let mut races = [race_desc::EMPTY; 2];
    for race in &mut races {
        race.ship_info.crew_level = 10;
        race.ship_info.max_energy = 100;
        race.ship_info.energy_level = 50;
    }
    ([Element::new(); 2], states, races)
}

fn world<'a>(
    elements: &'a mut [Element; 2],
    states: &'a mut [STARSHIP; 2],
    races: &'a mut [race_desc; 2],
) -> World<'a> {
    let mut world = World::new((SUPER_MELEE | IN_BATTLE) as u16, [2; 2], 0);
    for (side, ((element, state), race)) in elements.iter_mut().zip(states).zip(races).enumerate() {
        let handle = ptr::from_mut(element);
        let owner = ptr::from_mut(state).cast();
        element.p_parent = owner;
        element.player_nr = side as i16;
        element.state_flags = ElementFlags::PLAYER_SHIP;
        element.life_span = 1;
        element.crew_or_hp = 10;
        state.hShip = handle.cast();
        state.playerNr = side as i16;
        world.insert_element(element);
        let mut ship = Ship {
            state,
            info: None,
            data: None,
            characteristics: None,
            preprocess: None,
            postprocess: None,
            intelligence: None,
            phoenix: None,
        };
        ship.descriptor(
            &mut race.ship_info,
            &mut race.ship_data,
            &mut race.characteristics,
            (None, None, None),
        );
        world.ships.insert(owner, ship);
    }
    world
}

fn run(world: &World<'_>, name: FixtureName) -> Battle {
    let mut run = Battle::new(name);
    [run.a, run.b] = world.fighters().unwrap();
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
        matches!(effects.as_slice(), [Effect::Charge(a, 50), Effect::Crew(b, -9), Effect::Weapon(shooter)] if *a == run.a && *b == run.b && *shooter == run.a)
    );
    assert_eq!(world.ship(run.a).state.weapon_counter, 0);
    assert_ne!(world.ship(run.a).state.cur_status_flags & WEAPON as u16, 0);
    run.stage = 2;
    run.object = run.a;
    let effects = frames::advance(&mut run, &mut world, &super::tests::test_hooks())
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Damage(b, 10), Effect::Invoke(callback, owner)] if *b == run.b && *owner == run.b && callback.is_none())
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
    run.object = run.a;
    run.owner = world.elements[&run.b].p_parent;
    world.ship_mut(run.b).info.as_mut().unwrap().crew_level = 0;
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::RetainDescriptor(owner), Effect::Remove(handles)] if *owner == run.owner && handles == &[run.b])
    );
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(matches!(effects.as_slice(), [Effect::Spawn(owner)] if *owner == run.owner));
    run.stage = 9;
    run.array = ptr::from_mut(&mut run.captain).cast();
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(matches!(
        effects.as_slice(),
        [Effect::FreeRetained, Effect::Scale(3)]
    ));
    run.replacement = run.b;
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Intersect(a), Effect::Intersect(b), Effect::Redraw(true), Effect::ZoomProof(views, element, 0, 0)] if *a == run.a && *b == run.b && *element == run.a && views == &[ptr::null_mut(); 3])
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
    run.object = run.b;
    run.replacement = run.b;
    let hooks = super::tests::test_hooks();
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Intelligence(callback, a, concerns)] if callback.is_none() && *a == run.a && concerns[ENEMY_WEAPON_INDEX as usize].ObjectPtr == run.object.cast() && concerns[ENEMY_WEAPON_INDEX as usize].which_turn == 1)
    );
    let effects = frames::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Collide(weapon, target)] if *weapon == run.object && *target == run.a)
    );
    place(world.elements.get_mut(&run.a).unwrap(), 1000, 2000);
    let metadata = collision(world.elements[&run.a], world.elements[&run.b]);
    assert!(metadata.callback.is_none());
    assert_eq!(
        metadata.points[0],
        Point::new(1000 >> ONE_SHIFT, 2000 >> ONE_SHIFT)
    );
    assert_eq!(metadata.parent, world.elements[&run.a].p_parent as usize);
}

#[test]
fn phoenix_handoff_carries_the_actual_handle_and_saved_death_callback() {
    let (mut elements, mut states, mut races) = records();
    let mut world = world(&mut elements, &mut states, &mut races);
    let mut run = run(&world, FixtureName::PkunkRebirth);
    let hooks = super::tests::test_hooks();
    run.stage = 1;
    let phoenix = run.b;
    world.ship_mut(run.a).phoenix = Some(Phoenix {
        handle: phoenix,
        hooks: [0; 3],
    });
    let effects = rebirth::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::PhoenixFirst(h), Effect::Damage(a, 10)] if *h == phoenix && *a == run.a)
    );
    run.stage = 2;
    world.elements.get_mut(&run.a).unwrap().life_span = 0;
    let effects = rebirth::advance(&mut run, &mut world, &hooks)
        .unwrap()
        .unwrap();
    assert!(matches!(effects.as_slice(), [Effect::Death(h)] if *h == run.a));
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
        world.ship_mut(*element).state.hShip = element.cast();
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
        .ship(run.a)
        .data
        .as_ref()
        .unwrap()
        .captain_control
        .special;
    let effects = combat::mutual_loss(&mut run, &mut world).unwrap().unwrap();
    assert!(
        matches!(effects.as_slice(), [Effect::Intersect(a), Effect::Intersect(b), Effect::Captain(frame)]
        if *a == run.a && *b == run.b && *frame == captain)
    );
    assert_ne!(world.ship(run.a).state.cur_status_flags & SPECIAL as u16, 0);
    assert_eq!(
        world.elements[&run.a].current.location,
        Point::new(4000, 4000)
    );
    assert_eq!(
        world.elements[&run.b].current.location,
        Point::new(4160, 4000)
    );
}
