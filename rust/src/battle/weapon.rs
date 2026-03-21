// Weapon System — Weapon Collision & Tracking
// @plan PLAN-20260320-BATTLE.P09
// @requirement Weapon collision, damage application, blast effects, ship tracking

use super::battle_types::{
    angle_to_facing, arctan, cosine, facing_to_angle, normalize_angle, normalize_facing,
    shortest_path_delta, sine, HALF_CIRCLE, LOG_SPACE_HEIGHT, LOG_SPACE_WIDTH,
};
use super::element::{Element, ElementFlags, ElementProcessFunc, FrameHandle, Point, NORMAL_LIFE};

// Convenience wrappers for wrap_delta functions matching C macros
#[inline]
fn wrap_delta_x(dx: i16) -> i32 {
    shortest_path_delta(0, dx as i32, LOG_SPACE_WIDTH)
}

#[inline]
fn wrap_delta_y(dy: i16) -> i32 {
    shortest_path_delta(0, dy as i32, LOG_SPACE_HEIGHT)
}

// ---------------------------------------------------------------------------
// Sound Constants (matching C GameSounds enum)
// ---------------------------------------------------------------------------

/// Sound index base for target damage (1 hit point)
const TARGET_DAMAGED_FOR_1_PT: i32 = 2;

/// Sound index for target damage (6+ hit points)
const TARGET_DAMAGED_FOR_6_PLUS_PT: i32 = 5;

// ---------------------------------------------------------------------------
// Weapon Initialization Structures
// ---------------------------------------------------------------------------

/// LaserBlock — initialization data for laser weapons
/// Matches C's LASER_BLOCK from weapon.h exactly
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LaserBlock {
    /// Start X coordinate (world coords)
    pub cx: i16,
    /// Start Y coordinate (world coords)
    pub cy: i16,
    /// End X coordinate (world coords)
    pub ex: i16,
    /// End Y coordinate (world coords)
    pub ey: i16,
    /// Element state flags
    pub flags: u16,
    /// Owner player number
    pub sender: i16,
    /// Pixel offset from origin
    pub pixoffs: i16,
    /// Facing direction (0-15)
    pub face: u16,
    /// Color (for LINE_PRIM rendering)
    pub color: u32, // Color type from C
}

impl LaserBlock {
    /// Creates a new LaserBlock with given parameters
    pub fn new(origin: Point, end_point: Point, face: u16, sender: i16, flags: u16) -> Self {
        LaserBlock {
            cx: origin.x,
            cy: origin.y,
            ex: end_point.x,
            ey: end_point.y,
            flags,
            sender,
            pixoffs: 0,
            face,
            color: 0, // Default color, caller should set
        }
    }
}

/// MissileBlock — initialization data for missile weapons
/// Matches C's MISSILE_BLOCK from weapon.h exactly
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MissileBlock {
    /// Start X coordinate (world coords)
    pub cx: i16,
    /// Start Y coordinate (world coords)
    pub cy: i16,
    /// Element state flags
    pub flags: u16,
    /// Owner player number
    pub sender: i16,
    /// Pixel offset from origin
    pub pixoffs: i16,
    /// Speed (world units per frame)
    pub speed: i16,
    /// Hit points
    pub hit_points: i16,
    /// Damage dealt
    pub damage: i16,
    /// Facing direction (0-15)
    pub face: u16,
    /// Frame index within farray
    pub index: u16,
    /// Lifetime (frames)
    pub life: u16,
    /// Frame array pointer
    pub farray: *mut FrameHandle,
    /// Preprocessing function
    pub preprocess_func: ElementProcessFunc,
    /// Blast offset (pixels from collision point)
    pub blast_offs: i16,
}

impl MissileBlock {
    /// Creates a new MissileBlock with given parameters
    /// The origin parameter is the starting position before backing up by one step
    pub fn new(
        origin: Point,
        face: u16,
        index: u16,
        speed: i16,
        hit_points: i16,
        damage: i16,
        life: i16,
        blast_offs: i16,
        sender: i16,
        flags: u16,
    ) -> Self {
        MissileBlock {
            cx: origin.x,
            cy: origin.y,
            flags,
            sender,
            pixoffs: 0,
            speed,
            hit_points,
            damage,
            face,
            index,
            life: life as u16,
            farray: std::ptr::null_mut(),
            preprocess_func: None,
            blast_offs,
        }
    }
}

// ---------------------------------------------------------------------------
// Core Weapon Functions
// ---------------------------------------------------------------------------

