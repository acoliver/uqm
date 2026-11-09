//! Live dialogue animation engine, ported from commanim.c (GPL-2.0-or-later).

use super::locdata::CAnimationDesc;
use crate::comm::Comm;

const RANDOM: u8 = 1;
const CIRCULAR: u8 = 2;
const YOYO: u8 = 4;
const WAIT_TALKING: u8 = 8;
const COLOR: u8 = 16;
const INTRO: u8 = 32;
const DONE: u8 = 64;
const DISABLED: u8 = 128;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct Sequence {
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
        alarm: 0,
        direction: 0,
        current: 0,
        next: 0,
        left: 0,
        kind: 0,
        change: 0,
    };

    fn neutral(&self, d: &CAnimationDesc) -> bool {
        if d.anim_flags & CIRCULAR != 0 {
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

    fn setup_ambient(d: &CAnimationDesc, random: &mut impl FnMut() -> u32) -> Self {
        let mut s = Self {
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

    fn advance_ambient(&mut self, d: &CAnimationDesc, random: &mut impl FnMut() -> u32) -> bool {
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

    fn advance_talk(
        &mut self,
        d: &mut CAnimationDesc,
        ticks: u32,
        live: CAnimationDesc,
        random: &mut impl FnMut() -> u32,
    ) {
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

    fn advance_transit(
        &mut self,
        d: &mut CAnimationDesc,
        ticks: u32,
        live: &mut CAnimationDesc,
        random: &mut impl FnMut() -> u32,
    ) -> bool {
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

pub(crate) mod runtime {
    use super::*;
    use crate::comm::locdata::{CPoint, CStamp};

    use crate::graphics::cmap_abi::GetColorMapAddress;

    use crate::graphics::cmap_abi::XFormColorMap;

    use crate::graphics::cmap_abi::XFormColorMap_step;

    use crate::graphics::frame_abi::DrawStamp;
    use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

    use crate::graphics::pixmap_abi::SetAbsFrameIndex;

    use crate::strings::string_table::SetAbsStringTableIndex;
    use crate::time::counter_abi::GetTimeCounter;

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

    pub(crate) struct RuntimeState {
        pub(crate) last_time: u32,
        pub(crate) sequences: [Sequence; 22],
        pub(crate) active_mask: u32,
        pub(crate) talk_desc: CAnimationDesc,
        pub(crate) transit_desc: CAnimationDesc,
        pub(crate) total: u16,
    }
    impl Default for RuntimeState {
        fn default() -> Self {
            Self {
                last_time: 0,
                sequences: [Sequence::EMPTY; 22],
                active_mask: 0,
                talk_desc: EMPTY_DESC,
                transit_desc: EMPTY_DESC,
                total: 0,
            }
        }
    }
    fn random() -> u32 {
        crate::math::random()
    }

    /// # Safety
    /// CommData must be initialized and exclusively owned by the game thread.
    pub unsafe fn init_comm_animations(comm: &mut Comm) {
        comm.animation_runtime.active_mask = 0;
        comm.animation_runtime.talk_desc = comm.data.alien_talk_desc;
        comm.animation_runtime.transit_desc = comm.data.alien_transition_desc;
        comm.animation_runtime.transit_desc.anim_flags |= DISABLED;
        comm.animation_runtime.talk_desc.anim_flags |= DISABLED;
        comm.animation_runtime.sequences[0] = Sequence::EMPTY;
        comm.animation_runtime.sequences[1] = Sequence::EMPTY;
        for i in 0..usize::from(comm.data.num_animations) {
            comm.animation_runtime.sequences[i + 2] =
                Sequence::setup_ambient(&comm.data.alien_ambient_array[i], &mut random);
        }
        comm.animation_runtime.total = 2 + comm.data.num_animations;
        comm.animation_runtime.last_time = GetTimeCounter();
    }

    /// # Safety
    /// Called from the game thread with initialized animations and graphics.
    pub unsafe fn process_comm_animations(comm: &mut Comm, mut full: i32, paused: i32) -> i32 {
        if paused != 0 {
            XFormColorMap_step();
            return 0;
        }
        let now = GetTimeCounter();
        let elapsed = now.wrapping_sub(comm.animation_runtime.last_time);
        comm.animation_runtime.last_time = now;
        let mut next_mask = comm.animation_runtime.active_mask;
        let mut can_talk = true;
        for i in 0..usize::from(comm.data.num_animations) {
            let s = &mut comm.animation_runtime.sequences[i + 2];
            let d = &comm.data.alien_ambient_array[i];
            let bit = 1u32 << i;
            if d.anim_flags & DISABLED != 0 {
                continue;
            }
            let conflicts = d.anim_flags & comm.data.alien_talk_desc.anim_flags & WAIT_TALKING != 0;
            if s.direction == 0 {
                if !conflicts {
                    s.direction = 1;
                }
            } else if s.alarm > elapsed {
                s.alarm -= elapsed;
            } else if comm.animation_runtime.active_mask & d.block_mask != 0 {
                assert_eq!(
                    comm.animation_runtime.active_mask & bit,
                    0,
                    "Check animations' mutual blocking masks"
                );
                assert!(s.neutral(d));
                s.alarm = restart_rate(d, &mut random) + 1;
                continue;
            } else if s.advance_ambient(d, &mut random) {
                comm.animation_runtime.active_mask |= bit;
                next_mask |= bit;
            } else {
                next_mask &= !bit;
            }
            if s.kind == 0 && s.direction != 0 && conflicts {
                if s.neutral(d) {
                    s.direction = 0;
                    next_mask &= !bit;
                } else {
                    can_talk = false;
                }
            }
        }
        comm.animation_runtime.active_mask = next_mask;
        process_talking(comm, elapsed, can_talk);
        BatchGraphics();
        if XFormColorMap_step() != 0 {
            full = 1;
        }
        for i in 0..usize::from(comm.data.num_animations) {
            let s = &mut comm.animation_runtime.sequences[i + 2];
            let d = &comm.data.alien_ambient_array[i];
            if d.anim_flags & DISABLED != 0 || s.kind != 1 || s.change == 0 {
                continue;
            }
            XFormColorMap(
                GetColorMapAddress(SetAbsStringTableIndex(
                    (comm.data.alien_colormap).cast(),
                    d.start_index.wrapping_add(s.current),
                )),
                s.alarm.wrapping_sub(1) as i16,
            );
            s.change = 0;
        }
        let mut change = draw_alien_frame(comm, comm.animation_runtime.total, full);
        if full != 0 {
            change = 1;
        }
        UnbatchGraphics();
        for i in 0..usize::from(comm.data.num_animations) {
            let d = &mut comm.data.alien_ambient_array[i];
            if d.anim_flags & DISABLED != 0 {
                continue;
            }
            if d.anim_flags & INTRO != 0 && next_mask & (1 << i) == 0 {
                d.anim_flags |= DISABLED;
            }
        }
        change
    }

    fn process_talking(comm: &mut Comm, elapsed: u32, can_talk: bool) {
        let flags = comm.data.alien_talk_desc.anim_flags;
        if can_talk && comm.data.alien_talk_desc.num_frames > 0 && flags & WAIT_TALKING != 0 {
            let mut done = false;
            if flags & DONE != 0 && comm.data.alien_transition_desc.num_frames > 0 {
                comm.data.alien_transition_desc.anim_flags |= DONE;
            }
            if comm.data.alien_transition_desc.anim_flags & (INTRO | DONE) != 0 {
                if comm.data.alien_transition_desc.anim_flags & DONE != 0
                    && comm.animation_runtime.sequences[0].direction == 0
                {
                    comm.animation_runtime.sequences[1].reset();
                }
                done = comm.animation_runtime.sequences[0].advance_transit(
                    &mut comm.animation_runtime.transit_desc,
                    elapsed,
                    &mut comm.data.alien_transition_desc,
                    &mut random,
                );
            } else if flags & DONE == 0 {
                comm.animation_runtime.sequences[1].advance_talk(
                    &mut comm.animation_runtime.talk_desc,
                    elapsed,
                    comm.data.alien_talk_desc,
                    &mut random,
                );
            } else {
                comm.animation_runtime.sequences[1].reset();
                done = true;
            }
            if comm.data.alien_talk_desc.anim_flags & DONE != 0 && done {
                comm.data.alien_talk_desc.anim_flags &= !(WAIT_TALKING | DONE);
            }
        } else if comm.animation_runtime.sequences[1].direction == 0 {
            comm.animation_runtime.talk_desc.anim_flags |= DISABLED;
        }
    }

    /// # Safety
    /// Sequence array, CommData, and drawing context must be live and exclusive.
    pub unsafe fn draw_alien_frame(comm: &mut Comm, count: u16, full: i32) -> i32 {
        let mut change = 0;
        BatchGraphics();
        let mut stamp = CStamp {
            origin: CPoint { x: 0, y: 0 },
            frame: comm.data.alien_frame,
        };
        if full != 0 {
            DrawStamp((&raw mut stamp).cast());
            for i in (0..usize::from(comm.data.num_animations)).rev() {
                let descriptor = &raw mut comm.data.alien_ambient_array[i];
                let d = &mut *descriptor;
                if d.anim_flags & (RANDOM | CIRCULAR | YOYO) != 0 {
                    continue;
                }
                d.anim_flags |= DISABLED;
                if d.anim_flags & COLOR == 0 {
                    stamp.frame =
                        (SetAbsFrameIndex((comm.data.alien_frame).cast(), d.start_index)).cast();
                    DrawStamp((&raw mut stamp).cast());
                }
            }
        }
        {
            for i in (0..usize::from(count)).rev() {
                let s = &mut comm.animation_runtime.sequences[i];
                let d = match i {
                    0 => &comm.animation_runtime.transit_desc,
                    1 => &comm.animation_runtime.talk_desc,
                    _ => &comm.data.alien_ambient_array[i - 2],
                };
                if d.anim_flags & DISABLED != 0 || s.kind != 0 {
                    continue;
                }
                if full == 0 && s.change == 0 {
                    continue;
                }
                stamp.frame = (SetAbsFrameIndex(
                    (comm.data.alien_frame).cast(),
                    d.start_index.wrapping_add(s.current),
                ))
                .cast();
                DrawStamp((&raw mut stamp).cast());
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
        let d = CAnimationDesc {
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
        {
            let mut s = Sequence::setup_ambient(&d, &mut rng);
            assert_eq!((s.current, s.next, s.alarm), (2, 0, 10));
            assert!(s.neutral(&d));
            assert!(s.advance_ambient(&d, &mut rng));
            assert_eq!((s.current, s.next, s.alarm), (0, 1, 5));
            assert!(s.advance_ambient(&d, &mut rng));
            assert!(!s.advance_ambient(&d, &mut rng));
            assert_eq!((s.current, s.next, s.alarm), (2, 0, 10));
            assert!(s.neutral(&d));
        }
        assert_eq!(calls, 4);
    }

    #[test]
    fn yoyo_returns_to_neutral_before_becoming_inactive() {
        let d = CAnimationDesc {
            num_frames: 3,
            anim_flags: YOYO,
            ..Default::default()
        };
        {
            let mut s = Sequence::setup_ambient(&d, &mut || 0);
            let mut frames = Vec::new();
            let mut active = Vec::new();
            for _ in 0..4 {
                active.push(s.advance_ambient(&d, &mut || 0));
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
        let d = CAnimationDesc {
            num_frames: 2,
            anim_flags: RANDOM,
            ..Default::default()
        };
        let mut calls = 0;
        let mut rng = || {
            calls += 1;
            1
        };
        {
            let mut s = Sequence::setup_ambient(&d, &mut rng);
            assert!(s.advance_ambient(&d, &mut rng));
            assert!(!s.advance_ambient(&d, &mut rng));
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
        let mut s = Sequence { ..Sequence::EMPTY };
        let mut calls = 0;
        let mut rng = || {
            calls += 1;
            2
        };
        {
            s.advance_talk(&mut d, 0, live, &mut rng);
            assert_eq!((s.current, s.alarm), (3, 15));
            s.advance_talk(&mut d, 14, live, &mut rng);
            assert_eq!((s.current, s.alarm), (3, 1));
            s.advance_talk(&mut d, 1, live, &mut rng);
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
        let mut s = Sequence { ..Sequence::EMPTY };
        {
            assert!(!s.advance_transit(&mut d, 0, &mut live, &mut || 0));
            assert_eq!((s.current, s.direction, d.start_index), (0, -1, 10));
            assert!(s.advance_transit(&mut d, 0, &mut live, &mut || 0));
            assert_eq!(live.anim_flags, INTRO);
            assert_ne!(d.anim_flags & DISABLED, 0);
            assert!(!s.advance_transit(&mut d, 0, &mut live, &mut || 0));
            assert_eq!((s.current, s.direction, d.start_index), (1, 1, 30));
            assert!(!s.advance_transit(&mut d, 0, &mut live, &mut || 0));
            assert_eq!(live.anim_flags, 0);
        }
    }
}

#[cfg(test)]
mod owner_tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn initialized_sequences_survive_a_move_and_a_paused_frame() {
        let mut first = Comm::new();
        let second = Comm::new();
        first.data.num_animations = 1;
        first.data.alien_ambient_array[0].num_frames = 1;
        first.data.alien_ambient_array[0].anim_flags = CIRCULAR;
        unsafe {
            runtime::init_comm_animations(&mut first);
        }
        first.animation_runtime.last_time = 17;
        let mut moved = Box::new(first);
        assert_eq!(moved.animation_runtime.total, 3);
        assert_eq!(second.animation_runtime.total, 0);
        assert_eq!(
            unsafe { runtime::process_comm_animations(&mut moved, 0, 1) },
            0
        );
        assert_eq!(moved.animation_runtime.last_time, 17);
        assert_eq!(moved.animation_runtime.total, 3);
    }
}
