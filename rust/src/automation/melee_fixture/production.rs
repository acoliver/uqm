//! The synchronous boundary owns raw traversal, resource retirement and callbacks.

use super::*;
#[cfg(not(test))]
use crate::graphics::native_abi::frame_desc;

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
fn zoom_proof(
    view: usize,
    zoom: i16,
    index: u16,
    frame: &crate::graphics::native_abi::frame_desc,
    expected: &crate::graphics::native_abi::frame_desc,
    mipmap: Option<(
        &crate::graphics::native_abi::TFB_Image,
        &crate::graphics::native_abi::TFB_Image,
    )>,
) -> Reply {
    Reply::Check(
        ["trilinear-large", "trilinear-medium", "trilinear-small"][view],
        zoom_matches(
            view,
            zoom,
            (frame.parent as usize, frame.Index),
            (expected.parent as usize, index),
            mipmap.map(|(image, adjacent)| (image.MipmapImg as usize, adjacent.NormalImg as usize)),
        ),
    )
}

#[cfg(not(test))]
fn hooks() -> Hooks {
    use crate::ships::races::{
        androsynth_native, chmmr_native, pkunk_native, urquan_native, vux_native,
    };
    Hooks {
        blazer_collision: androsynth_native::engine::blazer_collision as *const () as usize,
        satellite: Some(chmmr_native::engine::satellite_postprocess),
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
    drive(task, &hooks, &catalog)
}

#[cfg(not(test))]
fn drive(
    mut task: Task<'_>,
    hooks: &Hooks,
    catalog: &BTreeMap<u8, u32>,
) -> Result<Option<FixtureRun>, String> {
    let mut retained = ptr::null_mut();
    let mut old_scale = 0;
    loop {
        // SAFETY: callbacks cannot run until this borrow is explicitly dropped.
        let mut world = unsafe { borrow_world() };
        let Some(effects) = task.advance(&mut world, hooks, catalog)? else {
            break;
        };
        let slots = world.descriptors.clone();
        let elements = world.order.clone();
        let ships = world.ship_order.clone();
        let replies = world.callbacks(effects, |effect| {
            // SAFETY: all record borrows ended; every operand came from that graph.
            unsafe {
                Ok(match effect {
                    Effect::RetainDescriptor(owner) => {
                        retained = *slots[&owner];
                        Reply::None
                    }
                    Effect::FreeRetained => {
                        crate::mainloop::input_frame::drain_committed_draws();
                        crate::ships::loader_native::free_ship(retained, 1, 1);
                        retained = ptr::null_mut();
                        Reply::None
                    }
                    Effect::Reset => {
                        crate::mainloop::input_frame::drain_committed_draws();
                        crate::battle::tactrans_native::ResetWinnerStarShip();
                        crate::app::bridge::battle(|b| b.counters = [7, 7]);
                        apply(Effect::Remove(elements.clone()))?;
                        for owner in &ships {
                            let descriptor = mem::replace(&mut *slots[owner], ptr::null_mut());
                            if !descriptor.is_null() {
                                crate::ships::loader_native::free_ship(descriptor, 1, 1);
                            }
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
                    other => return apply(other),
                })
            }
        });
        for reply in replies {
            task.reply(reply?)?;
        }
    }
    assert!(retained.is_null(), "outgoing fixture descriptor retired");
    task.finish()
}

/// # Safety
/// The initialized queues and every linked record must be exclusively available
/// on the game thread until the returned borrows are dropped.
#[cfg(not(test))]
unsafe fn borrow_world<'a>() -> World<'a> {
    let ship_record = |state| Ship {
        callback_addresses: [0; 5],
        state,
        info: None,
        data: None,
        characteristics: None,
        phoenix: None,
    };
    // SAFETY: the caller holds the synchronous game-thread boundary, with no callbacks.
    unsafe {
        let mut world = World::new(
            GlobData.Game_state.CurrentActivity,
            crate::app::bridge::battle(|b| b.counters),
            crate::planets::starmap::CurStarDescPtr as usize,
        );
        let mut element = (*crate::c_bindings::live_queues::display_list())
            .head
            .cast::<Element>();
        while !element.is_null() {
            element = world.insert_element(&mut *element);
        }
        for side in 0..2 {
            let mut owner = (*crate::c_bindings::live_queues::player_ships(side)).head;
            while !owner.is_null() {
                let next = world.insert_ship(ship_record(&mut *owner.cast()));
                let slot = world.descriptors[&owner];
                let Some(ship) = world.ships.get_mut(&owner) else {
                    panic!("projected fixture owner {owner:p} disappeared from its graph");
                };
                if let Some(race) = (*slot).as_mut() {
                    ship.callback_addresses = race.callbacks();
                    if ship.state.SpeciesID == SPECIES_ID_PKUNK_ID {
                        let crate::engine::battle::RaceState::Pkunk { phoenix: token, .. } =
                            race.race_state
                        else {
                            panic!("Pkunk fixture state missing");
                        };
                        let handle = crate::automation::parity_state::phoenix_address(token);
                        ship.phoenix = Some(phoenix(race.race_state, handle));
                    }
                    ship.descriptor(
                        &mut race.ship_info,
                        &mut race.ship_data,
                        &mut race.characteristics,
                    );
                }
                owner = next;
            }
        }
        world
    }
}

pub(super) enum Reply {
    None,
    Loaded(bool),
    Check(&'static str, bool),
    Captain(FRAME),
}

/// # Safety
/// Handles and callback addresses must have come from the just-dropped live
/// graph. No borrowed Element, ship, descriptor or frame may cross this call.
#[cfg(not(test))]
unsafe fn apply(effect: Effect) -> Result<Reply, String> {
    // SAFETY: execute drops all live graph borrows before dispatching effects.
    unsafe {
        match effect {
            Effect::Redraw(checkpoint) => {
                crate::battle::process_native::RedrawQueue(0);
                redrawn(checkpoint);
            }
            Effect::Intersect(h) => crate::ships::native::init_intersect_points(h.cast()),
            Effect::Charge(h, delta) => assert!(
                DeltaEnergy(h.cast(), delta) != 0,
                "fixture charged weapon setup"
            ),
            Effect::Damage(h, delta) => do_damage(h.cast(), delta),
            Effect::Crew(h, delta) => {
                DeltaCrew(h.cast(), delta);
            }

            Effect::Remove(handles) => {
                crate::mainloop::input_frame::drain_committed_draws();
                for h in handles {
                    crate::battle::process_native::RemoveElement(h.cast());
                    FreeElement(h.cast());
                }
            }
            Effect::Spawn(owner) => {
                return Ok(Reply::Loaded(
                    crate::battle::ship_native::spawn_ship(owner.cast()) != 0,
                ))
            }
            Effect::PhoenixFirst(h) => {
                let queue = crate::c_bindings::live_queues::display_list()
                    .cast_mut()
                    .cast();
                crate::battle::process_native::RemoveElement(h.cast());
                InsertQueue(queue, h.cast(), (*queue).head);
            }
            Effect::ProbePool => {
                return Ok(Reply::Check("allocation-failed", AllocElement().is_null()))
            }
            Effect::ProbeRecovery => {
                let h = AllocElement();
                let recovered = !h.is_null();
                if recovered {
                    FreeElement(h);
                }
                return Ok(Reply::Check("pool-recovered", recovered));
            }
            Effect::ZoomProof(frames, element, view, index) => {
                crate::mainloop::input_frame::drain_committed_draws();
                let frame = &*(*element).current.frame.cast::<frame_desc>();
                let mipmap = (view == 1).then(|| {
                    let adjacent = &*SetAbsFrameIndex(frames[view + 1], index);
                    (&*frame.image, &*adjacent.image)
                });
                return Ok(zoom_proof(
                    view,
                    crate::app::bridge::battle(|b| b.zoom_out),
                    index,
                    frame,
                    &*frames[view],
                    mipmap,
                ));
            }
            Effect::Captain(frame) => {
                return Ok(Reply::Captain(SetAbsFrameIndex(
                    frame,
                    GetFrameCount(frame) - 4,
                )))
            }
            Effect::ClearStar => crate::planets::starmap::CurStarDescPtr = ptr::null_mut(),
            _ => unreachable!(),
        }
        Ok(Reply::None)
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

#[cfg(not(test))]
impl World<'_> {
    fn callback_observation(
        &self,
        effect: &Effect,
        collision: Option<&Collision>,
    ) -> Option<(usize, usize)> {
        let parent = |h| self.elements[&h].p_parent as usize;
        match effect {
            Effect::NativeShip(callback, h) => Some((*callback as usize, parent(*h))),
            Effect::Invoke(callback, h) => {
                callback.map(|callback| (callback.address(), parent(*h)))
            }
            Effect::RaceProcess(post, h) => {
                let address = parent(*h);
                let callbacks = self.ships[&(address as HLINK)].callback_addresses;
                Some((callbacks[if *post { 2 } else { 1 }], address))
            }
            Effect::Death(h) => self.elements[h]
                .death_func
                .map(|callback| (callback.address(), parent(*h))),
            Effect::Collide(..) => collision.and_then(|collision| {
                collision
                    .callback
                    .map(|callback| (callback.address(), collision.parent))
            }),
            Effect::Intelligence(h, _) => {
                let address = self.ship(*h).callback_addresses[4];
                (address != 0).then_some((address, parent(*h)))
            }
            _ => None,
        }
    }

    fn projected_thinkers(&mut self, effect: &Effect) -> Vec<(HLINK, u16)> {
        if let Effect::Think(active) = effect {
            active
                .iter()
                .map(|&(element, crew)| (ptr::from_mut(self.ship_mut(element).state).cast(), crew))
                .collect()
        } else {
            Vec::new()
        }
    }

    fn callbacks(
        mut self,
        effects: Vec<Effect>,
        mut dispatch: impl FnMut(Effect) -> Result<Reply, String>,
    ) -> Vec<Result<Reply, String>> {
        let prepared = effects
            .into_iter()
            .map(|effect| {
                let collision = match effect {
                    Effect::Collide(a, b) => Some(collision(self.elements[&a], self.elements[&b])),
                    _ => None,
                };
                let death = match effect {
                    Effect::Death(h) => self.elements[&h].death_func,
                    _ => None,
                };
                let observed = self.callback_observation(&effect, collision.as_ref());
                let active = self.projected_thinkers(&effect);
                (effect, collision, death, observed, active)
            })
            .collect::<Vec<_>>();
        drop(self);
        prepared
            .into_iter()
            .map(|(effect, collision, death, observed, active)| {
                // SAFETY: callback metadata came from the owned live graph, now fully dropped.
                unsafe {
                    match effect {
                        Effect::RaceProcess(_, h) | Effect::Weapon(h) | Effect::NativeShip(_, h) => {
                            crate::app::bridge::enter(|cx| {
                                let id = cx.elements().id(h.cast()).unwrap_or_else(|| panic!("fixture element missing from pool"));
                                ship_effect(cx, id, effect, observed);
                            });
                        }
                        Effect::Death(h) => {
                            crate::battle::process_native::invoke_element::invoke_legacy_element(death.ok_or("fixture ship has no production death callback")?, h);
                        }
                        Effect::Invoke(callback, h) => {
                            let callback = callback.ok_or("fixture is missing its production callback")?;
                            observed.ok_or("fixture callback lacks owner metadata")?;
                            crate::battle::process_native::invoke_element::invoke_legacy_element(callback, h.cast());
                        }
                        Effect::Collide(a, b) => {
                            let Collision {
                                callback,
                                points: [mut pa, mut pb],
                                ..
                            } = collision.ok_or("collision effect lacks projected operands")?;
                            crate::battle::process_native::invoke_collision::invoke_legacy_collision(callback.ok_or("fixture projectile has no collision callback")?,
                                a,
                                &raw mut pa,
                                b,
                                &raw mut pb,
                             crate::battle::process_native::invoke_collision::CollisionCall::Hook);
                        }
                        Effect::Think(_) => {
                            for (owner, crew) in active {
                                let owner=owner.cast();
                                let input = crate::battle::cyborg_native::engine::tactical_intelligence(
                                    ptr::null_mut(),
                                    owner,
                                );
                                (*owner).ship_input_state =
                                    crate::battle::battle_native::ship_input(input, crew);
                            }
                        }
                        Effect::Intelligence(h, mut concerns) => {
                            let (_, parent) = observed.ok_or("fixture intelligence lacks owner metadata")?;
                            crate::engine::battle::Cx::invoke_intelligence(parent, h.cast(), concerns.as_mut_ptr(), concerns.len() as u16);
                        }
                        other => return dispatch(other),
                    }
                }
                Ok(Reply::None)
            })
            .collect()
    }
}

#[cfg(not(test))]
fn phoenix(state: crate::engine::battle::RaceState, handle: usize) -> Phoenix {
    let crate::engine::battle::RaceState::Pkunk {
        saved_pre,
        saved_post,
        saved_death,
        ..
    } = state
    else {
        panic!("Pkunk fixture state missing");
    };
    Phoenix {
        handle: handle as HElement,
        hooks: [saved_pre, saved_post, saved_death].map(|hook| hook.map_or(0, |f| f.address())),
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

#[cfg(not(test))]
fn ship_effect(
    cx: &mut crate::engine::battle::Cx<'_>,
    id: crate::engine::battle::ElementId,
    effect: Effect,
    observed: Option<(usize, usize)>,
) {
    match effect {
        Effect::RaceProcess(post, _) => {
            let ship = cx.owner(id);
            cx.invoke_ship_process(ship, post);
        }
        Effect::NativeShip(callback, _) => {
            let ship = cx.owner(id);
            let (_, parent) = observed.unwrap_or_else(|| panic!("fixture native callback owner"));
            crate::battle::process_native::dispatch::invoke(callback as usize, parent, || {
                callback(cx, ship)
            });
        }
        Effect::Weapon(_) => crate::battle::ship_native::ship_postprocess(cx, id),
        _ => unreachable!("non-ship fixture effect"),
    }
}
