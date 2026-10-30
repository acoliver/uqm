//! The synchronous fixture driver commits plans before callbacks and resource retirement.

use super::*;

fn zoom_matches(
    view: usize,
    zoom: i16,
    selected: (usize, u16),
    expected: (usize, u16),
    mipmap: Option<(usize, usize)>,
) -> bool {
    selected == expected
        && selected.0 != 0
        && zoom == [256, 700, 1024][view]
        && (view != 1 || mipmap.is_some_and(|(actual, adjacent)| actual != 0 && actual == adjacent))
}

#[cfg(not(test))]
fn hooks() -> Hooks {
    use crate::ships::races::{
        androsynth_native, chmmr_native, pkunk_native, urquan_native, vux_native,
    };
    Hooks {
        blazer_collision: androsynth_native::engine::blazer_collision as *const () as usize,
        satellite: Some(crate::engine::battle::ElementHook::Native(
            chmmr_native::engine::satellite_postprocess,
        )),
        defense: chmmr_native::engine::spawn_point_defense as *const () as usize,
        limpets: Some(crate::engine::battle::ElementHook::Native(
            vux_native::engine::spawn_limpets,
        )),
        fighters: Some(crate::engine::battle::ElementHook::Native(
            urquan_native::engine::spawn_fighters,
        )),
        fighter_preprocess: urquan_native::engine::fighter_preprocess as *const () as usize,
        pkunk: Some(pkunk_native::engine::pkunk_preprocess),
        intercept: pkunk_native::engine::intercept_pkunk_death as *const () as usize,
        rebirth: pkunk_native::engine::new_pkunk as *const () as usize,
        ship_death: Some(crate::engine::battle::ElementHook::Native(
            crate::battle::tactrans_native::ship_death,
        )),
        star_collision: crate::navigation::hyper_transition::star_collision as *const () as usize,
        unhyper: crate::navigation::hyper_transition::unhyper as *const () as usize,
    }
}

#[cfg(not(test))]
pub(in crate::automation::melee_fixture) fn execute(
    operation: Operation<'_>,
) -> Result<Option<FixtureRun>, String> {
    let hooks = hooks();
    let mut registry = crate::ships::hook_registry::hooks();
    registry.extend(crate::automation::parity_hooks::hooks());
    // SAFETY: automation admits operations on the game thread after initialization.
    let (activity, position) = unsafe {
        (
            GlobData.Game_state.CurrentActivity,
            SavedPosition {
                slot: 0,
                log_x: GlobData.SIS_state.log_x,
                log_y: GlobData.SIS_state.log_y,
                star: crate::planets::starmap::CurStarDescPtr
                    .as_ref()
                    .map(|s| (s.star_pt.x, s.star_pt.y)),
            },
        )
    };
    let task = match admit(operation, activity, position, registry)? {
        Admission::Configure(request) => return configure(request).map(|_| None),
        Admission::Complete(proof) => return Ok(proof),
        Admission::Task(task) => task,
    };
    // SAFETY: the catalog was loaded before this admitted fixture action.
    let catalog = unsafe {
        task.needed_species()
            .into_iter()
            .map(|index| {
                let master = crate::battle::build_native::engine::GetStarShipFromIndex(
                    &raw mut master_q,
                    u16::from(index),
                )
                .cast::<MASTER_SHIP_INFO>();
                assert!(!master.is_null(), "loaded fixture catalog");
                (index, (*master).SpeciesID)
            })
            .collect()
    };
    // SAFETY: this admitted operation owns the game thread; legacy callbacks republish Cx.
    unsafe { crate::app::bridge::enter(|cx| drive(cx, task, &hooks, &catalog)) }
}

