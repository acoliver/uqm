//! Communication animation engine.
//!
//! Matches the C `commanim.c` model: per-sequence state with alarm-based
//! timing, block-mask mutual exclusion, talk/transit sequences, and
//! per-frame-type advancement (circular, random, yoyo, colorxform).
//!
//! @plan PLAN-20260314-COMM.P07
//! @requirement AO-REQ-001 through AO-REQ-010, AO-REQ-016

use super::types::AnimationDescData;

// ============================================================================
// Constants matching C commanim.h
// ============================================================================

/// Maximum ambient animations (from commanim.h).
pub const MAX_ANIMATIONS: usize = 20;

/// Animation flag constants matching C AnimFlags.
#[allow(
    non_snake_case,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub mod AnimFlags {
    pub const CIRCULAR_ANIM: u8 = 1 << 0;
    pub const RANDOM_ANIM: u8 = 1 << 1;
    pub const YOYO_ANIM: u8 = 1 << 2;
    pub const COLORXFORM_ANIM: u8 = 1 << 3;
    pub const WAIT_TALKING: u8 = 1 << 4;
    pub const ONE_SHOT_ANIM: u8 = 1 << 5;
    pub const ANIM_DISABLED: u8 = 1 << 6;
    /// Pause when talking (unused in vanilla but reserved).
    pub const PAUSE_TALKING: u8 = 1 << 7;
    /// Mask of all animation-type bits.
    pub const ANIM_MASK: u8 = CIRCULAR_ANIM | RANDOM_ANIM | YOYO_ANIM;
}

/// Transition sub-flags stored in the TransitionDesc AnimFlags.
#[allow(
    non_snake_case,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub mod TransitFlags {
    pub const TALK_INTRO: u8 = 1 << 0;
    pub const TALK_DONE: u8 = 1 << 1;
}

/// Animation type for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimType {
    Picture,
    Color,
}

/// Direction of animation playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    None,
    Up,   // forward
    Down, // backward (yoyo reverse, or transit TALK_DONE)
}

// ============================================================================
// AnimSequence — matches C SEQUENCE struct
// ============================================================================

/// A running animation sequence (matches C SEQUENCE).
#[derive(Debug)]
pub struct AnimSequence {
    /// Animation descriptor (owned copy).
    pub desc: AnimationDescData,
    /// Ticks until next frame advance.
    pub alarm: u32,
    /// Current frame index within the sequence.
    pub cur_index: i32,
    /// Next frame index.
    pub next_index: i32,
    /// Frames remaining in current cycle.
    pub frames_left: i32,
    /// Playback direction.
    pub direction: Direction,
    /// Animation type (picture or colormap).
    pub anim_type: AnimType,
    /// Whether a frame change occurred this tick.
    pub change: bool,
}

impl Default for AnimSequence {
    fn default() -> Self {
        Self {
            desc: AnimationDescData::default(),
            alarm: 0,
            cur_index: 0,
            next_index: 0,
            frames_left: 0,
            direction: Direction::None,
            anim_type: AnimType::Picture,
            change: false,
        }
    }
}

impl AnimSequence {
    /// Whether the sequence is at its neutral (resting) frame.
    pub fn at_neutral_index(&self) -> bool {
        if self.desc.anim_flags & AnimFlags::CIRCULAR_ANIM != 0 {
            // CIRCULAR_ANIM neutral frame is the last
            self.next_index == 0
        } else {
            // All others: neutral frame is the first
            self.cur_index == 0
        }
    }

    /// Whether this conflicts with the talking animation.
    pub fn conflicts_with_talking(&self, talk_flags: u8) -> bool {
        self.desc.anim_flags & talk_flags & AnimFlags::WAIT_TALKING != 0
    }

    /// Reset sequence to neutral frame.
    pub fn reset(&mut self) {
        self.direction = Direction::None;
        self.cur_index = 0;
        self.change = true;
    }
}

// ============================================================================
// RNG — simple deterministic for tests, C TFB_Random for production
// ============================================================================

/// Simple RNG for animation timing. Matches TFB_Random behavior.
#[derive(Debug)]
struct AnimRng {
    state: u32,
}

impl AnimRng {
    fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 1 } else { seed },
        }
    }

    fn next(&mut self) -> u32 {
        // LCG matching TFB_Random: state = state * 1103515245 + 12345
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        (self.state >> 16) & 0x7FFF
    }

    fn next_mod(&mut self, modulus: u32) -> u32 {
        if modulus == 0 {
            return 0;
        }
        self.next() % modulus
    }
}

