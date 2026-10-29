//! A2/A4 entry points for races still calling the legacy intelligence surface.
#![cfg(not(test))]

use crate::battle::cyborg_native as ai;
use crate::engine::battle::{record::*, Cx, Subject};
use crate::graphics::native_abi::{INTERSECT_CONTROL, RECT};

pub(crate) fn frame_rect(frame: crate::graphics::native_abi::FRAME) -> RECT {
    let mut rect = RECT {
        corner: crate::graphics::native_abi::POINT { x: 0, y: 0 },
        extent: crate::graphics::native_abi::EXTENT {
            width: 0,
            height: 0,
        },
    };
    // SAFETY: projected subjects retain their loaded frame resources.
    unsafe {
        crate::ships::native::abi::GetFrameRect(frame, &mut rect);
    }
    rect
}
pub(crate) fn intersect(a: &mut INTERSECT_CONTROL, b: &mut INTERSECT_CONTROL, time: u16) -> u16 {
    // SAFETY: the local projections retain both loaded intersect frames.
    unsafe { crate::ships::native::abi::DrawablesIntersect(a, b, time) }
}

/// # Safety
/// The caller retains this projected legacy element and its resources on the game thread.
unsafe fn subject_call<R>(
    pointer: *mut ELEMENT,
    write_back: bool,
    call: impl FnOnce(&mut Cx<'_>, &mut Subject) -> R,
) -> R {
    // SAFETY: the published context is suspended by the legacy caller; no record borrow crosses call.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let mut subject = cx.legacy_subject(&*pointer);
            let result = call(cx, &mut subject);
            if write_back {
                *pointer = cx.encode_subject(&subject);
            }
            result
        })
    }
}

/// # Safety
/// Both pointers name live legacy element projections and retain their loaded resources.
#[no_mangle]
pub unsafe extern "C" fn PlotIntercept(
    a: *mut ELEMENT,
    b: *mut ELEMENT,
    turns: u16,
    margin: u16,
) -> u16 {
    // SAFETY: the caller retains both projections through this synchronous calculation.
    unsafe {
        crate::app::bridge::enter(|cx| {
            ai::plot_intercept(
                cx,
                &cx.legacy_intercept_subject(&*a),
                &cx.legacy_intercept_subject(&*b),
                turns,
                margin,
            )
        })
    }
}
/// # Safety
/// Ship is a live legacy subject with a live owner and loaded frames.
#[no_mangle]
pub unsafe extern "C" fn TurnShip(ship: *mut ELEMENT, angle: u16) -> u32 {
    // SAFETY: the subject is copied before the Native operation and written back afterward.
    unsafe { subject_call(ship, true, |cx, subject| ai::turn_ship(cx, subject, angle)) }
}
/// # Safety
/// Ship is a live legacy subject with a live owner.
#[no_mangle]
pub unsafe extern "C" fn ThrustShip(ship: *mut ELEMENT, angle: u16) -> u32 {
    // SAFETY: the subject is copied before the Native operation and written back afterward.
    unsafe {
        subject_call(ship, true, |cx, subject| {
            ai::thrust_ship(cx, subject, angle)
        })
    }
}
/// # Safety
/// Both pointers retain their subjects, owners and loaded resources on the game thread.
#[no_mangle]
pub unsafe extern "C" fn ship_weapons(ship: *mut ELEMENT, other: *mut ELEMENT, margin: u16) -> u32 {
    // SAFETY: the copied values outlive all weapon callbacks; no caller record borrow survives.
    unsafe {
        subject_call(ship, false, |cx, subject| {
            let other = cx.legacy_subject(&*other);
            ai::weapons(cx, subject, &other, margin)
        })
    }
}

