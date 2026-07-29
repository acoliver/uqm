//! Lifecycle animation constants and state, ported from the deleted
//! `sc2/src/uqm/planets/lander.c`.
//!
//! The C `PlanetSide()` function drove a multi-phase animation sequence:
//!
//! 1. **Warmup** — lander powers up, crew fills the bar.
//! 2. **Launch** — `AnimateLaunch(LanderFrame[5])`: plays all frames of the
//!    launch graphic, then emits `LANDER_DEPARTS`.
//! 3. **Landing** — `LandingTakeoffSequence(TRUE)`: one-second idle, then a
//!    13-step smooth-acceleration descent from `DISTANCE_COVERED` to ground.
//! 4. **Active** — the main gameplay loop (`DoPlanetSide`).
//! 5. **TakingOff** — `LandingTakeoffSequence(FALSE)`: 13-step ascent, then a
//!    half-second idle. The lander plays `LANDER_RETURNS` before ascending.
//! 6. **Return** — `AnimateLaunch(LanderFrame[6])`: plays all frames of the
//!    return graphic.
//! 7. **Explosion** — `LanderExplosion` + `EXPLOSION_WAIT_FRAMES` dramatic
//!    pause.
//!
//! All durations are expressed in 35-Hz simulation frames.

/// Surface viewport height — `SIS_SCREEN_HEIGHT - MAP_HEIGHT - MAP_BORDER_HEIGHT`.
pub const SURFACE_HEIGHT: i32 = 162;

/// `DISTANCE_COVERED` = `SURFACE_HEIGHT / 2 + 10`.
pub const DISTANCE_COVERED: i32 = SURFACE_HEIGHT / 2 + 10;

/// Number of acceleration steps produced by the triangular sum
/// `1 + 2 + 3 + … + n >= DISTANCE_COVERED`.
///
/// `1+2+…+13 = 91 >= 91`.
pub const DESCENT_STEPS: u16 = 13;

/// Idle frames before the landing descent — `ONE_SECOND` at 35 Hz.
pub const LANDING_IDLE_FRAMES: u16 = 35;

/// Idle frames after the takeoff ascent — `ONE_SECOND / 2` at 35 Hz (= 17.5,
/// rounded up to 18 to guarantee the lander clears the viewport).
pub const TAKEOFF_IDLE_FRAMES: u16 = 18;

/// `EXPLOSION_LIFE * 3` — the explosion element advances every 3rd frame via
/// `object_animation` (`turn_wait = MAKE_BYTE(2, 2)`, `LONIBBLE + 1 = 3`).
pub const EXPLOSION_ANIM_FRAMES: u16 = 30;

/// `EXPLOSION_WAIT_FRAMES` — `ONE_SECOND * 2 / PLANET_SIDE_RATE` = 70.
pub const EXPLOSION_WAIT_FRAMES: u16 = 70;

/// Total explosion phase duration: animation + dramatic wait.
pub const EXPLOSION_TOTAL_FRAMES: u16 = EXPLOSION_ANIM_FRAMES + EXPLOSION_WAIT_FRAMES;

/// Compute the smooth-acceleration vertical pixel offset for descent/ascent
/// step `step` (0-based).
///
/// Matches the C `landingOfs` computation: `delta += index + 1`, stored as
/// `-delta`. The result is always ≤ 0 (negative = upward on the surface
/// coordinate system).
///
/// ```text
/// step 0 → -(1)          = -1
/// step 1 → -(1+2)        = -3
/// step 2 → -(1+2+3)      = -6
/// …
/// step 12 → -(1+…+13)    = -91 = -DISTANCE_COVERED
/// ```
#[must_use]
pub fn descent_offset(step: u16) -> i32 {
    let n = i32::from(step) + 1;
    -(n * (n + 1) / 2)
}

/// Monotonic animation frame counter for lifecycle rendering phases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LifecycleAnimation {
    frame: u16,
}

impl LifecycleAnimation {
    /// Current frame index within the active animation phase.
    #[must_use]
    pub const fn frame(self) -> u16 {
        self.frame
    }

    /// Reset the counter to zero (start of a new phase).
    pub fn reset(&mut self) {
        self.frame = 0;
    }

    /// Advance one frame and return `true` if the animation has completed.
    ///
    /// Completion is `frame >= total` *after* incrementing, so the final
    /// rendered frame is `total - 1`.
    pub fn advance(&mut self, total: u16) -> bool {
        self.frame = self.frame.saturating_add(1);
        self.frame >= total
    }

    /// Vertical pixel offset for the landing descent.
    ///
    /// During the idle portion (`frame < LANDING_IDLE_FRAMES`) the lander is
    /// stationary at `-DISTANCE_COVERED`. Once the descent begins, the offset
    /// moves from step `DESCENT_STEPS-1` toward step `0` (ground level).
    #[must_use]
    pub fn landing_offset(self) -> i32 {
        if self.frame < LANDING_IDLE_FRAMES {
            return -DISTANCE_COVERED;
        }
        let step = self.frame - LANDING_IDLE_FRAMES;
        if step >= DESCENT_STEPS {
            return 0;
        }
        // Landing plays offsets in reverse: step 12, 11, …, 0.
        descent_offset(DESCENT_STEPS - 1 - step)
    }