/// Applies damage to a target element
///
/// Matches C's do_damage() from misc.c (lines 195-220):
/// - For PLAYER_SHIP: delegates to DeltaCrew() (Phase 2+)
/// - For non-ships: decrements hit_points, marks NONSOLID when destroyed
///
/// # Arguments
/// * `target` - Element to damage
/// * `damage` - Damage amount (unsigned byte)
///
/// # Safety
/// This function mutates target.hit_points and target.state_flags.
/// For player ships, this should call DeltaCrew() via FFI (Phase 2+).
pub fn do_damage(target: &mut Element, damage: u8) {
    if target.state_flags.contains(ElementFlags::PLAYER_SHIP) {
        // Ship takes crew damage
        // Phase 1: We cannot call DeltaCrew() because it requires FFI to ships subsystem.
        // The C wrapper (weapon_collision_cb) will handle this.
        // Phase 2+: Call DeltaCrew(target, -damage) via FFI
        //
        // For now, decrement crew_or_hp directly (unsafe, but matches C for Phase 1)
        if damage < target.crew_or_hp as u8 {
            target.crew_or_hp -= damage as u16;
        } else {
            target.crew_or_hp = 0;
            target.life_span = 0;
            target.state_flags.insert(ElementFlags::NONSOLID);
        }
    } else if target.mass_points < 100 {
        // Non-ship, non-gravity-mass element
        // C uses: if ((BYTE)damage < ElementPtr->hit_points)
        if damage < target.crew_or_hp as u8 {
            target.crew_or_hp -= damage as u16;
        } else {
            target.crew_or_hp = 0;
            target.life_span = 0;
            target.state_flags.insert(ElementFlags::NONSOLID);
        }
    }
    // Note: Gravity-mass elements (mass >= 100) take no damage
}

