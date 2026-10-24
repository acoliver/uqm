//! Live dialogue animation engine, ported from commanim.c (GPL-2.0-or-later).

use super::locdata::CAnimationDesc;

const RANDOM: u8 = 1;
const CIRCULAR: u8 = 2;
const YOYO: u8 = 4;
#[cfg(not(test))]
const WAIT_TALKING: u8 = 8;
const COLOR: u8 = 16;
const INTRO: u8 = 32;
const DONE: u8 = 64;
const DISABLED: u8 = 128;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Sequence {
    desc: *mut CAnimationDesc,
    alarm: u32,
    direction: i32,
    current: u16,
    next: u16,
    left: u16,
    kind: i32,
    change: i32,
}

impl Sequence {
    const EMPTY: Self = Self {
        desc: std::ptr::null_mut(),
        alarm: 0,
        direction: 0,
        current: 0,
        next: 0,
        left: 0,
        kind: 0,
        change: 0,
    };

    unsafe fn neutral(&self) -> bool {
        if (*self.desc).anim_flags & CIRCULAR != 0 {
            self.next == 0
        } else {
            self.current == 0
        }
    }

    fn reset(&mut self) {
        self.direction = 0;
        self.current = 0;
        self.change = 1;
    }

    unsafe fn setup_ambient(desc: *mut CAnimationDesc, random: &mut impl FnMut() -> u32) -> Self {
        let d = &*desc;
        let mut s = Self {
            desc,
            direction: 1,
            left: d.num_frames.into(),
            kind: i32::from(d.anim_flags & COLOR != 0),
            ..Self::EMPTY
        };
        if d.anim_flags & RANDOM != 0 {
            s.next = (random() % u32::from(d.num_frames)) as u16;
        } else if d.anim_flags & YOYO != 0 {
            s.next = 1;
            s.left = s.left.wrapping_sub(1);
        } else if d.anim_flags & CIRCULAR != 0 {
            s.current = u16::from(d.num_frames).wrapping_sub(1);
            s.next = 0;
        }
        s.alarm = restart_rate(d, random) + 1;
        s
    }

    unsafe fn advance_ambient(&mut self, random: &mut impl FnMut() -> u32) -> bool {
        let d = &*self.desc;
        self.left = self.left.wrapping_sub(1);
        let active = self.left != 0 || (d.anim_flags & YOYO != 0 && self.next != 0);
        if active {
            self.alarm = frame_rate(d, random) + 1;
        } else {
            self.alarm = restart_rate(d, random) + 1;
            if d.anim_flags & RANDOM != 0 {
                self.next = 0;
            }
        }
        self.current = self.next;
        self.change = 1;
        if self.left == 0 {
            self.left = d.num_frames.into();
            if d.anim_flags & YOYO != 0 {
                self.left = self.left.wrapping_sub(1);
                self.direction = -self.direction;
            } else if d.anim_flags & CIRCULAR != 0 {
                self.next = u16::MAX;
            }
        }
        if d.anim_flags & RANDOM != 0 {
            self.next = frame_index(d, 0, random);
        } else {
            self.next = self.next.wrapping_add(self.direction as u16);
        }
        active
    }

    unsafe fn advance_talk(
        &mut self,
        ticks: u32,
        live: CAnimationDesc,
        random: &mut impl FnMut() -> u32,
    ) {
        let d = &mut *self.desc;
        if self.direction == 0 {
            self.direction = 1;
            if live.start_index != d.start_index {
                *d = live;
            }
            assert_eq!(self.current, 0);
            self.alarm = 0;
            d.anim_flags &= !DISABLED;
        }
        if self.alarm > ticks {
            self.alarm -= ticks;
            return;
        }
        self.alarm = frame_rate(d, random);
        self.change = 1;
        if self.current == 0 {
            self.current = frame_index(d, 1, random);
            self.alarm += restart_rate(d, random);
        } else {
            self.current = 0;
        }
    }

    unsafe fn advance_transit(
        &mut self,
        ticks: u32,
        live: &mut CAnimationDesc,
        random: &mut impl FnMut() -> u32,
    ) -> bool {
        let d = &mut *self.desc;
        let mut done = false;
        if self.direction == 0 {
            self.alarm = 0;
            d.anim_flags &= !DISABLED;
        }
        if self.alarm > ticks {
            self.alarm -= ticks;
            return false;
        }
        self.change = 1;
        if self.direction == 0 {
            // Preserve C's order: frame count comes from the old descriptor.
            self.left = d.num_frames.into();
            if live.anim_flags & DONE != 0 {
                self.direction = -1;
                self.current = u16::from(d.num_frames).wrapping_sub(1);
            } else if live.anim_flags & INTRO != 0 {
                self.direction = 1;
                if live.start_index != d.start_index {
                    *d = *live;
                }
                self.current = 0;
            }
        }
        self.left = self.left.wrapping_sub(1);
        if self.left == 0 {
            if self.direction == 1 {
                live.anim_flags &= !INTRO;
            } else if self.direction == -1 {
                live.anim_flags &= !DONE;
                d.anim_flags |= DISABLED;
                done = true;
            }
            self.direction = 0;
        } else {
            self.alarm = frame_rate(d, random);
            self.current = self.current.wrapping_add(self.direction as u16);
        }
        done
    }
}