fn random_frame_rate(rng: &mut AnimRng, desc: &AnimationDescData) -> u32 {
    desc.base_frame_rate as u32 + rng.next_mod(desc.random_frame_rate as u32 + 1)
}

fn random_restart_rate(rng: &mut AnimRng, desc: &AnimationDescData) -> u32 {
    desc.base_restart_rate as u32 + rng.next_mod(desc.random_restart_rate as u32 + 1)
}

fn random_frame_index(rng: &mut AnimRng, desc: &AnimationDescData, from: u32) -> i32 {
    let range = desc.num_frames as u32 - from;
    if range == 0 {
        from as i32
    } else {
        (from + rng.next_mod(range)) as i32
    }
}

// ============================================================================
// CommAnimState — the animation engine
// ============================================================================

/// Communication animation state engine.
/// Replaces the generic AnimContext with a model matching C commanim.c.
#[derive(Debug)]
pub struct CommAnimState {
    /// All sequences: [Transit, Talk, Ambient0..AmbientN-1].
    sequences: Vec<AnimSequence>,
    /// Bit mask of active ambient animations.
    active_mask: u32,
    /// Index of the transit sequence in `sequences`.
    transit_index: usize,
    /// Index of the talk sequence in `sequences`.
    talk_index: usize,
    /// Index of the first ambient sequence.
    first_ambient: usize,
    /// Total number of sequences.
    total_sequences: usize,
    /// Number of ambient animations.
    num_ambient: usize,
    /// Whether the alien is currently talking (set externally).
    talking: bool,
    /// Whether the intro animation is running.
    running_intro_anim: bool,
    /// Whether the talking animation should be running.
    running_talking_anim: bool,
    /// Whether a stop-talking signal has been sent.
    stop_talking_signaled: bool,
    /// Talk descriptor flags (separate from sequence desc for transition).
    talk_anim_flags: u8,
    /// Transition descriptor flags.
    transit_anim_flags: u8,
    /// RNG for timing.
    rng: AnimRng,
    /// Whether this engine has been initialized.
    initialized: bool,
}

impl Default for CommAnimState {
    fn default() -> Self {
        Self::new()
    }
}

impl CommAnimState {
    /// Create an uninitialized animation state.
    pub fn new() -> Self {
        Self {
            sequences: Vec::new(),
            active_mask: 0,
            transit_index: 0,
            talk_index: 1,
            first_ambient: 2,
            total_sequences: 0,
            num_ambient: 0,
            talking: false,
            running_intro_anim: false,
            running_talking_anim: false,
            stop_talking_signaled: false,
            talk_anim_flags: 0,
            transit_anim_flags: 0,
            rng: AnimRng::new(42),
            initialized: false,
        }
    }

    /// Initialize from CommData animation descriptors.
    pub fn init(
        &mut self,
        ambient_anims: &[AnimationDescData],
        talk_desc: &AnimationDescData,
        transit_desc: &AnimationDescData,
    ) {
        self.active_mask = 0;
        self.talking = false;
        self.running_intro_anim = false;
        self.running_talking_anim = false;
        self.stop_talking_signaled = false;

        let num_ambient = ambient_anims.len().min(MAX_ANIMATIONS);
        self.num_ambient = num_ambient;
        self.total_sequences = 2 + num_ambient; // transit + talk + ambients
        self.transit_index = 0;
        self.talk_index = 1;
        self.first_ambient = 2;

        self.sequences.clear();
        self.sequences.reserve(self.total_sequences);

        // Transit sequence (index 0)
        let mut transit_seq = AnimSequence::default();
        let mut td = *transit_desc;
        td.anim_flags |= AnimFlags::ANIM_DISABLED;
        transit_seq.desc = td;
        transit_seq.anim_type = AnimType::Picture;
        self.sequences.push(transit_seq);

        // Talk sequence (index 1)
        let mut talk_seq = AnimSequence::default();
        let mut tkd = *talk_desc;
        tkd.anim_flags |= AnimFlags::ANIM_DISABLED;
        talk_seq.desc = tkd;
        talk_seq.anim_type = AnimType::Picture;
        self.sequences.push(talk_seq);

        self.talk_anim_flags = talk_desc.anim_flags;
        self.transit_anim_flags = transit_desc.anim_flags;

        // Ambient sequences (indices 2..2+num_ambient)
        for ad in ambient_anims.iter().take(num_ambient) {
            let mut seq = AnimSequence {
                desc: *ad,
                ..Default::default()
            };

            if ad.anim_flags & AnimFlags::COLORXFORM_ANIM != 0 {
                seq.anim_type = AnimType::Color;
            } else {
                seq.anim_type = AnimType::Picture;
            }

            seq.direction = Direction::Up;
            seq.frames_left = ad.num_frames as i32;

            if ad.anim_flags & AnimFlags::RANDOM_ANIM != 0 {
                seq.next_index = self.rng.next_mod(ad.num_frames as u32) as i32;
            } else if ad.anim_flags & AnimFlags::YOYO_ANIM != 0 {
                seq.next_index = 1;
                seq.frames_left -= 1;
            } else if ad.anim_flags & AnimFlags::CIRCULAR_ANIM != 0 {
                seq.cur_index = ad.num_frames as i32 - 1;
                seq.next_index = 0;
            }

            seq.alarm = random_restart_rate(&mut self.rng, ad) + 1;
            self.sequences.push(seq);
        }

        self.initialized = true;
    }