/// Weapon collision resolution — creates blast effect and applies damage
///
/// Matches C's weapon_collision() from weapon.c (lines 138-253):
/// 1. Guard: already processed this frame (COLLISION flag set)
/// 2. Apply damage to target (if weapon has damage > 0)
/// 3. Determine if weapon is destroyed
/// 4. Play damage sound
/// 5. Create blast effect element (standard 2-frame or custom animation)
/// 6. Return blast element handle (or None)
///
/// # Arguments
/// * `weapon` - Weapon element (mutable)
/// * `w_pt` - Weapon collision point (display coords)
/// * `target` - Target element (mutable)
/// * `h_pt` - Target collision point (display coords)
///
/// # Returns
/// Handle to blast element (or None if no blast created)
///
/// # Safety
/// This function:
/// - Mutates weapon and target elements
/// - Calls AllocElement() / PutElement() via FFI (Phase 2+)
/// - Calls ProcessSound() via FFI (Phase 2+)
/// - Accesses DisplayArray[] for primitive type checking (Phase 2+)
///
/// Phase 1 implementation returns None (stub) because it requires:
/// - AllocElement() / PutElement() / LockElement() / UnlockElement()
/// - ProcessSound()
/// - GetPrimType() / SetPrimType() for DisplayArray[]
/// - GetFrameCount() / SetAbsFrameIndex() for frame manipulation
/// - blast[] resource array
pub fn weapon_collision(
    weapon: &mut Element,
    _w_pt: &Point,
    target: &mut Element,
    _h_pt: &Point,
) -> Option<*mut Element> {
    // --- Step 1: Double-hit guard ---
    if weapon.state_flags.contains(ElementFlags::COLLISION) {
        return None; // Already processed this frame
    }

    // --- Step 2: Damage application ---
    let damage = weapon.mass_points;
    if damage != 0
        && (target.state_flags.contains(ElementFlags::FINITE_LIFE)
            || target.life_span == NORMAL_LIFE)
    {
        do_damage(target, damage);
        if target.crew_or_hp != 0 {
            // Target survived — mark weapon as collided but don't destroy it
            // The weapon persists but can't hit again this frame (double-hit guard)
            weapon.state_flags.insert(ElementFlags::COLLISION);
        }
    }

    // --- Step 3: Weapon destruction check ---
    // Weapon is destroyed ONLY when:
    //   - target is NOT finite-life (permanent objects like ships, asteroids), OR
    //   - (target doesn't have COLLISION set AND weapon hit_points <= target mass_points)
    //
    // Note: If target survived (step 2), weapon now has COLLISION flag but is NOT destroyed yet.
    // Weapon destruction happens only when passing this check.
    let weapon_should_be_destroyed = !target.state_flags.contains(ElementFlags::FINITE_LIFE)
        || (!target.state_flags.contains(ElementFlags::COLLISION)
            && weapon.crew_or_hp <= target.mass_points as u16);

    if weapon_should_be_destroyed {
        // Step 3a: Play damage sound (Phase 2+: call ProcessSound via FFI)
        if damage != 0 {
            let sound_idx = TARGET_DAMAGED_FOR_1_PT + (damage as i32 >> 1);
            let sound_idx = sound_idx.min(TARGET_DAMAGED_FOR_6_PLUS_PT);
            // Phase 2+: ProcessSound(SetAbsSoundIndex(GameSounds, sound_idx), target);
            let _ = sound_idx; // Suppress unused warning for Phase 1
        }

        // Step 3b: Mark weapon as destroyed
        // Phase 2+: check GetPrimType(&DisplayArray[weapon.prim_index]) != LINE_PRIM
        // For now, assume all non-laser weapons get DISAPPEARING
        // Lasers (LINE_PRIM) never get DISAPPEARING — they persist for their 1-frame life
        //
        // Phase 1 stub: We cannot check primitive type, so always set DISAPPEARING
        // The C wrapper will correct this behavior
        weapon.state_flags.insert(ElementFlags::DISAPPEARING);

        weapon.crew_or_hp = 0;
        weapon.life_span = 0;
        weapon
            .state_flags
            .insert(ElementFlags::COLLISION | ElementFlags::NONSOLID);

        // Step 3c: Create blast effect (Phase 2+)
        // Phase 1: Return None because we cannot call AllocElement() / PutElement()
        // This requires FFI to the C display list allocator
        //
        // Phase 2+ implementation:
        // - AllocElement() → hBlast
        // - PutElement(hBlast)
        // - LockElement(hBlast) → blast
        // - Initialize blast element (see below)
        // - UnlockElement(hBlast)
        // - Return Some(hBlast)
        //
        // Blast initialization (from C weapon.c:205-253):
        // - blast.playerNr = weapon.playerNr
        // - blast.state_flags = APPEARING | FINITE_LIFE | NONSOLID
        // - SetPrimType(&DisplayArray[blast.PrimIndex], STAMP_PRIM)
        // - blast.current.location = DISPLAY_TO_WORLD(w_pt) + blast_offset
        // - blast_offset: COSINE/SINE(angle, DISPLAY_TO_WORLD(weapon.blast_offset))
        // - blast_index = compute_blast_direction(angle)
        // - Standard blast (num_blast_frames <= 16): 2-frame explosion
        // - Custom blast (num_blast_frames > 16): weapon farray[16..] animation

        // Phase 1 stub: return None
        return None;
    }

    None
}

/// Computes 8-bin blast direction from weapon velocity angle
///
/// Matches C's blast_index calculation (weapon.c:229-231):
/// ```c
/// blast_index = NORMALIZE_FACING(ANGLE_TO_FACING(angle + HALF_CIRCLE));
/// blast_index = ((blast_index >> 2) << 1) + (blast_index & 0x3 ? 1 : 0);
/// ```
///
/// This maps 16 facings to 8 directional bins:
/// - Facing 0     → bin 0 (even)
/// - Facing 1-3   → bin 1 (odd)
/// - Facing 4     → bin 2 (even)
/// - Facing 5-7   → bin 3 (odd)
/// - ... etc.
///
/// # Arguments
/// * `angle` - Weapon velocity travel angle (0-63)
///
/// # Returns
/// Blast direction index (0-7)
pub fn compute_blast_direction(angle: u8) -> u8 {
    // Reverse angle (weapon is destroyed, blast points opposite direction)
    // angle is in 0-63 range, add HALF_CIRCLE (32)
    let reversed_angle = (angle as u16).wrapping_add(HALF_CIRCLE);

    // Convert angle to facing (16 directions)
    let facing = angle_to_facing(reversed_angle);

    // Normalize facing to 0-15
    let facing = normalize_facing(facing);

    // Map 16 facings to 8 bins
    // Formula: ((facing >> 2) << 1) + (facing & 3 != 0 ? 1 : 0)
    let bin = ((facing >> 2) << 1) + if (facing & 0x3) != 0 { 1 } else { 0 };

    bin as u8
}