fn frame_rate(d: &CAnimationDesc, random: &mut impl FnMut() -> u32) -> u32 {
    u32::from(d.base_frame_rate) + random() % (u32::from(d.random_frame_rate) + 1)
}
fn restart_rate(d: &CAnimationDesc, random: &mut impl FnMut() -> u32) -> u32 {
    u32::from(d.base_restart_rate) + random() % (u32::from(d.random_restart_rate) + 1)
}
fn frame_index(d: &CAnimationDesc, from: u16, random: &mut impl FnMut() -> u32) -> u16 {
    (u32::from(from) + random() % (u32::from(d.num_frames) - u32::from(from))) as u16
}

#[cfg(not(test))]
mod runtime {
    use super::*;
    use crate::comm::locdata::{CPoint, CStamp, COMM_DATA};
    use std::ffi::c_void;

    use crate::time::counter_abi::GetTimeCounter;
    #[allow(clashing_extern_declarations)]
    extern "C" {
        fn XFormColorMap_step() -> i32;
        fn XFormColorMap(map: *mut c_void, ticks: i16) -> u32;
        fn SetAbsStringTableIndex(map: *mut c_void, index: u16) -> *mut c_void;
        fn GetColorMapAddress(map: *mut c_void) -> *mut c_void;
        fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
        fn DrawStamp(stamp: *mut CStamp);
    }
    use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

    static mut LAST_TIME: u32 = 0;
    static mut SEQUENCES: [Sequence; 22] = [Sequence::EMPTY; 22];
    static mut ACTIVE_MASK: u32 = 0;
    const EMPTY_DESC: CAnimationDesc = CAnimationDesc {
        start_index: 0,
        num_frames: 0,
        anim_flags: 0,
        base_frame_rate: 0,
        random_frame_rate: 0,
        base_restart_rate: 0,
        random_restart_rate: 0,
        block_mask: 0,
    };
    static mut TALK_DESC: CAnimationDesc = EMPTY_DESC;
    static mut TRANSIT_DESC: CAnimationDesc = EMPTY_DESC;
    static mut TOTAL: u16 = 0;

    fn random() -> u32 {
        crate::math::random()
    }

    /// # Safety
    /// CommData must be initialized and exclusively owned by the game thread.
    #[no_mangle]
    pub unsafe extern "C" fn InitCommAnimations() {
        ACTIVE_MASK = 0;
        TALK_DESC = COMM_DATA.alien_talk_desc;
        TRANSIT_DESC = COMM_DATA.alien_transition_desc;
        TRANSIT_DESC.anim_flags |= DISABLED;
        TALK_DESC.anim_flags |= DISABLED;
        SEQUENCES[0] = Sequence {
            desc: &raw mut TRANSIT_DESC,
            ..Sequence::EMPTY
        };
        SEQUENCES[1] = Sequence {
            desc: &raw mut TALK_DESC,
            ..Sequence::EMPTY
        };
        for i in 0..usize::from(COMM_DATA.num_animations) {
            SEQUENCES[i + 2] =
                Sequence::setup_ambient(&raw mut COMM_DATA.alien_ambient_array[i], &mut random);
        }
        TOTAL = 2 + COMM_DATA.num_animations;
        LAST_TIME = GetTimeCounter();
    }