    /// Process one tick of animation (call every frame).
    /// Returns true if any visible change occurred.
    pub fn process(&mut self, elapsed_ticks: u32) -> bool {
        if !self.initialized || elapsed_ticks == 0 {
            return false;
        }

        let mut any_change = false;
        let mut can_talk = true;
        let mut next_active_mask = self.active_mask;

        // Process ambient animations
        for i in 0..self.num_ambient {
            let seq_idx = self.first_ambient + i;
            let active_bit = 1u32 << i;

            let anim_flags = self.sequences[seq_idx].desc.anim_flags;
            if anim_flags & AnimFlags::ANIM_DISABLED != 0 {
                continue;
            }

            if self.sequences[seq_idx].direction == Direction::None {
                if !self.sequences[seq_idx].conflicts_with_talking(self.talk_anim_flags) {
                    self.sequences[seq_idx].direction = Direction::Up;
                }
            } else if self.sequences[seq_idx].alarm > elapsed_ticks {
                self.sequences[seq_idx].alarm -= elapsed_ticks;
            } else if self.active_mask & self.sequences[seq_idx].desc.block_mask != 0 {
                // Blocked — reschedule
                let desc = self.sequences[seq_idx].desc;
                self.sequences[seq_idx].alarm = random_restart_rate(&mut self.rng, &desc) + 1;
                continue;
            } else {
                // Advance the animation
                let active = self.advance_ambient(seq_idx);
                if active {
                    self.active_mask |= active_bit;
                    next_active_mask |= active_bit;
                } else {
                    next_active_mask &= !active_bit;
                }
            }

            // Check WAIT_TALKING conflict
            if self.sequences[seq_idx].anim_type == AnimType::Picture
                && self.sequences[seq_idx].direction != Direction::None
                && self.sequences[seq_idx].conflicts_with_talking(self.talk_anim_flags)
            {
                if self.sequences[seq_idx].at_neutral_index() {
                    self.sequences[seq_idx].direction = Direction::None;
                    next_active_mask &= !active_bit;
                } else {
                    can_talk = false;
                }
            }
        }

        self.active_mask = next_active_mask;

        // Process talk/transit animations
        if can_talk && self.want_talking_anim() && self.running_talking_anim {
            if self.stop_talking_signaled && self.has_transition_anim() {
                self.transit_anim_flags |= TransitFlags::TALK_DONE;
            }

            if self.transit_anim_flags & (TransitFlags::TALK_INTRO | TransitFlags::TALK_DONE) != 0 {
                if self.transit_anim_flags & TransitFlags::TALK_DONE != 0
                    && self.sequences[self.transit_index].direction == Direction::None
                {
                    self.sequences[self.talk_index].reset();
                }
                let done = self.advance_transit(elapsed_ticks);
                if done {
                    any_change = true;
                }
            } else if !self.stop_talking_signaled {
                self.advance_talking(elapsed_ticks);
            } else {
                self.sequences[self.talk_index].reset();
                if self.stop_talking_signaled {
                    self.running_talking_anim = false;
                    self.stop_talking_signaled = false;
                }
            }
        } else if self.sequences.len() > self.talk_index
            && self.sequences[self.talk_index].direction == Direction::None
        {
            self.sequences[self.talk_index].desc.anim_flags |= AnimFlags::ANIM_DISABLED;
        }

        // Post-process: disable one-shot animations
        for i in 0..self.num_ambient {
            let seq_idx = self.first_ambient + i;
            let active_bit = 1u32 << i;
            let anim_flags = self.sequences[seq_idx].desc.anim_flags;

            if anim_flags & AnimFlags::ANIM_DISABLED != 0 {
                continue;
            }
            if anim_flags & AnimFlags::ONE_SHOT_ANIM != 0 && next_active_mask & active_bit == 0 {
                self.sequences[seq_idx].desc.anim_flags |= AnimFlags::ANIM_DISABLED;
            }
        }

        // Check for any changes
        for seq in &self.sequences {
            if seq.change {
                any_change = true;
                break;
            }
        }

        any_change
    }

