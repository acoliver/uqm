//! Animation handling for alien communication
//!
//! Manages alien character animations during conversations.

use std::time::Duration;

/// Animation frame rate (40 FPS)
pub const ANIM_FPS: u32 = 40;
pub const ANIM_FRAME_DURATION: Duration = Duration::from_millis(25);

/// Animation playback mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimPlayMode {
    /// Play once and stop
    #[default]
    Once,
    /// Loop continuously
    Loop,
    /// Play forward then backward
    PingPong,
    /// Random frame selection
    Random,
}

/// Animation state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AnimState {
    #[default]
    Stopped,
    Playing,
    Paused,
    Finished,
}

/// Animation description
#[derive(Debug, Clone)]
pub struct AnimDesc {
    /// Animation ID
    pub id: u32,
    /// Start frame index
    pub start_frame: u32,
    /// Number of frames
    pub frame_count: u32,
    /// Playback mode
    pub play_mode: AnimPlayMode,
    /// Frame rate multiplier (1.0 = normal)
    pub speed: f32,
    /// Priority (higher = more important)
    pub priority: u32,
}

impl Default for AnimDesc {
    fn default() -> Self {
        Self {
            id: 0,
            start_frame: 0,
            frame_count: 1,
            play_mode: AnimPlayMode::Once,
            speed: 1.0,
            priority: 0,
        }
    }
}

impl AnimDesc {
    /// Create a new animation description
    pub fn new(id: u32, start_frame: u32, frame_count: u32) -> Self {
        Self {
            id,
            start_frame,
            frame_count,
            ..Default::default()
        }
    }

    /// Create a looping animation
    pub fn looping(id: u32, start_frame: u32, frame_count: u32) -> Self {
        Self {
            id,
            start_frame,
            frame_count,
            play_mode: AnimPlayMode::Loop,
            ..Default::default()
        }
    }

    /// Set playback speed
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }
}

/// Running animation instance
#[derive(Debug, Clone)]
pub struct Animation {
    /// Animation description
    pub desc: AnimDesc,
    /// Current frame (0-based within animation)
    pub current_frame: u32,
    /// Time accumulated for current frame
    pub frame_time: Duration,
    /// Playback state
    pub state: AnimState,
    /// Direction for ping-pong mode (1 = forward, -1 = backward)
    pub direction: i32,
}

impl Animation {
    /// Create a new animation from description
    pub fn new(desc: AnimDesc) -> Self {
        Self {
            desc,
            current_frame: 0,
            frame_time: Duration::ZERO,
            state: AnimState::Stopped,
            direction: 1,
        }
    }

    /// Start the animation
    pub fn start(&mut self) {
        self.state = AnimState::Playing;
        self.current_frame = 0;
        self.frame_time = Duration::ZERO;
        self.direction = 1;
    }

    /// Stop the animation
    pub fn stop(&mut self) {
        self.state = AnimState::Stopped;
        self.current_frame = 0;
        self.frame_time = Duration::ZERO;
    }

    /// Pause the animation
    pub fn pause(&mut self) {
        if self.state == AnimState::Playing {
            self.state = AnimState::Paused;
        }
    }

    /// Resume the animation
    pub fn resume(&mut self) {
        if self.state == AnimState::Paused {
            self.state = AnimState::Playing;
        }
    }

    /// Update the animation
    pub fn update(&mut self, delta: Duration) {
        if self.state != AnimState::Playing {
            return;
        }

        // Adjust delta by speed
        let adjusted_delta = Duration::from_secs_f32(delta.as_secs_f32() * self.desc.speed);
        self.frame_time += adjusted_delta;

        // Check if we should advance frame
        while self.frame_time >= ANIM_FRAME_DURATION {
            self.frame_time -= ANIM_FRAME_DURATION;
            self.advance_frame();

            if self.state != AnimState::Playing {
                break;
            }
        }
    }

    /// Advance to next frame
    fn advance_frame(&mut self) {
        if self.desc.frame_count == 0 {
            self.state = AnimState::Finished;
            return;
        }

        match self.desc.play_mode {
            AnimPlayMode::Once => {
                if self.current_frame + 1 >= self.desc.frame_count {
                    self.state = AnimState::Finished;
                } else {
                    self.current_frame += 1;
                }
            }
            AnimPlayMode::Loop => {
                self.current_frame = (self.current_frame + 1) % self.desc.frame_count;
            }
            AnimPlayMode::PingPong => {
                let next = self.current_frame as i32 + self.direction;
                if next < 0 {
                    self.direction = 1;
                    self.current_frame = 1.min(self.desc.frame_count - 1);
                } else if next >= self.desc.frame_count as i32 {
                    self.direction = -1;
                    self.current_frame = self.desc.frame_count.saturating_sub(2);
                } else {
                    self.current_frame = next as u32;
                }
            }
            AnimPlayMode::Random => {
                // Simple pseudo-random (not cryptographic)
                self.current_frame =
                    (self.current_frame * 1103515245 + 12345) % self.desc.frame_count;
            }
        }
    }

