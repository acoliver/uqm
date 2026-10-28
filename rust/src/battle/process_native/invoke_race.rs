//! Race dispatch selects converted behavior before consulting legacy vtables.
#[cfg(not(test))]
use crate::collections::queue::HLink;
#[cfg(not(test))]
use crate::engine::battle::record::{ELEMENT, EVALUATE_DESC};
#[cfg(not(test))]
use crate::engine::battle::{Concern, Concerns, Cx, RaceState, ShipId, WeaponOrigin};

/// # Safety
/// The closure calls legacy code on the published thread without retaining records.
pub unsafe fn invoke_context<R>(callback: usize, parent: usize, call: impl FnOnce() -> R) -> R {
    // SAFETY: enter removes publication while deriving the nested handoff.
    unsafe {
        crate::app::bridge::enter(|cx| super::dispatch::invoke_legacy(cx, callback, parent, call))
    }
}
#[cfg(not(test))]
pub fn spawn_weapons(
    cx: &mut Cx<'_>,
    ship: ShipId,
    origin: WeaponOrigin,
) -> crate::engine::battle::WeaponSpawn {
    let parent = std::ptr::from_ref(cx.ship(ship)) as usize;
    if let Some(race) = cx.desc(ship).race {
        super::dispatch::invoke(race.init_weapon as usize, parent, || {
            (race.init_weapon)(cx, ship, origin)
        })
    } else {
        let callback = cx
            .desc(ship)
            .init_weapon_func
            .expect("ship weapon initializer");
        let id = cx.ship_element(ship);
        let pointer = cx.b.elements.raw(id).cast::<ELEMENT>();
        let mut weapons = [std::ptr::null_mut(); 6];
        // SAFETY: the checked ship and six output slots remain live for this handoff.
        let count = unsafe {
            super::dispatch::invoke_legacy(cx, callback as usize, parent, || {
                callback(pointer, weapons.as_mut_ptr())
            })
        };
        weapons[..usize::from(count)]
            .iter()
            .map(|&h| cx.b.elements.id(h))
            .collect()
    }
}

#[cfg(not(test))]
pub fn invoke_ship_process(cx: &mut Cx<'_>, ship: ShipId, post: bool) -> bool {
    let desc = cx.desc(ship);
    if let Some(race) = desc.race {
        if !post && matches!(desc.race_state, RaceState::Vux { entry_done: true }) {
            return false;
        }
        let callback = if post {
            race.postprocess
        } else {
            race.preprocess
        };
        if let Some(callback) = callback {
            let parent = std::ptr::from_ref(cx.ship(ship)) as usize;
            super::dispatch::invoke(callback as usize, parent, || callback(cx, ship));
            true
        } else {
            false
        }
    } else {
        let callback = if post {
            desc.postprocess_func
        } else {
            desc.preprocess_func
        };
        if let Some(callback) = callback {
            let element = cx.ship_element(ship);
            let pointer = cx.b.elements.raw(element).cast::<ELEMENT>();
            // SAFETY: the owner and checked element remain live across this handoff.
            unsafe {
                super::invoke_element::invoke_race_element(cx, callback, pointer);
            }
            true
        } else {
            false
        }
    }
}

/// # Safety
/// The owner, projected element and six weapon output slots must be live.
#[cfg(not(test))]
pub unsafe fn invoke_weapon(parent: usize, element: *mut ELEMENT, weapons: *mut HLink) -> u16 {
    // SAFETY: this raw legacy entry retains no record borrow across callbacks.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let ship = cx.ship_id(parent);
            if let Some(race) = cx.desc(ship).race {
                let origin = WeaponOrigin {
                    location: (*element).next.location,
                    player: (*element).playerNr,
                };
                let spawned = super::dispatch::invoke(race.init_weapon as usize, parent, || {
                    (race.init_weapon)(cx, ship, origin)
                });
                assert!(spawned.len() <= 6, "race produced too many weapons");
                for (i, id) in spawned.iter().enumerate() {
                    *weapons.add(i) = id.map_or(std::ptr::null_mut(), |id| cx.b.elements.raw(id));
                }
                spawned.len() as u16
            } else {
                let callback = cx
                    .desc(ship)
                    .init_weapon_func
                    .expect("ship weapon initializer");
                super::dispatch::invoke_legacy(cx, callback as usize, parent, || {
                    callback(element, weapons)
                })
            }
        })
    }
}

/// # Safety
/// The owner, projected element and four concern records must be live.
#[cfg(not(test))]
pub unsafe fn invoke_intelligence(
    parent: usize,
    element: *mut ELEMENT,
    concerns: *mut EVALUATE_DESC,
    count: u16,
) {
    // SAFETY: raw values are copied before any callback can re-enter the context.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let ship = cx.ship_id(parent);
            if let Some(race) = cx.desc(ship).race {
                let objects = std::array::from_fn(|i| {
                    let c = *concerns.add(i);
                    Concern {
                        object: cx.b.elements.id(c.ObjectPtr.cast()),
                        facing: c.facing,
                        which_turn: c.which_turn,
                        movement: c.MoveState,
                    }
                });
                let mut native = Concerns {
                    objects,
                    subject: *element,
                    count,
                };
                super::dispatch::invoke(race.intelligence as usize, parent, || {
                    (race.intelligence)(cx, ship, &mut native)
                });
                for (i, c) in native.objects.into_iter().enumerate() {
                    let output = concerns.add(i);
                    (*output).ObjectPtr = c
                        .object
                        .map_or(std::ptr::null_mut(), |id| cx.b.elements.raw(id).cast());
                    (*output).facing = c.facing;
                    (*output).which_turn = c.which_turn;
                    (*output).MoveState = c.movement;
                }
                *element = native.subject;
            } else {
                let callback = cx
                    .desc(ship)
                    .cyborg_control
                    .intelligence_func
                    .expect("ship intelligence");
                super::dispatch::invoke_legacy(cx, callback as usize, parent, || {
                    callback(element, concerns, count)
                });
            }
        })
    }
}