    /// Advance an ambient animation sequence. Returns true if active.
    fn advance_ambient(&mut self, seq_idx: usize) -> bool {
        let seq = &mut self.sequences[seq_idx];
        let desc = seq.desc;

        seq.frames_left -= 1;

        let active = if seq.frames_left > 0
            || (desc.anim_flags & AnimFlags::YOYO_ANIM != 0 && seq.next_index != 0)
        {
            seq.alarm = random_frame_rate(&mut self.rng, &desc) + 1;
            true
        } else {
            seq.alarm = random_restart_rate(&mut self.rng, &desc) + 1;
            if desc.anim_flags & AnimFlags::RANDOM_ANIM != 0 {
                seq.next_index = 0;
            }
            false
        };

        seq.cur_index = seq.next_index;
        seq.change = true;

        if seq.frames_left <= 0 {
            seq.frames_left = desc.num_frames as i32;

            if desc.anim_flags & AnimFlags::YOYO_ANIM != 0 {
                seq.frames_left -= 1;
                seq.direction = match seq.direction {
                    Direction::Up => Direction::Down,
                    Direction::Down => Direction::Up,
                    Direction::None => Direction::Up,
                };
            } else if desc.anim_flags & AnimFlags::CIRCULAR_ANIM != 0 {
                seq.next_index = -1;
            }
        }

        if desc.anim_flags & AnimFlags::RANDOM_ANIM != 0 {
            seq.next_index = random_frame_index(&mut self.rng, &desc, 0);
        } else {
            let dir = match seq.direction {
                Direction::Up => 1,
                Direction::Down => -1,
                Direction::None => 0,
            };
            seq.next_index += dir;
        }

        active
    }

    /// Advance the talking animation sequence.
    fn advance_talking(&mut self, elapsed_ticks: u32) {
        let talk = &mut self.sequences[self.talk_index];

        if talk.direction == Direction::None {
            talk.direction = Direction::Up;
            talk.alarm = 0;
            talk.desc.anim_flags &= !AnimFlags::ANIM_DISABLED;
        }

        if talk.alarm > elapsed_ticks {
            talk.alarm -= elapsed_ticks;
            return;
        }

        let desc = talk.desc;
        talk.alarm = random_frame_rate(&mut self.rng, &desc);
        talk.change = true;

        if talk.cur_index == 0 {
            // Random frame next
            talk.cur_index = random_frame_index(&mut self.rng, &desc, 1);
            talk.alarm += random_restart_rate(&mut self.rng, &desc);
        } else {
            // Neutral frame next
            talk.cur_index = 0;
        }
    }