    /// Get the actual frame index in the sprite sheet
    pub fn frame_index(&self) -> u32 {
        self.desc.start_frame + self.current_frame
    }

    /// Check if animation is running
    pub fn is_running(&self) -> bool {
        self.state == AnimState::Playing
    }

    /// Check if animation has finished
    pub fn is_finished(&self) -> bool {
        self.state == AnimState::Finished
    }
}

/// Animation context managing multiple animations
#[derive(Debug, Default)]
pub struct AnimContext {
    /// All animations
    animations: Vec<Animation>,
    /// Global time scale
    time_scale: f32,
    /// Whether animations are paused globally
    paused: bool,
}

impl AnimContext {
    /// Create a new animation context
    pub fn new() -> Self {
        Self {
            animations: Vec::new(),
            time_scale: 1.0,
            paused: false,
        }
    }

    /// Add an animation
    pub fn add(&mut self, desc: AnimDesc) -> usize {
        let id = self.animations.len();
        self.animations.push(Animation::new(desc));
        id
    }

    /// Get an animation by index
    pub fn get(&self, index: usize) -> Option<&Animation> {
        self.animations.get(index)
    }

    /// Get a mutable animation by index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Animation> {
        self.animations.get_mut(index)
    }

    /// Start an animation
    pub fn start(&mut self, index: usize) {
        if let Some(anim) = self.animations.get_mut(index) {
            anim.start();
        }
    }

    /// Stop an animation
    pub fn stop(&mut self, index: usize) {
        if let Some(anim) = self.animations.get_mut(index) {
            anim.stop();
        }
    }

    /// Start all animations
    pub fn start_all(&mut self) {
        for anim in &mut self.animations {
            anim.start();
        }
    }

    /// Stop all animations
    pub fn stop_all(&mut self) {
        for anim in &mut self.animations {
            anim.stop();
        }
    }

    /// Update all animations
    pub fn update(&mut self, delta: Duration) {
        if self.paused {
            return;
        }

        let scaled_delta = Duration::from_secs_f32(delta.as_secs_f32() * self.time_scale);

        for anim in &mut self.animations {
            anim.update(scaled_delta);
        }
    }

    /// Set global time scale
    pub fn set_time_scale(&mut self, scale: f32) {
        self.time_scale = scale.max(0.0);
    }

    /// Pause all animations
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Resume all animations
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Clear all animations
    pub fn clear(&mut self) {
        self.animations.clear();
    }

    /// Get number of animations
    pub fn count(&self) -> usize {
        self.animations.len()
    }

    /// Get number of running animations
    pub fn running_count(&self) -> usize {
        self.animations.iter().filter(|a| a.is_running()).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anim_desc_new() {
        let desc = AnimDesc::new(1, 0, 10);
        assert_eq!(desc.id, 1);
        assert_eq!(desc.start_frame, 0);
        assert_eq!(desc.frame_count, 10);
        assert_eq!(desc.play_mode, AnimPlayMode::Once);
        assert_eq!(desc.speed, 1.0);
    }

    #[test]
    fn test_anim_desc_looping() {
        let desc = AnimDesc::looping(1, 0, 5);
        assert_eq!(desc.play_mode, AnimPlayMode::Loop);
    }

    #[test]
    fn test_anim_desc_with_speed() {
        let desc = AnimDesc::new(1, 0, 5).with_speed(2.0);
        assert_eq!(desc.speed, 2.0);
    }

    #[test]
    fn test_animation_new() {
        let anim = Animation::new(AnimDesc::new(1, 0, 5));
        assert_eq!(anim.state, AnimState::Stopped);
        assert_eq!(anim.current_frame, 0);
    }

    #[test]
    fn test_animation_start_stop() {
        let mut anim = Animation::new(AnimDesc::new(1, 0, 5));

        anim.start();
        assert!(anim.is_running());

        anim.stop();
        assert!(!anim.is_running());
        assert_eq!(anim.current_frame, 0);
    }

    #[test]
    fn test_animation_pause_resume() {
        let mut anim = Animation::new(AnimDesc::new(1, 0, 5));

        anim.start();
        anim.pause();
        assert_eq!(anim.state, AnimState::Paused);

        anim.resume();
        assert!(anim.is_running());
    }

    #[test]
    fn test_animation_update_once() {
        let mut anim = Animation::new(AnimDesc::new(1, 0, 3));
        anim.start();

        // Advance 3 frames
        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 1);

        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 2);

        anim.update(ANIM_FRAME_DURATION);
        assert!(anim.is_finished());
    }

    #[test]
    fn test_animation_update_loop() {
        let mut anim = Animation::new(AnimDesc::looping(1, 0, 3));
        anim.start();

        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 1);

        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 2);

        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 0); // Wrapped
        assert!(anim.is_running());
    }

    #[test]
    fn test_animation_pingpong() {
        let mut anim = Animation::new(AnimDesc {
            play_mode: AnimPlayMode::PingPong,
            frame_count: 3,
            ..Default::default()
        });
        anim.start();

        // Forward: 0 -> 1 -> 2
        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 1);
        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 2);

        // Backward: 2 -> 1 -> 0
        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 1);
        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 0);
    }

    #[test]
    fn test_animation_frame_index() {
        let anim = Animation::new(AnimDesc::new(1, 10, 5));
        assert_eq!(anim.frame_index(), 10); // start_frame + current_frame
    }

    #[test]
    fn test_animation_speed() {
        let mut anim = Animation::new(AnimDesc::new(1, 0, 5).with_speed(2.0));
        anim.start();

        // At 2x speed, should advance 2 frames in the time of 1
        anim.update(ANIM_FRAME_DURATION);
        assert_eq!(anim.current_frame, 2);
    }

    #[test]
    fn test_anim_context_new() {
        let ctx = AnimContext::new();
        assert_eq!(ctx.count(), 0);
        assert!(!ctx.paused);
    }

    #[test]
    fn test_anim_context_add() {
        let mut ctx = AnimContext::new();
        let id = ctx.add(AnimDesc::new(1, 0, 5));
        assert_eq!(id, 0);
        assert_eq!(ctx.count(), 1);
    }

    #[test]
    fn test_anim_context_start_stop() {
        let mut ctx = AnimContext::new();
        ctx.add(AnimDesc::new(1, 0, 5));

        ctx.start(0);
        assert!(ctx.get(0).unwrap().is_running());

        ctx.stop(0);
        assert!(!ctx.get(0).unwrap().is_running());
    }

    #[test]
    fn test_anim_context_start_stop_all() {
        let mut ctx = AnimContext::new();
        ctx.add(AnimDesc::new(1, 0, 5));
        ctx.add(AnimDesc::new(2, 10, 5));

        ctx.start_all();
        assert_eq!(ctx.running_count(), 2);

        ctx.stop_all();
        assert_eq!(ctx.running_count(), 0);
    }

    #[test]
    fn test_anim_context_update() {
        let mut ctx = AnimContext::new();
        ctx.add(AnimDesc::looping(1, 0, 3));
        ctx.start(0);

        ctx.update(ANIM_FRAME_DURATION);
        assert_eq!(ctx.get(0).unwrap().current_frame, 1);
    }

    #[test]
    fn test_anim_context_pause_resume() {
        let mut ctx = AnimContext::new();
        ctx.add(AnimDesc::looping(1, 0, 3));
        ctx.start(0);

        ctx.pause();
        ctx.update(ANIM_FRAME_DURATION);
        assert_eq!(ctx.get(0).unwrap().current_frame, 0); // No change

        ctx.resume();
        ctx.update(ANIM_FRAME_DURATION);
        assert_eq!(ctx.get(0).unwrap().current_frame, 1); // Advanced
    }

    #[test]
    fn test_anim_context_time_scale() {
        let mut ctx = AnimContext::new();
        ctx.add(AnimDesc::looping(1, 0, 5));
        ctx.start(0);

        ctx.set_time_scale(2.0);
        ctx.update(ANIM_FRAME_DURATION);
        assert_eq!(ctx.get(0).unwrap().current_frame, 2); // 2x speed
    }

    #[test]
    fn test_anim_context_clear() {
        let mut ctx = AnimContext::new();
        ctx.add(AnimDesc::new(1, 0, 5));
        ctx.add(AnimDesc::new(2, 10, 5));

        ctx.clear();
        assert_eq!(ctx.count(), 0);
    }
}