#[cfg(not(test))]
fn drive(
    cx: &mut Cx<'_>,
    mut task: Task<'_>,
    hooks: &Hooks,
    catalog: &BTreeMap<u8, u32>,
) -> Result<Option<FixtureRun>, String> {
    let mut retained: Option<crate::battle::context_access::fixture::Descriptor> = None;
    let mut old_scale = 0;
    loop {
        let mut world = borrow_world(cx);
        // SAFETY: planning owns only copied values; its legacy checkpoint may enter Cx.
        let effects =
            unsafe { crate::app::bridge::publish(cx, || task.advance(&mut world, hooks, catalog)) };
        world.commit(cx);
        let elements = world.order;
        let ships = world.ship_order;
        let Some(effects) = effects? else {
            break;
        };
        for effect in effects {
            // SAFETY: planning values are committed and no record borrow crosses any effect.
            let reply = unsafe {
                match effect {
                    Effect::RetainDescriptor(owner) => {
                        retained = Some(cx.descriptor_handle(owner));
                        Reply::None
                    }
                    Effect::FreeRetained => {
                        crate::mainloop::input_frame::drain_committed_draws();
                        retained
                            .take()
                            .unwrap_or_else(|| panic!("retained fixture descriptor"))
                            .free();
                        Reply::None
                    }
                    Effect::Reset => {
                        crate::mainloop::input_frame::drain_committed_draws();
                        crate::battle::tactrans_native::ResetWinnerStarShip();
                        cx.b.counters = [7, 7];
                        apply(cx, Effect::Remove(elements.clone()))?;
                        for &owner in &ships {
                            cx.detach_descriptor(owner).free();
                        }
                        Reply::None
                    }
                    Effect::Scale(value) => {
                        old_scale = ptr::replace(&raw mut optMeleeScale, value);
                        Reply::None
                    }
                    Effect::RestoreScale => {
                        optMeleeScale = old_scale;
                        Reply::None
                    }
                    other => apply(cx, other)?,
                }
            };
            task.reply(reply)?;
        }
    }
    assert!(retained.is_none(), "outgoing fixture descriptor retired");
    task.finish()
}

#[cfg(not(test))]
fn borrow_world(cx: &Cx<'_>) -> World {
    let mut world = World::new(
        cx.world.activity(),
        cx.b.counters,
        // SAFETY: the catalog selection is read on its owning game thread.
        unsafe { crate::planets::starmap::CurStarDescPtr as usize },
    );
    let mut next = cx.elements().head();
    while let Some(id) = next {
        world.order.push(id);
        world.elements.insert(id, cx.fixture_element(id));
        next = cx.elements().succ(id);
    }
    for side in 0..2 {
        for id in cx.queued_ships(side) {
            world.ship_order.push(id);
            world.ships.insert(id, cx.fixture_ship(id));
        }
    }
    world
}