pub(crate) unsafe fn decode_concerns(
    cx: &Cx<'_>,
    raw: *const EVALUATE_DESC,
    count: usize,
) -> Vec<crate::engine::battle::Concern> {
    (0..count)
        .map(|i| {
            // SAFETY: each caller retains count initialized concern slots.
            let item = unsafe { *raw.add(i) };
            crate::engine::battle::Concern {
                object: cx.elements().id(item.ObjectPtr.cast()),
                facing: item.facing,
                which_turn: item.which_turn,
                movement: item.MoveState,
            }
        })
        .collect()
}
pub(crate) unsafe fn encode_concerns(
    cx: &Cx<'_>,
    raw: *mut EVALUATE_DESC,
    concerns: &[crate::engine::battle::Concern],
) {
    for (i, item) in concerns.iter().enumerate() {
        // SAFETY: each caller retains one writable raw slot per concern value.
        unsafe {
            *raw.add(i) = EVALUATE_DESC {
                ObjectPtr: item
                    .object
                    .map_or(std::ptr::null_mut(), |id| cx.elements().raw(id).cast()),
                facing: item.facing,
                which_turn: item.which_turn,
                MoveState: item.movement,
            };
        }
    }
}
/// # Safety
/// The projected ship and count concern records are live for this game-thread call.
#[no_mangle]
pub unsafe extern "C" fn ship_intelligence(
    ship: *mut ELEMENT,
    raw: *mut EVALUATE_DESC,
    count: u16,
) {
    // SAFETY: raw slots are copied before Native dispatch and written back after it.
    unsafe {
        subject_call(ship, true, |cx, subject| {
            let mut concerns = decode_concerns(cx, raw, usize::from(count));
            ai::intelligence(cx, subject, &mut concerns, count);
            encode_concerns(cx, raw, &concerns);
        });
    }
}
/// # Safety
/// The projected ship and one concern record remain live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Pursue(ship: *mut ELEMENT, raw: *mut EVALUATE_DESC) {
    // SAFETY: the raw concern is copied before Native dispatch.
    unsafe {
        subject_call(ship, true, |cx, subject| {
            let mut concerns = decode_concerns(cx, raw, 1);
            ai::pursue(cx, subject, &mut concerns[0]);
            encode_concerns(cx, raw, &concerns);
        });
    }
}
/// # Safety
/// The projected ship and one concern record remain live on the game thread.
#[no_mangle]
pub unsafe extern "C" fn Entice(ship: *mut ELEMENT, raw: *mut EVALUATE_DESC) {
    // SAFETY: the raw concern is copied before Native dispatch.
    unsafe {
        subject_call(ship, true, |cx, subject| {
            let mut concerns = decode_concerns(cx, raw, 1);
            ai::entice(cx, subject, &mut concerns[0]);
            encode_concerns(cx, raw, &concerns);
        });
    }
}
/// # Safety
/// This legacy no-op retains its ABI until its last unconverted race caller moves.
#[no_mangle]
pub unsafe extern "C" fn Avoid(_ship: *mut ELEMENT, _raw: *mut EVALUATE_DESC) {}

/// # Safety
/// The owner is a live ship in the published battle context on the game thread.
#[no_mangle]
pub unsafe extern "C" fn tactical_intelligence(
    _context: *mut crate::ships::native::abi::ComputerInputContext,
    owner: *mut STARSHIP,
) -> u8 {
    // SAFETY: the owner address is decoded before Native dispatch.
    unsafe {
        crate::app::bridge::enter(|cx| {
            let id = cx.ship_id(owner as usize);
            ai::tactical(cx, id)
        })
    }
}

/// # Safety
/// Context and a non-null ship are live game-thread records in the published battle.
#[no_mangle]
pub unsafe extern "C" fn computer_intelligence(
    context: *mut crate::ships::native::abi::ComputerInputContext,
    ship: *mut STARSHIP,
) -> u8 {
    // SAFETY: input and ship addresses are decoded before the Native entry.
    unsafe {
        crate::app::bridge::enter(|cx| {
            if cx.world.activity() as u8 == crate::ships::native::abi::IN_LAST_BATTLE as u8 {
                return 0;
            }
            let player = (*context).playerNr;
            let id = (!ship.is_null()).then(|| cx.ship_id(ship as usize));
            let control = if id.is_none() {
                crate::supermelee::c_bridge::PlayerControl[player as usize]
            } else {
                0
            };
            crate::battle::ai_entry::computer_input(cx, player, id, control)
        })
    }
}
pub(crate) fn human_input(player: u16) -> u8 {
    // SAFETY: the owning game thread reads the current admitted input state.
    unsafe { crate::ships::native::abi::CurrentInputToBattleInput(player) }
}
pub(crate) fn selection_delay() {
    // SAFETY: the owning game thread performs the existing half-second picker delay.
    unsafe {
        crate::ships::native::abi::SleepThread(crate::ships::native::abi::ONE_SECOND >> 1);
    }
}

impl Cx<'_> {
    /// # Safety
    /// Owner, projected element, and four initialized concern slots remain live on the game thread.
    pub unsafe fn invoke_intelligence(
        parent: usize,
        element: *mut ELEMENT,
        raw: *mut EVALUATE_DESC,
        count: u16,
    ) {
        // SAFETY: the shared entry copies all operands before calling the race dispatcher.
        unsafe {
            subject_call(element, true, |cx, subject| {
                let ship = cx.ship_id(parent);
                let objects: [crate::engine::battle::Concern; 4] = decode_concerns(cx, raw, 4)
                    .try_into()
                    .unwrap_or_else(|_| panic!("missing checked battle value"));
                let mut concerns = crate::engine::battle::Concerns {
                    objects,
                    subject: *subject,
                    count,
                };
                cx.race_intelligence(ship, &mut concerns);
                *subject = concerns.subject;
                encode_concerns(cx, raw, &concerns.objects);
            });
        }
    }
}