    /// Vertical pixel offset for the takeoff ascent.
    ///
    /// During the ascent (`frame < DESCENT_STEPS`) the offset moves from
    /// step `0` (ground) toward step `DESCENT_STEPS-1` (`-DISTANCE_COVERED`).
    /// After that, the idle portion keeps the lander off-screen.
    #[must_use]
    pub fn takeoff_offset(self) -> i32 {
        if self.frame < DESCENT_STEPS {
            descent_offset(self.frame)
        } else {
            -DISTANCE_COVERED
        }
    }

    /// Total landing phase duration in frames.
    #[must_use]
    pub const fn landing_total() -> u16 {
        LANDING_IDLE_FRAMES + DESCENT_STEPS
    }

    /// Total takeoff phase duration in frames.
    #[must_use]
    pub const fn takeoff_total() -> u16 {
        DESCENT_STEPS + TAKEOFF_IDLE_FRAMES
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descent_offsets_match_c_landingofs() {
        // C: landingOfs[index] = -(sum of 1..index+1)
        assert_eq!(descent_offset(0), -1);
        assert_eq!(descent_offset(1), -3);
        assert_eq!(descent_offset(2), -6);
        assert_eq!(descent_offset(3), -10);
        assert_eq!(descent_offset(4), -15);
        assert_eq!(descent_offset(5), -21);
        assert_eq!(descent_offset(6), -28);
        assert_eq!(descent_offset(7), -36);
        assert_eq!(descent_offset(8), -45);
        assert_eq!(descent_offset(9), -55);
        assert_eq!(descent_offset(10), -66);
        assert_eq!(descent_offset(11), -78);
        assert_eq!(descent_offset(12), -91);
    }

    #[test]
    fn landing_offset_starts_high_and_descends_to_zero() {
        // Idle: stationary at -DISTANCE_COVERED.
        for frame in 0..LANDING_IDLE_FRAMES {
            let anim = LifecycleAnimation { frame };
            assert_eq!(anim.landing_offset(), -DISTANCE_COVERED);
        }

        // Descent: step 12 → 0 (reverse order).
        let anim = LifecycleAnimation {
            frame: LANDING_IDLE_FRAMES,
        };
        assert_eq!(anim.landing_offset(), descent_offset(12));
        let anim = LifecycleAnimation {
            frame: LANDING_IDLE_FRAMES + DESCENT_STEPS - 1,
        };
        assert_eq!(anim.landing_offset(), descent_offset(0));

        // After descent: at ground level.
        let anim = LifecycleAnimation {
            frame: LANDING_IDLE_FRAMES + DESCENT_STEPS,
        };
        assert_eq!(anim.landing_offset(), 0);
    }

    #[test]
    fn takeoff_offset_ascends_from_zero() {
        // Ascent: step 0 → 12 (forward order).
        let anim = LifecycleAnimation { frame: 0 };
        assert_eq!(anim.takeoff_offset(), descent_offset(0));
        let anim = LifecycleAnimation {
            frame: DESCENT_STEPS - 1,
        };
        assert_eq!(anim.takeoff_offset(), descent_offset(DESCENT_STEPS - 1));

        // After ascent: off-screen.
        let anim = LifecycleAnimation {
            frame: DESCENT_STEPS,
        };
        assert_eq!(anim.takeoff_offset(), -DISTANCE_COVERED);
    }

    #[test]
    fn advance_returns_false_until_total_then_true() {
        let mut anim = LifecycleAnimation::default();
        for _ in 0..4 {
            assert!(!anim.advance(5));
        }
        assert!(anim.advance(5));
        assert_eq!(anim.frame(), 5);
    }

    #[test]
    fn advance_saturates_at_u16_max() {
        let mut anim = LifecycleAnimation {
            frame: u16::MAX - 1,
        };
        anim.advance(0);
        assert_eq!(anim.frame(), u16::MAX);
        // Further advances stay at MAX.
        anim.advance(0);
        assert_eq!(anim.frame(), u16::MAX);
    }

    #[test]
    fn landing_total_is_idle_plus_descent() {
        assert_eq!(
            LifecycleAnimation::landing_total(),
            LANDING_IDLE_FRAMES + DESCENT_STEPS
        );
    }

    #[test]
    fn takeoff_total_is_ascent_plus_idle() {
        assert_eq!(
            LifecycleAnimation::takeoff_total(),
            DESCENT_STEPS + TAKEOFF_IDLE_FRAMES
        );
    }

    #[test]
    fn explosion_total_is_anim_plus_wait() {
        assert_eq!(
            EXPLOSION_TOTAL_FRAMES,
            EXPLOSION_ANIM_FRAMES + EXPLOSION_WAIT_FRAMES
        );
    }
}