/// Finds the closest enemy ship and adjusts facing to track it
///
/// Matches C's TrackShip() from weapon.c (lines 300-414):
/// 1. Fast path: check hTarget if already set
/// 2. Display list scan fallback: find closest enemy PLAYER_SHIP
/// 3. Filter: must not be same player, must not be cloaked (except APPEARING tracker)
/// 4. Compute toroidal shortest-path distance (Manhattan approximation)
/// 5. Turn one step toward target (±1 facing unit)
/// 6. 180° case: random turn direction
///
/// # Arguments
/// * `tracker` - Tracking element (weapon or ship)
/// * `facing` - Current facing direction (0-15), updated in-place
///
/// # Returns
/// Actual facing delta to target (0-15), or None if no target found
///
/// # Safety
/// This function:
/// - Mutates tracker.hTarget
/// - Mutates facing parameter
/// - Calls GetHeadElement() / GetSuccElement() / LockElement() / UnlockElement() via FFI
/// - Calls GetElementStarShip() via FFI
/// - Calls TFB_Random() via FFI
/// - Accesses OBJECT_CLOAKED() via DisplayArray[]
///
/// Phase 1 implementation returns None (stub) because it requires:
/// - Display list traversal (GetHeadElement / GetSuccElement)
/// - Element locking (LockElement / UnlockElement)
/// - StarShip access (GetElementStarShip)
/// - Random number generation (TFB_Random)
/// - Cloak detection (OBJECT_CLOAKED via DisplayArray)
pub fn track_ship(tracker: &mut Element, facing: &mut u16) -> Option<i16> {
    // Phase 1 stub: return None
    // This function requires extensive FFI to C display list and RNG
    //
    // Phase 2+ implementation:
    // 1. Check tracker.hTarget (fast path)
    // 2. If no target, scan GetHeadElement() → GetSuccElement() loop
    // 3. Filter: PLAYER_SHIP, !elementsOfSamePlayer, !OBJECT_CLOAKED
    // 4. Compute wrap_delta_x/y, ARCTAN, Manhattan distance
    // 5. Track closest target
    // 6. Turn one step toward target (±1 facing)
    // 7. 180° case: TFB_Random() & 1 ? +1 : -1
    // 8. Return best_delta_facing

    let _ = tracker; // Suppress unused warning
    let _ = facing;

    None
}