pub(super) enum Reply {
    None,
    Loaded(bool),
    Check(&'static str, bool),
    Captain(Frame),
}

/// # Safety
/// Handles and callback addresses must have come from the just-dropped live
/// graph. No borrowed Element, ship, descriptor or frame may cross this call.
#[cfg(not(test))]
unsafe fn apply(cx: &mut Cx<'_>, effect: Effect) -> Result<Reply, String> {
    // SAFETY: effects execute at the published game-thread boundary after command planning commits.
    unsafe {
        match effect {
            Effect::Redraw(check) => {
                crate::app::bridge::publish(cx, || {
                    crate::battle::process_native::RedrawQueue(0);
                    redrawn(check);
                });
                return Ok(Reply::None);
            }
            Effect::ClearStar => {
                crate::planets::starmap::CurStarDescPtr = ptr::null_mut();
                return Ok(Reply::None);
            }
            Effect::Remove(_) | Effect::ZoomProof(..) => {
                crate::mainloop::input_frame::drain_committed_draws()
            }
            _ => {}
        }
        {
            use crate::battle::process_native::{
                invoke_collision::invoke_collision, invoke_element::invoke_element,
            };
            match effect {
                Effect::Intersect(id) => cx.init_intersect(id),
                Effect::Charge(id, delta) => {
                    assert!(cx.delta_energy(id, delta), "fixture charged weapon setup")
                }
                Effect::Damage(id, delta) => cx.fixture_damage(id, delta),
                Effect::Crew(id, delta) => {
                    cx.delta_crew(id, delta);
                }
                Effect::Remove(ids) => {
                    for id in ids {
                        cx.unlink(id);
                        cx.dispose(id);
                    }
                }
                Effect::Spawn(ship) => return Ok(Reply::Loaded(cx.fixture_spawn(ship))),
                Effect::PhoenixFirst(id) => {
                    cx.unlink(id);
                    let head = cx.elements().head();
                    cx.insert_before(id, head);
                }
                Effect::ProbePool => {
                    return Ok(Reply::Check(
                        "allocation-failed",
                        cx.alloc_element().is_none(),
                    ))
                }
                Effect::ProbeRecovery => {
                    let id = cx.alloc_element();
                    if let Some(id) = id {
                        cx.dispose(id);
                    }
                    return Ok(Reply::Check("pool-recovered", id.is_some()));
                }
                Effect::ZoomProof(frames, id, view, index) => {
                    let (selected, expected, mipmap) = cx.zoom_sample(frames, id, view, index);
                    return Ok(Reply::Check(
                        ["trilinear-large", "trilinear-medium", "trilinear-small"][view],
                        zoom_matches(view, cx.b.zoom_out, selected, expected, mipmap),
                    ));
                }
                Effect::Captain(frame) => {
                    return Ok(Reply::Captain(
                        cx.absolute_frame(frame, cx.frame_count(frame) - 4),
                    ))
                }
                Effect::RaceProcess(post, id) => {
                    let ship = cx.owner(id);
                    cx.invoke_ship_process(ship, post);
                }
                Effect::NativeShip(callback, id) => {
                    let ship = cx.owner(id);
                    let parent = cx.ship_address(ship);
                    crate::battle::process_native::dispatch::invoke(
                        callback as usize,
                        parent,
                        || callback(cx, ship),
                    );
                }
                Effect::Weapon(id) => crate::battle::ship_native::ship_postprocess(cx, id),
                Effect::Death(id) => {
                    let hook = cx
                        .element(id)
                        .death
                        .ok_or("fixture ship has no production death callback")?;
                    invoke_element(cx, hook, id);
                }
                Effect::Invoke(hook, id) => invoke_element(
                    cx,
                    hook.ok_or("fixture is missing its production callback")?,
                    id,
                ),
                Effect::Collide(a, b) => {
                    let hook = cx
                        .element(a)
                        .collide
                        .ok_or("fixture projectile has no collision callback")?;
                    let point = |p: Point| Point {
                        x: p.x >> ONE_SHIFT,
                        y: p.y >> ONE_SHIFT,
                    };
                    let mut pa = point(cx.element(a).next.location);
                    let mut pb = point(cx.element(b).next.location);
                    invoke_collision(cx, hook, a, &mut pa, b, &mut pb);
                }
                Effect::Think(active) => {
                    for (id, crew) in active {
                        let ship = cx.owner(id);
                        cx.fixture_think(ship, crew);
                    }
                }
                Effect::Intelligence(id, concerns) => {
                    cx.fixture_intelligence(id, concerns);
                }
                _ => unreachable!(),
            }
            Ok(Reply::None)
        }
    }
}

#[cfg(not(test))]
fn configure(request: &MeleeSetupRequest) -> Result<(), String> {
    use crate::supermelee::native::menu::live::drawing::P_MELEE_STATE;
    let controls = &raw mut crate::supermelee::c_bridge::PlayerControl;
    let ships = request.ships.iter().enumerate().flat_map(|(side, fleet)| {
        fleet
            .iter()
            .copied()
            .chain([255; 7])
            .enumerate()
            .map(move |(slot, ship)| (1 - side, slot as u16, u32::from(ship)))
    });
    // SAFETY: execute is the admitted live menu input boundary.
    unsafe {
        let state = P_MELEE_STATE
            .as_mut()
            .ok_or("configure_parity_melee requires the live Super Melee menu")?;
        configure_menu(
            state.melee_started != 0,
            &mut *controls,
            GlobData.Game_state.CurrentActivity,
            request.cyborg,
        )?;
        state.melee_option = 4;
        state.input_func = Some(crate::supermelee::native::menu::live::DoMelee);
        for (side, slot, ship) in ships {
            crate::supermelee::native::teams::MeleeSetup_setShip(
                state.melee_setup,
                side,
                slot,
                ship,
            );
        }
    }
    Ok(())
}

fn configure_menu(
    started: bool,
    controls: &mut [u8; 2],
    activity: u16,
    cyborg: bool,
) -> Result<(), String> {
    if activity & IN_BATTLE as u16 != 0 || started {
        return Err("cannot replace a fleet during battle".into());
    }
    controls.fill(if cyborg { 2 | 64 } else { 1 | 16 });
    Ok(())
}

pub(super) fn loaded(reply: Reply) -> Result<(), String> {
    match reply {
        Reply::None | Reply::Loaded(true) => Ok(()),
        Reply::Loaded(false) => Err("production fixture ship load failed".into()),
        _ => panic!("unexpected production corpus reply"),
    }
}

#[cfg(not(test))]
fn redrawn(check: bool) {
    crate::collections::observation::advance();
    if check {
        checkpoint();
    }
}

#[cfg(test)]
mod zoom_tests {
    use super::zoom_matches;