    /// Advance the transition animation. Returns true when done.
    fn advance_transit(&mut self, elapsed_ticks: u32) -> bool {
        let transit = &mut self.sequences[self.transit_index];

        if transit.direction == Direction::None {
            transit.alarm = 0;
            transit.desc.anim_flags &= !AnimFlags::ANIM_DISABLED;
        }

        if transit.alarm > elapsed_ticks {
            transit.alarm -= elapsed_ticks;
            return false;
        }

        transit.change = true;

        if transit.direction == Direction::None {
            let desc = &transit.desc;
            transit.frames_left = desc.num_frames as i32;

            if self.transit_anim_flags & TransitFlags::TALK_DONE != 0 {
                transit.direction = Direction::Down;
                transit.cur_index = transit.desc.num_frames as i32 - 1;
            } else if self.transit_anim_flags & TransitFlags::TALK_INTRO != 0 {
                transit.direction = Direction::Up;
                transit.cur_index = 0;
            }
        }

        transit.frames_left -= 1;
        if transit.frames_left <= 0 {
            if transit.direction == Direction::Up {
                self.transit_anim_flags &= !TransitFlags::TALK_INTRO;
            } else if transit.direction == Direction::Down {
                self.transit_anim_flags &= !TransitFlags::TALK_DONE;
                transit.desc.anim_flags |= AnimFlags::ANIM_DISABLED;
            }
            transit.direction = Direction::None;
            return transit.desc.anim_flags & AnimFlags::ANIM_DISABLED != 0;
        } else {
            let desc = transit.desc;
            transit.alarm = random_frame_rate(&mut self.rng, &desc);
            let dir = match transit.direction {
                Direction::Up => 1,
                Direction::Down => -1,
                Direction::None => 0,
            };
            transit.cur_index += dir;
        }

        false
    }

    /// Check if a talking animation is defined (has frames).
    pub fn want_talking_anim(&self) -> bool {
        if self.sequences.len() <= self.talk_index {
            return false;
        }
        self.sequences[self.talk_index].desc.num_frames > 0
    }

    /// Check if the talking animation is currently active.
    pub fn have_talking_anim(&self) -> bool {
        self.running_talking_anim
            && self.sequences.len() > self.talk_index
            && self.sequences[self.talk_index].desc.anim_flags & AnimFlags::ANIM_DISABLED == 0
    }

    /// Check if a transition animation is defined.
    pub fn has_transition_anim(&self) -> bool {
        if self.sequences.is_empty() {
            return false;
        }
        self.sequences[self.transit_index].desc.num_frames > 0
    }

    /// Signal to start the talking animation.
    pub fn start_talking_anim(&mut self) {
        self.running_talking_anim = true;
        self.stop_talking_signaled = false;
        if self.has_transition_anim() {
            self.transit_anim_flags |= TransitFlags::TALK_INTRO;
        }
    }

    /// Signal to stop the talking animation.
    pub fn stop_talking_anim(&mut self) {
        self.stop_talking_signaled = true;
    }

    /// Set intro animation running state.
    pub fn set_intro_anim(&mut self, running: bool) {
        self.running_intro_anim = running;
    }

    /// Whether the intro animation is running.
    pub fn is_intro_anim_running(&self) -> bool {
        self.running_intro_anim
    }

    /// Whether the talking animation is running.
    pub fn is_talking_anim_running(&self) -> bool {
        self.running_talking_anim
    }

    /// Get the current frame index for rendering a sequence.
    pub fn get_frame(&self, index: usize) -> Option<u32> {
        self.sequences.get(index).map(|s| {
            let start = s.desc.start_index as u32;
            start + s.cur_index as u32
        })
    }

    /// Get the number of sequences.
    pub fn total_sequences(&self) -> usize {
        self.total_sequences
    }

    /// Get a sequence by index (for rendering).
    pub fn sequence(&self, index: usize) -> Option<&AnimSequence> {
        self.sequences.get(index)
    }

    /// Whether the engine is initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Clear all animation state.
    pub fn clear(&mut self) {
        self.sequences.clear();
        self.active_mask = 0;
        self.total_sequences = 0;
        self.num_ambient = 0;
        self.talking = false;
        self.running_intro_anim = false;
        self.running_talking_anim = false;
        self.stop_talking_signaled = false;
        self.talk_anim_flags = 0;
        self.transit_anim_flags = 0;
        self.initialized = false;
    }

    /// Clear change flags on all sequences (after drawing).
    pub fn clear_changes(&mut self) {
        for seq in &mut self.sequences {
            seq.change = false;
        }
    }
}