    /// # Safety
    /// Called from the game thread with initialized animations and graphics.
    #[no_mangle]
    pub unsafe extern "C" fn ProcessCommAnimations(mut full: i32, paused: i32) -> i32 {
        if paused != 0 {
            XFormColorMap_step();
            return 0;
        }
        let now = GetTimeCounter();
        let elapsed = now.wrapping_sub(LAST_TIME);
        LAST_TIME = now;
        let mut next_mask = ACTIVE_MASK;
        let mut can_talk = true;
        for i in 0..usize::from(COMM_DATA.num_animations) {
            let s = &mut SEQUENCES[i + 2];
            let d = &*s.desc;
            let bit = 1u32 << i;
            if d.anim_flags & DISABLED != 0 {
                continue;
            }
            let conflicts = d.anim_flags & COMM_DATA.alien_talk_desc.anim_flags & WAIT_TALKING != 0;
            if s.direction == 0 {
                if !conflicts {
                    s.direction = 1;
                }
            } else if s.alarm > elapsed {
                s.alarm -= elapsed;
            } else if ACTIVE_MASK & d.block_mask != 0 {
                assert_eq!(
                    ACTIVE_MASK & bit,
                    0,
                    "Check animations' mutual blocking masks"
                );
                assert!(s.neutral());
                s.alarm = restart_rate(d, &mut random) + 1;
                continue;
            } else if s.advance_ambient(&mut random) {
                ACTIVE_MASK |= bit;
                next_mask |= bit;
            } else {
                next_mask &= !bit;
            }
            if s.kind == 0 && s.direction != 0 && conflicts {
                if s.neutral() {
                    s.direction = 0;
                    next_mask &= !bit;
                } else {
                    can_talk = false;
                }
            }
        }
        ACTIVE_MASK = next_mask;
        let flags = COMM_DATA.alien_talk_desc.anim_flags;
        if can_talk && COMM_DATA.alien_talk_desc.num_frames > 0 && flags & WAIT_TALKING != 0 {
            let mut done = false;
            if flags & DONE != 0 && COMM_DATA.alien_transition_desc.num_frames > 0 {
                COMM_DATA.alien_transition_desc.anim_flags |= DONE;
            }
            if COMM_DATA.alien_transition_desc.anim_flags & (INTRO | DONE) != 0 {
                if COMM_DATA.alien_transition_desc.anim_flags & DONE != 0
                    && SEQUENCES[0].direction == 0
                {
                    SEQUENCES[1].reset();
                }
                let transition = &raw mut COMM_DATA.alien_transition_desc;
                done = SEQUENCES[0].advance_transit(elapsed, &mut *transition, &mut random);
            } else if flags & DONE == 0 {
                SEQUENCES[1].advance_talk(elapsed, COMM_DATA.alien_talk_desc, &mut random);
            } else {
                SEQUENCES[1].reset();
                done = true;
            }
            if COMM_DATA.alien_talk_desc.anim_flags & DONE != 0 && done {
                COMM_DATA.alien_talk_desc.anim_flags &= !(WAIT_TALKING | DONE);
            }
        } else if SEQUENCES[1].direction == 0 {
            TALK_DESC.anim_flags |= DISABLED;
        }
        BatchGraphics();
        if XFormColorMap_step() != 0 {
            full = 1;
        }
        for i in 0..usize::from(COMM_DATA.num_animations) {
            let s = &mut SEQUENCES[i + 2];
            let d = &*s.desc;
            if d.anim_flags & DISABLED != 0 || s.kind != 1 || s.change == 0 {
                continue;
            }
            XFormColorMap(
                GetColorMapAddress(SetAbsStringTableIndex(
                    COMM_DATA.alien_colormap,
                    d.start_index.wrapping_add(s.current),
                )),
                s.alarm.wrapping_sub(1) as i16,
            );
            s.change = 0;
        }
        let mut change = DrawAlienFrame((&raw mut SEQUENCES).cast(), TOTAL, full);
        if full != 0 {
            change = 1;
        }
        UnbatchGraphics();
        for i in 0..usize::from(COMM_DATA.num_animations) {
            let d = &mut *SEQUENCES[i + 2].desc;
            if d.anim_flags & DISABLED != 0 {
                continue;
            }
            if d.anim_flags & INTRO != 0 && next_mask & (1 << i) == 0 {
                d.anim_flags |= DISABLED;
            }
        }
        change
    }