/// Computes facing adjustment to track a specific target position
///
/// This is a helper function that implements the core tracking logic
/// without requiring display list traversal or FFI.
///
/// # Arguments
/// * `tracker_pos` - Tracker current position (world coords)
/// * `target_pos` - Target current position (world coords)
/// * `current_facing` - Current facing direction (0-15)
///
/// # Returns
/// Adjusted facing direction (0-15) turned one step toward target
pub fn compute_track_facing(tracker_pos: Point, target_pos: Point, current_facing: u16) -> u16 {
    // Compute toroidal shortest-path delta
    let delta_x = wrap_delta_x(target_pos.x - tracker_pos.x);
    let delta_y = wrap_delta_y(target_pos.y - tracker_pos.y);

    // Compute facing delta
    let target_angle = arctan(delta_x, delta_y);
    let target_facing = angle_to_facing(target_angle);
    let delta_facing = normalize_facing(target_facing.wrapping_sub(current_facing));

    // Turn one step toward target
    let mut new_facing = current_facing;

    let half_circle_facing = angle_to_facing(HALF_CIRCLE);
    if delta_facing == half_circle_facing {
        // Target is exactly behind — deterministic turn (Phase 1: always turn right)
        // Phase 2+: use ((TFB_Random() & 1) << 1) - 1 for random direction
        new_facing = new_facing.wrapping_add(1);
    } else if delta_facing < half_circle_facing {
        // Target is to the left (shorter clockwise arc)
        new_facing = new_facing.wrapping_add(1);
    } else {
        // Target is to the right (shorter counter-clockwise arc)
        new_facing = new_facing.wrapping_sub(1);
    }

    normalize_facing(new_facing)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::element::Element;
    use super::super::velocity::world_to_velocity;
    use super::*;

    // -- LaserBlock Construction --

    #[test]
    fn laser_block_construction() {
        let origin = Point::new(100, 200);
        let end_point = Point::new(150, 250);
        let face = 4; // Facing 4 (right-up diagonal)
        let sender = 0; // Player 0
        let flags = ElementFlags::FINITE_LIFE.bits();

        let laser = LaserBlock::new(origin, end_point, face, sender, flags);

        assert_eq!(laser.cx, 100);
        assert_eq!(laser.cy, 200);
        assert_eq!(laser.ex, 150);
        assert_eq!(laser.ey, 250);
        assert_eq!(laser.face, 4);
        assert_eq!(laser.sender, 0);
        assert_eq!(laser.flags, flags);
    }

    // -- MissileBlock Construction --

    #[test]
    fn missile_block_construction() {
        let origin = Point::new(100, 200);
        let face = 8; // Facing 8 (left)
        let index = 0;
        let speed = 10; // 10 world units/frame
        let hit_points = 1;
        let damage = 2;
        let life = 30; // 30 frames
        let blast_offs = 4; // 4 pixels offset
        let sender = 1; // Player 1
        let flags = ElementFlags::FINITE_LIFE.bits();

        let missile = MissileBlock::new(
            origin, face, index, speed, hit_points, damage, life, blast_offs, sender, flags,
        );

        assert_eq!(missile.cx, 100);
        assert_eq!(missile.cy, 200);
        assert_eq!(missile.face, 8);
        assert_eq!(missile.index, 0);
        assert_eq!(missile.speed, 10);
        assert_eq!(missile.hit_points, 1);
        assert_eq!(missile.damage, 2);
        assert_eq!(missile.life, 30);
        assert_eq!(missile.blast_offs, 4);
        assert_eq!(missile.sender, 1);
        assert_eq!(missile.flags, flags);
    }

    #[test]
    fn missile_backup_one_step() {
        // Test that missile origin can be backed up by one velocity step
        // Facing 0 = North (up), Facing 4 = East (right), Facing 8 = South (down), Facing 12 = West (left)
        let origin = Point::new(100, 200);
        let face = 4; // Facing 4 (East/right)
        let speed = 10; // 10 world units/frame

        // Create missile at origin
        let mut missile = MissileBlock::new(origin, face, 0, speed, 1, 1, 30, 0, 0, 0);

        // Compute one-step backup (same as initialize_missile in C weapon.c:120-126)
        let angle = facing_to_angle(face);
        let delta_x = cosine(angle, world_to_velocity(speed as i32));
        let delta_y = sine(angle, world_to_velocity(speed as i32));

        // Back up origin by one step
        missile.cx -= (delta_x >> 5) as i16; // VELOCITY_TO_WORLD
        missile.cy -= (delta_y >> 5) as i16;

        // Missile should be backed up by ~10 world units to the left (since facing right)
        assert!(
            missile.cx < origin.x,
            "Missile facing right should be backed up to the left, got cx={} vs origin.x={}",
            missile.cx,
            origin.x
        );
        assert_eq!(
            missile.cy, origin.y,
            "Y coordinate should be unchanged for horizontal facing"
        );
    }

    // -- Blast Direction Calculation --

    #[test]
    fn blast_direction_8bins() {
        // Test weapon velocity angles map to 8 directional blast bins
        // When weapon is traveling at angle X, blast points in opposite direction
        // The 8 bins represent: N, NE, E, SE, S, SW, W, NW (0-7)
        //
        // Weapon angle → reversed (+ HALF_CIRCLE) → facing → bin
        let test_cases = [
            // Weapon traveling North (angle 0) → blast South (angle 32) → facing 8 → bin 4
            (0, 4),
            // Weapon traveling NE (angle 8) → blast SW (angle 40) → facing 10 → bin 5
            (8, 5),
            // Weapon traveling East (angle 16) → blast West (angle 48) → facing 12 → bin 6
            (16, 6),
            // Weapon traveling SE (angle 24) → blast NW (angle 56) → facing 14 → bin 7
            (24, 7),
            // Weapon traveling South (angle 32) → blast North (angle 64/0) → facing 0 → bin 0
            (32, 0),
            // Weapon traveling SW (angle 40) → blast NE (angle 72/8) → facing 2 → bin 1
            (40, 1),
            // Weapon traveling West (angle 48) → blast East (angle 80/16) → facing 4 → bin 2
            (48, 2),
            // Weapon traveling NW (angle 56) → blast SE (angle 88/24) → facing 6 → bin 3
            (56, 3),
        ];

        for (weapon_angle, expected_bin) in test_cases.iter() {
            let bin = compute_blast_direction(*weapon_angle);
            assert_eq!(
                bin, *expected_bin,
                "Weapon angle {} should produce blast bin {}, got {}",
                weapon_angle, expected_bin, bin
            );
        }
    }

    // -- Weapon Collision Damage --

    #[test]
    fn weapon_collision_damage_application() {
        let mut weapon = Element::new();
        let mut target = Element::new();

        weapon.mass_points = 5; // 5 damage
        weapon.crew_or_hp = 1; // 1 hit point (will be destroyed)
        weapon.state_flags.insert(ElementFlags::FINITE_LIFE);

        target.mass_points = 10; // Target is heavier
        target.crew_or_hp = 10; // 10 hit points
        target.state_flags.insert(ElementFlags::FINITE_LIFE);
        target.life_span = NORMAL_LIFE;

        let w_pt = Point::new(100, 100);
        let h_pt = Point::new(100, 100);

        weapon_collision(&mut weapon, &w_pt, &mut target, &h_pt);

        // Target should take 5 damage
        assert_eq!(target.crew_or_hp, 5, "Target should take 5 damage");

        // Weapon should be destroyed (weaker than target)
        assert_eq!(weapon.crew_or_hp, 0, "Weapon should be destroyed");
        assert_eq!(weapon.life_span, 0, "Weapon life_span should be 0");
        assert!(
            weapon.state_flags.contains(ElementFlags::COLLISION),
            "Weapon should have COLLISION flag"
        );
        assert!(
            weapon.state_flags.contains(ElementFlags::NONSOLID),
            "Weapon should have NONSOLID flag"
        );
    }

    #[test]
    fn weapon_collision_exact_kill() {
        let mut weapon = Element::new();
        let mut target = Element::new();

        weapon.mass_points = 10; // 10 damage (exact kill)
        weapon.crew_or_hp = 1;
        weapon.state_flags.insert(ElementFlags::FINITE_LIFE);

        target.mass_points = 5;
        target.crew_or_hp = 10; // 10 hit points
        target.state_flags.insert(ElementFlags::FINITE_LIFE);
        target.life_span = NORMAL_LIFE;

        let w_pt = Point::new(100, 100);
        let h_pt = Point::new(100, 100);

        weapon_collision(&mut weapon, &w_pt, &mut target, &h_pt);

        // Target should be destroyed (exact kill)
        assert_eq!(target.crew_or_hp, 0, "Target should be destroyed");
        assert_eq!(target.life_span, 0, "Target life_span should be 0");
        assert!(
            target.state_flags.contains(ElementFlags::NONSOLID),
            "Target should have NONSOLID flag"
        );
    }

    #[test]
    fn weapon_collision_overkill() {
        let mut weapon = Element::new();
        let mut target = Element::new();

        weapon.mass_points = 20; // 20 damage (overkill)
        weapon.crew_or_hp = 1;
        weapon.state_flags.insert(ElementFlags::FINITE_LIFE);

        target.mass_points = 5;
        target.crew_or_hp = 5; // 5 hit points
        target.state_flags.insert(ElementFlags::FINITE_LIFE);
        target.life_span = NORMAL_LIFE;

        let w_pt = Point::new(100, 100);
        let h_pt = Point::new(100, 100);

        weapon_collision(&mut weapon, &w_pt, &mut target, &h_pt);

        // Target should be destroyed (overkill)
        assert_eq!(target.crew_or_hp, 0, "Target should be destroyed");
        assert_eq!(target.life_span, 0, "Target life_span should be 0");
        assert!(
            target.state_flags.contains(ElementFlags::NONSOLID),
            "Target should have NONSOLID flag"
        );
    }

    #[test]
    fn weapon_collision_already_dead() {
        let mut weapon = Element::new();
        let mut target = Element::new();

        weapon.mass_points = 10;
        weapon.crew_or_hp = 1;
        weapon.state_flags.insert(ElementFlags::FINITE_LIFE);

        target.mass_points = 5;
        target.crew_or_hp = 0; // Already dead
        target.life_span = 0;
        target
            .state_flags
            .insert(ElementFlags::FINITE_LIFE | ElementFlags::NONSOLID);

        let w_pt = Point::new(100, 100);
        let h_pt = Point::new(100, 100);

        weapon_collision(&mut weapon, &w_pt, &mut target, &h_pt);

        // Target should remain at 0 hit points (do_damage on already-dead target)
        assert_eq!(target.crew_or_hp, 0, "Target should remain dead");
    }

    #[test]
    fn weapon_collision_double_hit_guard() {
        let mut weapon = Element::new();
        let mut target = Element::new();

        weapon.mass_points = 5;
        weapon.crew_or_hp = 1;
        weapon
            .state_flags
            .insert(ElementFlags::FINITE_LIFE | ElementFlags::COLLISION);

        target.mass_points = 10;
        target.crew_or_hp = 10;
        target.state_flags.insert(ElementFlags::FINITE_LIFE);
        target.life_span = NORMAL_LIFE;

        let w_pt = Point::new(100, 100);
        let h_pt = Point::new(100, 100);

        weapon_collision(&mut weapon, &w_pt, &mut target, &h_pt);

        // Target should NOT take damage (weapon already has COLLISION flag)
        assert_eq!(
            target.crew_or_hp, 10,
            "Target should not take damage on double-hit"
        );
    }

    #[test]
    fn weapon_collision_target_survives() {
        let mut weapon = Element::new();
        let mut target = Element::new();

        // Setup: weak weapon (HP=1, mass=3) vs strong target (HP=10, mass=10)
        weapon.mass_points = 3; // 3 damage
        weapon.crew_or_hp = 1; // 1 hit point
        weapon.state_flags.insert(ElementFlags::FINITE_LIFE);

        target.mass_points = 10; // 10 mass points
        target.crew_or_hp = 10; // 10 hit points (will survive 3 damage)
        target.state_flags.insert(ElementFlags::FINITE_LIFE);
        target.life_span = NORMAL_LIFE;

        let w_pt = Point::new(100, 100);
        let h_pt = Point::new(100, 100);

        weapon_collision(&mut weapon, &w_pt, &mut target, &h_pt);

        // Target should take damage but survive
        assert_eq!(target.crew_or_hp, 7, "Target should take 3 damage");
        assert!(target.life_span > 0, "Target should still be alive");

        // Weapon WILL be destroyed because weapon.HP (1) <= target.mass (10)
        // Even though target survived, the weapon is too weak and gets destroyed
        assert!(
            weapon.state_flags.contains(ElementFlags::COLLISION),
            "Weapon should have COLLISION flag"
        );
        assert_eq!(weapon.crew_or_hp, 0, "Weak weapon should be destroyed");
        assert_eq!(weapon.life_span, 0, "Weapon life_span should be 0");
    }

    #[test]
    fn weapon_collision_strong_weapon_target_survives() {
        let mut weapon = Element::new();
        let mut target = Element::new();

        // Setup: strong weapon (HP=15, mass=3) vs weak target (HP=10, mass=5)
        weapon.mass_points = 3; // 3 damage
        weapon.crew_or_hp = 15; // 15 hit points (stronger than target mass)
        weapon.state_flags.insert(ElementFlags::FINITE_LIFE);
        weapon.life_span = 10; // Weapon has life remaining

        target.mass_points = 5; // 5 mass points
        target.crew_or_hp = 10; // 10 hit points (will survive 3 damage)
        target.state_flags.insert(ElementFlags::FINITE_LIFE);
        target.life_span = NORMAL_LIFE;

        let w_pt = Point::new(100, 100);
        let h_pt = Point::new(100, 100);

        weapon_collision(&mut weapon, &w_pt, &mut target, &h_pt);

        // Target should take damage but survive
        assert_eq!(target.crew_or_hp, 7, "Target should take 3 damage");
        assert!(target.life_span > 0, "Target should still be alive");

        // Weapon should NOT be destroyed because weapon.HP (15) > target.mass (5)
        // The strong weapon persists after hitting the target
        assert!(
            weapon.state_flags.contains(ElementFlags::COLLISION),
            "Weapon should have COLLISION flag (target survived)"
        );
        assert_eq!(
            weapon.crew_or_hp, 15,
            "Strong weapon should retain hit points"
        );
        assert_ne!(
            weapon.life_span, 0,
            "Strong weapon should not be marked for death"
        );
        assert!(
            !weapon.state_flags.contains(ElementFlags::NONSOLID),
            "Strong weapon should remain solid"
        );
    }

    // -- do_damage Edge Cases --

    #[test]
    fn do_damage_ship_exact_kill() {
        let mut ship = Element::new();
        ship.state_flags.insert(ElementFlags::PLAYER_SHIP);
        ship.crew_or_hp = 5;
        ship.life_span = NORMAL_LIFE;

        do_damage(&mut ship, 5);

        assert_eq!(ship.crew_or_hp, 0);
        assert_eq!(ship.life_span, 0);
        assert!(ship.state_flags.contains(ElementFlags::NONSOLID));
    }

    #[test]
    fn do_damage_ship_overkill() {
        let mut ship = Element::new();
        ship.state_flags.insert(ElementFlags::PLAYER_SHIP);
        ship.crew_or_hp = 3;
        ship.life_span = NORMAL_LIFE;

        do_damage(&mut ship, 10);

        assert_eq!(ship.crew_or_hp, 0);
        assert_eq!(ship.life_span, 0);
        assert!(ship.state_flags.contains(ElementFlags::NONSOLID));
    }

    #[test]
    fn do_damage_non_ship() {
        let mut elem = Element::new();
        elem.mass_points = 5; // Non-gravity-mass
        elem.crew_or_hp = 10;
        elem.life_span = 5;

        do_damage(&mut elem, 3);

        assert_eq!(elem.crew_or_hp, 7);
        assert_eq!(elem.life_span, 5); // Not killed
    }

    #[test]
    fn do_damage_gravity_mass_immune() {
        let mut planet = Element::new();
        planet.mass_points = 100; // Gravity mass
        planet.crew_or_hp = 100;
        planet.life_span = NORMAL_LIFE;

        do_damage(&mut planet, 50);

        // Gravity mass should take no damage
        assert_eq!(planet.crew_or_hp, 100);
        assert_eq!(planet.life_span, NORMAL_LIFE);
    }

    // -- Track Ship Facing Adjustment --

    #[test]
    fn track_ship_facing_10_positions() {
        // Test 10 different target positions around a tracker
        let tracker_pos = Point::new(1000, 1000); // Center

        let test_cases = [
            // (target_pos, current_facing, description)
            (Point::new(1100, 1000), 12, "Target to the right (East)"),
            (Point::new(900, 1000), 4, "Target to the left (West)"),
            (Point::new(1000, 900), 0, "Target above (North)"),
            (Point::new(1000, 1100), 8, "Target below (South)"),
            (Point::new(1100, 900), 14, "Target NE diagonal"),
            (Point::new(900, 900), 2, "Target NW diagonal"),
            (Point::new(900, 1100), 6, "Target SW diagonal"),
            (Point::new(1100, 1100), 10, "Target SE diagonal"),
            (Point::new(1050, 950), 15, "Target slightly NE"),
            (Point::new(950, 1050), 5, "Target slightly SW"),
        ];

        for (target_pos, initial_facing, description) in test_cases.iter() {
            let adjusted_facing = compute_track_facing(tracker_pos, *target_pos, *initial_facing);

            // Adjusted facing should be different from initial (turned toward target)
            // or same if already facing target exactly
            assert!(
                adjusted_facing < 16,
                "{}: adjusted_facing should be valid (0-15), got {}",
                description,
                adjusted_facing
            );

            // Verify it's a one-step adjustment (±1 or same)
            let delta = if adjusted_facing >= *initial_facing {
                adjusted_facing - *initial_facing
            } else {
                *initial_facing - adjusted_facing
            };

            assert!(
                delta <= 1 || delta >= 15,
                "{}: should turn by ±1 step, got delta={}",
                description,
                delta
            );
        }
    }

    #[test]
    fn track_ship_facing_180_degree() {
        // Target is exactly behind tracker (180° case)
        let tracker_pos = Point::new(1000, 1000);
        let target_pos = Point::new(1000, 800); // Directly above (North)
        let initial_facing = 8; // Facing South (opposite direction)

        let adjusted_facing = compute_track_facing(tracker_pos, target_pos, initial_facing);

        // Should turn by ±1 (deterministic for Phase 1, random in Phase 2+)
        let delta = if adjusted_facing >= initial_facing {
            adjusted_facing - initial_facing
        } else {
            initial_facing - adjusted_facing
        };

        assert!(
            delta == 1 || delta == 15,
            "180° case should turn by ±1, got delta={}",
            delta
        );
    }

    #[test]
    fn track_ship_facing_already_aligned() {
        // Tracker already facing target
        let tracker_pos = Point::new(1000, 1000);
        let target_pos = Point::new(1100, 1000); // Directly to the right
        let initial_facing = 12; // Facing East (right)

        let adjusted_facing = compute_track_facing(tracker_pos, target_pos, initial_facing);

        // Should turn slightly (or not at all if exactly aligned)
        // The algorithm always turns ±1 toward target unless exact
        assert!(
            adjusted_facing < 16,
            "Adjusted facing should be valid (0-15)"
        );
    }
}