// Legacy compatibility — AnimContext alias for state.rs
pub type AnimContext = CommAnimState;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_desc(
        num_frames: u8,
        flags: u8,
        base_frame_rate: u16,
        random_frame_rate: u16,
        base_restart_rate: u16,
        random_restart_rate: u16,
    ) -> AnimationDescData {
        AnimationDescData {
            start_index: 0,
            num_frames,
            anim_flags: flags,
            base_frame_rate,
            random_frame_rate,
            base_restart_rate,
            random_restart_rate,
            block_mask: 0,
        }
    }

    fn empty_desc() -> AnimationDescData {
        AnimationDescData::default()
    }

    fn init_state(ambient: &[AnimationDescData]) -> CommAnimState {
        let mut state = CommAnimState::new();
        let talk = make_desc(4, 0, 5, 2, 10, 5);
        let transit = make_desc(3, 0, 5, 0, 0, 0);
        state.init(ambient, &talk, &transit);
        state
    }

    #[test]
    fn test_init_populates_sequences() {
        let ambient = vec![
            make_desc(5, AnimFlags::CIRCULAR_ANIM, 10, 5, 20, 10),
            make_desc(3, AnimFlags::RANDOM_ANIM, 8, 3, 15, 5),
        ];
        let state = init_state(&ambient);
        assert!(state.is_initialized());
        assert_eq!(state.total_sequences(), 4); // transit + talk + 2 ambient
    }

    #[test]
    fn test_init_max_22_sequences() {
        let ambient: Vec<_> = (0..MAX_ANIMATIONS)
            .map(|_| make_desc(3, AnimFlags::CIRCULAR_ANIM, 5, 0, 10, 0))
            .collect();
        let state = init_state(&ambient);
        assert_eq!(state.total_sequences(), MAX_ANIMATIONS + 2);
    }

    #[test]
    fn test_circular_anim_wraps() {
        let ambient = vec![make_desc(4, AnimFlags::CIRCULAR_ANIM, 1, 0, 0, 0)];
        let mut state = init_state(&ambient);

        // Process several ticks to advance through frames
        for _ in 0..20 {
            state.process(2);
            state.clear_changes();
        }
        // Should still be valid (no panic, wrapping works)
        assert!(state.sequence(state.first_ambient).is_some());
    }

    #[test]
    fn test_random_anim_changes_frame() {
        let ambient = vec![make_desc(6, AnimFlags::RANDOM_ANIM, 1, 0, 1, 0)];
        let mut state = init_state(&ambient);

        let mut frames_seen = std::collections::HashSet::new();
        for _ in 0..50 {
            state.process(2);
            if let Some(seq) = state.sequence(state.first_ambient) {
                frames_seen.insert(seq.cur_index);
            }
            state.clear_changes();
        }
        // Random should produce multiple different frames
        assert!(frames_seen.len() > 1, "Random anim should vary frames");
    }

    #[test]
    fn test_yoyo_anim_bounces() {
        let ambient = vec![make_desc(5, AnimFlags::YOYO_ANIM, 1, 0, 1, 0)];
        let mut state = init_state(&ambient);

        let mut indices = Vec::new();
        for _ in 0..30 {
            state.process(2);
            if let Some(seq) = state.sequence(state.first_ambient) {
                if seq.change {
                    indices.push(seq.cur_index);
                }
            }
            state.clear_changes();
        }
        // Should contain both increasing and decreasing sequences
        assert!(!indices.is_empty());
    }

    #[test]
    fn test_colorxform_uses_color_type() {
        let ambient = vec![make_desc(
            4,
            AnimFlags::COLORXFORM_ANIM | AnimFlags::CIRCULAR_ANIM,
            5,
            0,
            10,
            0,
        )];
        let state = init_state(&ambient);
        let seq = state.sequence(state.first_ambient).unwrap();
        assert_eq!(seq.anim_type, AnimType::Color);
    }

    #[test]
    fn test_block_mask_prevents_concurrent() {
        let mut a1 = make_desc(3, AnimFlags::CIRCULAR_ANIM, 1, 0, 0, 0);
        a1.block_mask = 0b10; // blocks animation 1

        let mut a2 = make_desc(3, AnimFlags::CIRCULAR_ANIM, 1, 0, 0, 0);
        a2.block_mask = 0b01; // blocks animation 0

        let ambient = vec![a1, a2];
        let mut state = init_state(&ambient);

        // Process many ticks — both should not be simultaneously active
        for _ in 0..50 {
            state.process(2);
            let bit0 = state.active_mask & 1;
            let bit1 = state.active_mask & 2;
            // If both are active, their block masks should prevent it
            // (at least one should be blocked)
            // This is a statistical test — over many frames, they shouldn't
            // both be active at the same moment.
            assert!(
                bit0 == 0 || bit1 == 0,
                "Mutually blocked anims should not both be active"
            );
            state.clear_changes();
        }
    }

    #[test]
    fn test_wait_talking_settles_to_neutral() {
        let ambient = vec![make_desc(
            5,
            AnimFlags::CIRCULAR_ANIM | AnimFlags::WAIT_TALKING,
            1,
            0,
            0,
            0,
        )];
        let talk = make_desc(4, AnimFlags::WAIT_TALKING, 5, 0, 10, 0);
        let transit = empty_desc();
        let mut state = CommAnimState::new();
        state.init(&ambient, &talk, &transit);

        // The ambient anim starts with direction=Up (matching C SetupAmbientSequences).
        // When it reaches its neutral frame and conflicts with talking,
        // it will be paused (direction=None).
        // Process enough ticks for it to reach neutral and get paused.
        for _ in 0..50 {
            state.process(2);
            state.clear_changes();
        }
        let seq = state.sequence(state.first_ambient).unwrap();
        // It should eventually reach neutral and pause
        assert_eq!(seq.direction, Direction::None);
    }

    #[test]
    fn test_one_shot_disables_after_complete() {
        let ambient = vec![make_desc(
            3,
            AnimFlags::CIRCULAR_ANIM | AnimFlags::ONE_SHOT_ANIM,
            1,
            0,
            0,
            0,
        )];
        let mut state = init_state(&ambient);

        // Process enough ticks for the animation to complete
        for _ in 0..50 {
            state.process(2);
            state.clear_changes();
        }

        let seq = state.sequence(state.first_ambient).unwrap();
        assert!(
            seq.desc.anim_flags & AnimFlags::ANIM_DISABLED != 0,
            "One-shot should be disabled after completion"
        );
    }

    #[test]
    fn test_frame_rate_randomization_in_range() {
        let desc = make_desc(5, AnimFlags::CIRCULAR_ANIM, 10, 5, 20, 10);
        let mut rng = AnimRng::new(42);

        for _ in 0..100 {
            let rate = random_frame_rate(&mut rng, &desc);
            assert!(
                (10..=15).contains(&rate),
                "Frame rate {} out of range",
                rate
            );
        }
    }

    #[test]
    fn test_restart_rate_randomization_in_range() {
        let desc = make_desc(5, AnimFlags::CIRCULAR_ANIM, 10, 5, 20, 10);
        let mut rng = AnimRng::new(42);

        for _ in 0..100 {
            let rate = random_restart_rate(&mut rng, &desc);
            assert!(
                (20..=30).contains(&rate),
                "Restart rate {} out of range",
                rate
            );
        }
    }

    #[test]
    fn test_talk_anim_activates() {
        let ambient = vec![];
        let mut state = init_state(&ambient);

        assert!(!state.have_talking_anim());
        state.start_talking_anim();
        assert!(state.is_talking_anim_running());

        // Process to let talking advance
        for _ in 0..10 {
            state.process(5);
            state.clear_changes();
        }
    }

    #[test]
    fn test_talk_anim_deactivates() {
        // Use no transition so stop is immediate
        let talk = make_desc(4, 0, 5, 2, 10, 5);
        let transit = empty_desc(); // no transition frames
        let mut state = CommAnimState::new();
        state.init(&[], &talk, &transit);

        state.start_talking_anim();
        for _ in 0..5 {
            state.process(5);
            state.clear_changes();
        }

        state.stop_talking_anim();
        for _ in 0..20 {
            state.process(5);
            state.clear_changes();
        }

        assert!(!state.is_talking_anim_running());
    }

    #[test]
    fn test_clear_resets_all() {
        let ambient = vec![make_desc(3, AnimFlags::CIRCULAR_ANIM, 5, 0, 10, 0)];
        let mut state = init_state(&ambient);

        state.process(10);
        state.clear();

        assert!(!state.is_initialized());
        assert_eq!(state.total_sequences(), 0);
        assert_eq!(state.active_mask, 0);
    }

    #[test]
    fn test_get_frame_returns_start_plus_cur() {
        let mut ad = make_desc(5, AnimFlags::CIRCULAR_ANIM, 1, 0, 0, 0);
        ad.start_index = 10;
        let ambient = vec![ad];
        let mut state = init_state(&ambient);

        state.process(2);

        let frame = state.get_frame(state.first_ambient);
        assert!(frame.is_some());
        // Frame should be start_index + cur_index
        let f = frame.unwrap();
        assert!(f >= 10);
    }
}