    /// # Safety
    /// Sequence array, CommData, and drawing context must be live and exclusive.
    #[no_mangle]
    pub unsafe extern "C" fn DrawAlienFrame(
        sequences: *mut Sequence,
        count: u16,
        full: i32,
    ) -> i32 {
        let mut change = 0;
        BatchGraphics();
        let mut stamp = CStamp {
            origin: CPoint { x: 0, y: 0 },
            frame: COMM_DATA.alien_frame,
        };
        if full != 0 {
            DrawStamp(&raw mut stamp);
            for i in (0..usize::from(COMM_DATA.num_animations)).rev() {
                let descriptor = &raw mut COMM_DATA.alien_ambient_array[i];
                let d = &mut *descriptor;
                if d.anim_flags & (RANDOM | CIRCULAR | YOYO) != 0 {
                    continue;
                }
                d.anim_flags |= DISABLED;
                if d.anim_flags & COLOR == 0 {
                    stamp.frame = SetAbsFrameIndex(COMM_DATA.alien_frame, d.start_index);
                    DrawStamp(&raw mut stamp);
                }
            }
        }
        if !sequences.is_null() {
            for i in (0..usize::from(count)).rev() {
                let s = &mut *sequences.add(i);
                let d = &*s.desc;
                if d.anim_flags & DISABLED != 0 || s.kind != 0 {
                    continue;
                }
                if full == 0 && s.change == 0 {
                    continue;
                }
                stamp.frame =
                    SetAbsFrameIndex(COMM_DATA.alien_frame, d.start_index.wrapping_add(s.current));
                DrawStamp(&raw mut stamp);
                s.change = 0;
                change = 1;
            }
        }
        UnbatchGraphics();
        change
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circular_neutral_last_and_restart_delay() {
        let mut d = CAnimationDesc {
            num_frames: 3,
            anim_flags: CIRCULAR,
            base_frame_rate: 4,
            base_restart_rate: 9,
            ..Default::default()
        };
        let mut calls = 0;
        let mut rng = || {
            calls += 1;
            12
        };
        unsafe {
            let mut s = Sequence::setup_ambient(&mut d, &mut rng);
            assert_eq!((s.current, s.next, s.alarm), (2, 0, 10));
            assert!(s.neutral());
            assert!(s.advance_ambient(&mut rng));
            assert_eq!((s.current, s.next, s.alarm), (0, 1, 5));
            assert!(s.advance_ambient(&mut rng));
            assert!(!s.advance_ambient(&mut rng));
            assert_eq!((s.current, s.next, s.alarm), (2, 0, 10));
            assert!(s.neutral());
        }
        assert_eq!(calls, 4);
    }

    #[test]
    fn yoyo_returns_to_neutral_before_becoming_inactive() {
        let mut d = CAnimationDesc {
            num_frames: 3,
            anim_flags: YOYO,
            ..Default::default()
        };
        unsafe {
            let mut s = Sequence::setup_ambient(&mut d, &mut || 0);
            let mut frames = Vec::new();
            let mut active = Vec::new();
            for _ in 0..4 {
                active.push(s.advance_ambient(&mut || 0));
                frames.push(s.current);
            }
            assert_eq!(frames, [1, 2, 1, 0]);
            assert_eq!(active, [true, true, true, false]);
            assert_eq!((s.next, s.direction), (1, 1));
            s.reset();
            assert_eq!((s.direction, s.current, s.change), (0, 0, 1));
        }
    }

    #[test]
    fn random_end_draws_neutral_but_still_consumes_next_rng() {
        let mut d = CAnimationDesc {
            num_frames: 2,
            anim_flags: RANDOM,
            ..Default::default()
        };
        let mut calls = 0;
        let mut rng = || {
            calls += 1;
            1
        };
        unsafe {
            let mut s = Sequence::setup_ambient(&mut d, &mut rng);
            assert!(s.advance_ambient(&mut rng));
            assert!(!s.advance_ambient(&mut rng));
            assert_eq!((s.current, s.next), (0, 1));
        }
        assert_eq!(calls, 6);
    }

    #[test]
    fn talking_alternates_random_and_neutral_without_extra_rng_on_wait() {
        let mut d = CAnimationDesc {
            start_index: 10,
            num_frames: 4,
            base_frame_rate: 5,
            base_restart_rate: 10,
            anim_flags: DISABLED,
            ..Default::default()
        };
        let live = d;
        let mut s = Sequence {
            desc: &mut d,
            ..Sequence::EMPTY
        };
        let mut calls = 0;
        let mut rng = || {
            calls += 1;
            2
        };
        unsafe {
            s.advance_talk(0, live, &mut rng);
            assert_eq!((s.current, s.alarm), (3, 15));
            s.advance_talk(14, live, &mut rng);
            assert_eq!((s.current, s.alarm), (3, 1));
            s.advance_talk(1, live, &mut rng);
            assert_eq!((s.current, s.alarm), (0, 5));
        }
        assert_eq!(calls, 4);
        assert_eq!(d.anim_flags & DISABLED, 0);
    }

    #[test]
    fn transition_finishes_done_before_switching_descriptor_for_intro() {
        let mut d = CAnimationDesc {
            start_index: 10,
            num_frames: 2,
            ..Default::default()
        };
        let mut live = CAnimationDesc {
            start_index: 30,
            num_frames: 2,
            anim_flags: INTRO | DONE,
            ..Default::default()
        };
        let mut s = Sequence {
            desc: &mut d,
            ..Sequence::EMPTY
        };
        unsafe {
            assert!(!s.advance_transit(0, &mut live, &mut || 0));
            assert_eq!((s.current, s.direction, d.start_index), (0, -1, 10));
            assert!(s.advance_transit(0, &mut live, &mut || 0));
            assert_eq!(live.anim_flags, INTRO);
            assert_ne!(d.anim_flags & DISABLED, 0);
            assert!(!s.advance_transit(0, &mut live, &mut || 0));
            assert_eq!((s.current, s.direction, d.start_index), (1, 1, 30));
            assert!(!s.advance_transit(0, &mut live, &mut || 0));
            assert_eq!(live.anim_flags, 0);
        }
    }
}
