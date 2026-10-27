//! The synchronous boundary owns raw traversal, resource retirement and callbacks.

use super::*;

#[cfg(not(test))]
fn hooks() -> Hooks {
    use crate::ships::races::{
        androsynth_native, chmmr_native, pkunk_native, urquan_native, vux_native,
    };
    Hooks {
        blazer_collision: androsynth_native::engine::blazer_collision as *const () as usize,
        satellite: Some(chmmr_native::engine::satellite_postprocess),
        defense: chmmr_native::engine::spawn_point_defense as *const () as usize,
        limpets: Some(vux_native::engine::spawn_limpets),
        fighters: Some(urquan_native::engine::spawn_fighters),
        fighter_preprocess: urquan_native::engine::fighter_preprocess as *const () as usize,
        pkunk: Some(pkunk_native::engine::pkunk_preprocess),
        intercept: pkunk_native::engine::intercept_pkunk_death as *const () as usize,
        rebirth: pkunk_native::engine::new_pkunk as *const () as usize,
        ship_death: Some(crate::battle::tactrans_native::ship_death),
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
                        crate::battle::battle_native::battle_counter = [7, 7];
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
    use crate::ships::races::pkunk_native::engine::PkunkData;
    // SAFETY: the caller holds the synchronous game-thread boundary, with no callbacks.
    unsafe {
        let mut world = World::new(
            GlobData.Game_state.CurrentActivity,
            crate::battle::battle_native::battle_counter,
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
                let next = world.insert_ship(Ship {
                    state: &mut *owner.cast(),
                    info: None,
                    data: None,
                    characteristics: None,
                    preprocess: None,
                    postprocess: None,
                    intelligence: None,
                    phoenix: None,
                });
                let slot = world.descriptors[&owner];
                let Some(ship) = world.ships.get_mut(&owner) else {
                    panic!("projected fixture owner {owner:p} disappeared from its graph");
                };
                if let Some(race) = (*slot).as_mut() {
                    if ship.state.SpeciesID == SPECIES_ID_PKUNK_ID {
                        let data = &*race.data.cast::<PkunkData>();
                        ship.phoenix = Some(phoenix(data));
                    }
                    ship.descriptor(
                        &mut race.ship_info,
                        &mut race.ship_data,
                        &mut race.characteristics,
                        (
                            race.preprocess_func,
                            race.postprocess_func,
                            race.cyborg_control.intelligence_func,
                        ),
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
            Effect::Weapon(h) => crate::battle::ship_native::ship_postprocess(h.cast()),
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
            Effect::ZoomProof(frames, frame) => {
                return Ok(Reply::Check(
                    "trilinear-index",
                    frames.iter().any(|&base| {
                        !base.is_null() && SetEquFrameIndex(base, frame.cast()) == frame.cast()
                    }),
                ))
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
            state,
            &mut *controls,
            GlobData.Game_state.CurrentActivity,
            request.cyborg,
        )?;
        for (side, slot, ship) in ships {
            crate::supermelee::native::teams::MeleeSetup_setShip(
                state.meleeSetup,
                side,
                slot,
                ship,
            );
        }
    }
    Ok(())
}

#[cfg(not(test))]
fn configure_menu(
    state: &mut MELEE_STATE,
    controls: &mut [u8; 2],
    activity: u16,
    cyborg: bool,
) -> Result<(), String> {
    if activity & IN_BATTLE as u16 != 0 || state.meleeStarted != 0 {
        return Err("cannot replace a fleet during battle".into());
    }
    controls.fill(if cyborg { 2 | 64 } else { 1 | 16 });
    state.MeleeOption = 4;
    state.InputFunc = Some(crate::supermelee::native::menu::live::DoMelee);
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
            Effect::Invoke(callback, h) => callback.map(|callback| (callback as usize, parent(*h))),
            Effect::Death(h) => self.elements[h]
                .death_func
                .map(|callback| (callback as usize, parent(*h))),
            Effect::Collide(..) => collision.and_then(|collision| {
                collision
                    .callback
                    .map(|callback| (callback as usize, collision.parent))
            }),
            Effect::Intelligence(callback, h, _) => {
                callback.map(|callback| (callback as usize, parent(*h)))
            }
            _ => None,
        }
    }

    fn callbacks(
        mut self,
        effects: Vec<Effect>,
        mut dispatch: impl FnMut(Effect) -> Result<Reply, String>,
    ) -> Vec<Result<Reply, String>> {
        use crate::collections::observation::observe;
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
                let active = match &effect {
                    Effect::Think(active) => active
                        .iter()
                        .map(|&(element, crew)| (ptr::from_mut(self.ship_mut(element).state), crew))
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                };
                (effect, collision, death, observed, active)
            })
            .collect::<Vec<_>>();
        drop(self);
        prepared
            .into_iter()
            .map(|(effect, collision, death, observed, active)| {
                if let Some((callback, parent)) = observed {
                    observe(callback, parent);
                }
                // SAFETY: callback metadata came from the owned live graph, now fully dropped.
                unsafe {
                    match effect {
                        Effect::Death(h) => {
                            death.ok_or("fixture ship has no production death callback")?(h);
                        }
                        Effect::Invoke(callback, h) => {
                            callback.ok_or("fixture is missing its production callback")?(h.cast());
                        }
                        Effect::Collide(a, b) => {
                            let Collision {
                                callback,
                                points: [mut pa, mut pb],
                                ..
                            } = collision.ok_or("collision effect lacks projected operands")?;
                            callback.ok_or("fixture projectile has no collision callback")?(
                                a,
                                (&raw mut pa).cast_const(),
                                b,
                                (&raw mut pb).cast_const(),
                            );
                        }
                        Effect::Think(_) => {
                            for (owner, crew) in active {
                                let input = crate::battle::cyborg_native::tactical_intelligence(
                                    ptr::null_mut(),
                                    owner,
                                );
                                (*owner).ship_input_state =
                                    crate::battle::battle_native::ship_input(input, crew);
                            }
                        }
                        Effect::Intelligence(callback, h, mut concerns) => {
                            callback.ok_or("Arilou intelligence callback is unavailable")?(
                                h.cast(),
                                concerns.as_mut_ptr(),
                                10,
                            );
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
fn phoenix(data: &crate::ships::races::pkunk_native::engine::PkunkData) -> Phoenix {
    Phoenix {
        handle: data.hPhoenix.cast(),
        hooks: [
            data.saved_preprocess_func.map_or(0, |f| f as usize),
            data.saved_postprocess_func.map_or(0, |f| f as usize),
            data.saved_death_func.map_or(0, |f| f as usize),
        ],
    }
}