    #[test]
    fn menu_configuration_validates_before_changing_controls() {
        let mut controls = [99; 2];
        assert!(super::configure_menu(true, &mut controls, 0, true).is_err());
        assert!(
            super::configure_menu(false, &mut controls, super::IN_BATTLE as u16, true).is_err()
        );
        assert_eq!(controls, [99; 2]);
        super::configure_menu(false, &mut controls, 0, true).unwrap();
        assert_eq!(controls, [66; 2]);
        super::configure_menu(false, &mut controls, 0, false).unwrap();
        assert_eq!(controls, [17; 2]);
    }

    #[test]
    fn a_receipt_requires_all_three_zoom_checkpoints() {
        use crate::automation::melee_fixture::{FixtureName, FixtureRun};
        let mut proof = FixtureRun::new(FixtureName::FrameSets);
        proof.steps = 3;
        for check in [
            "orphan-weapon",
            "same-species-entry",
            "arilou-threat",
            "blast-teardown",
            "trilinear-large",
            "trilinear-small",
        ] {
            proof.require(true, check).unwrap();
        }
        assert!(proof.verify().is_err());
        proof.require(true, "trilinear-medium").unwrap();
        assert!(proof.verify().is_ok());
    }

    #[test]
    fn every_checkpoint_requires_its_exact_view_and_animation_index() {
        for (view, zoom) in [256, 700, 1024].into_iter().enumerate() {
            let expected = (10 + view, 5);
            let mipmap = (view == 1).then_some((99, 99));
            assert!(zoom_matches(view, zoom, expected, expected, mipmap));
            assert!(!zoom_matches(view, zoom, (expected.0, 6), expected, mipmap));
            assert!(!zoom_matches(view, zoom + 1, expected, expected, mipmap));
            if view != 0 {
                assert!(
                    !zoom_matches(view, zoom, (10, 5), expected, mipmap),
                    "constant view zero must fail"
                );
            }
        }
    }

    #[test]
    fn fractional_checkpoint_requires_the_adjacent_mipmap_binding() {
        for mipmap in [None, Some((0, 0)), Some((98, 99))] {
            assert!(!zoom_matches(1, 700, (11, 5), (11, 5), mipmap));
        }
        assert!(zoom_matches(1, 700, (11, 5), (11, 5), Some((99, 99))));
    }
}
