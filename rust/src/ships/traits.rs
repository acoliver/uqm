// Ship Behavior Trait & Supporting Types
// @plan PLAN-20260314-SHIPS.P04
// @requirement REQ-HOOKS-REGISTRATION, REQ-AI-HOOK, REQ-NULL-HOOK-NOOP, REQ-HOOK-CHANGE

use super::types::{RaceDescTemplate, StatusFlags};

// ---------------------------------------------------------------------------
// ShipState  (mutable view into ship runtime state for behavior hooks)
// ---------------------------------------------------------------------------

/// Mutable view into the ship's runtime state, passed to behavior hooks.
///
/// This decouples race-specific behavior code from the full `Starship` /
/// `Element` representation.  Fields mirror the subset of `Starship` +
/// `Element` state that race hooks need to read or mutate each frame.
#[derive(Debug)]
pub struct ShipState {
    /// Current crew count.
    pub crew_level: u16,
    pub max_crew: u16,
    /// Current energy count.
    pub energy_level: u16,
    pub max_energy: u16,
    /// Current facing direction (0..15 for 16-way rotation).
    pub ship_facing: u8,
    /// Current status flags (thrust, weapon, special, etc.).
    pub cur_status_flags: StatusFlags,
    pub old_status_flags: StatusFlags,
    /// Player number (0 = bottom/RPG, 1 = top/NPC).
    pub player_nr: i16,
    /// Position in world coordinates.
    pub position: (i32, i32),
    /// Velocity in world coordinates.
    pub velocity: (i32, i32),

    // --- Fields needed for C FFI calls from ship behaviors ---

    /// Opaque C ELEMENT pointer. Ship behaviors pass this to battle_bridge
    /// functions (DeltaCrew, ProcessSound, etc.). Null in tests.
    pub element_ptr: *mut std::os::raw::c_void,
    /// Opaque C STARSHIP pointer. Null in tests.
    pub starship_ptr: *mut std::os::raw::c_void,
    /// Weapon cooldown counter.
    pub weapon_counter: u8,
    /// Special cooldown counter.
    pub special_counter: u8,
    /// Energy cooldown counter.
    pub energy_counter: u8,
    /// Ship input state (raw bitfield from C).
    pub ship_input_state: u8,
    /// Thrust wait counter from element (frames until next thrust).
    pub thrust_wait: u8,
    /// Turn wait counter from element (frames until next turn).
    pub turn_wait: u8,
    /// Ship sounds resource handle (for ProcessSound calls).
    pub ship_sounds: usize,
    /// Weapon animation frames array pointer (for MissileBlock.farray).
    pub weapon_farray: *mut std::os::raw::c_void,
    /// Special animation frames array pointer.
    pub special_farray: *mut std::os::raw::c_void,
}

// Safety: ShipState contains raw pointers that are never dereferenced in Rust
// without unsafe. They are only passed through to C bridge calls.
unsafe impl Send for ShipState {}

impl Default for ShipState {
    fn default() -> Self {
        Self {
            crew_level: 0,
            max_crew: 0,
            energy_level: 0,
            max_energy: 0,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 0,
            position: (0, 0),
            velocity: (0, 0),
            element_ptr: std::ptr::null_mut(),
            starship_ptr: std::ptr::null_mut(),
            weapon_counter: 0,
            special_counter: 0,
            energy_counter: 0,
            ship_input_state: 0,
            thrust_wait: 0,
            turn_wait: 0,
            ship_sounds: 0,
            weapon_farray: std::ptr::null_mut(),
            special_farray: std::ptr::null_mut(),
        }
    }
}

impl ShipState {
    /// Creates a ShipState with test defaults (null pointers, zero counters).
    #[cfg(test)]
    pub fn test_new(
        crew_level: u16,
        max_crew: u16,
        energy_level: u16,
        max_energy: u16,
        ship_facing: u8,
        player_nr: i16,
    ) -> Self {
        Self {
            crew_level,
            max_crew,
            energy_level,
            max_energy,
            ship_facing,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr,
            position: (0, 0),
            velocity: (0, 0),
            element_ptr: std::ptr::null_mut(),
            starship_ptr: std::ptr::null_mut(),
            weapon_counter: 0,
            special_counter: 0,
            energy_counter: 0,
            ship_input_state: 0,
            thrust_wait: 0,
            turn_wait: 0,
            ship_sounds: 0,
            weapon_farray: std::ptr::null_mut(),
            special_farray: std::ptr::null_mut(),
        }
    }
}

// ---------------------------------------------------------------------------
// BattleContext  (read-only battle environment context)
// ---------------------------------------------------------------------------

/// Read-only snapshot of the battle environment, passed to behavior hooks.
///
/// Provides the environmental information race hooks need to make decisions
/// (opponent state, gravity, battle bounds, etc.) without granting mutable
/// access to global battle state.
#[derive(Debug)]
pub struct BattleContext {
    /// Whether the battle is in hyperspace mode.
    pub hyperspace: bool,
    /// Current battle frame counter.
    pub frame_count: u32,
    /// Gravity well center, if present.
    pub gravity_center: Option<(i32, i32)>,
}

// ---------------------------------------------------------------------------
// WeaponElement  (weapon/projectile returned by init_weapon)
// ---------------------------------------------------------------------------

/// Describes a weapon or projectile to be spawned by `init_weapon`.
///
/// The battle engine uses these to create Element entries on the display list.
/// Fields correspond to the C weapon element setup pattern (position, facing,
/// image frame, life span, damage, etc.).
#[derive(Debug, Clone)]
pub struct WeaponElement {
    /// Offset from ship center.
    pub offset: (i32, i32),
    /// Facing direction (0..15).
    pub facing: u8,
    /// Velocity.
    pub velocity: (i32, i32),
    /// Frames the projectile lives.
    pub life_span: u16,
    /// Hit points the projectile can absorb before expiring.
    pub hit_points: u16,
    /// Damage dealt on collision.
    pub damage: u16,
    /// Mass for collision physics.
    pub mass: u8,
}

// ---------------------------------------------------------------------------
// CollisionHandler  (callback type for collision override)
// ---------------------------------------------------------------------------

/// Collision handler function pointer type.
///
/// If a race provides a collision override via `ShipBehavior::collision_override()`,
/// it replaces the default ship-to-ship collision logic.  The closure receives
/// the ship's element handle and the colliding element handle.
pub type CollisionHandler = Box<dyn Fn(usize, usize) + Send>;

// ---------------------------------------------------------------------------
// ShipBehavior  (the full trait — expanded from the P03 minimal definition)
// ---------------------------------------------------------------------------

/// Race-specific ship behavior hooks.
///
/// Each species implements this trait to provide its descriptor template
/// and combat callbacks.  Default implementations are safe no-ops, allowing
/// metadata-only descriptors to function without panic.
///
/// # Hook dispatch order (per frame)
/// 1. `preprocess` — called during the element preprocess pass
/// 2. `postprocess` — called during the element postprocess pass
/// 3. `init_weapon` — called when the weapon button fires successfully
/// 4. `intelligence` — called each frame to compute AI input flags
///
/// # Lifecycle
/// - `uninit` — called when the descriptor is being freed
pub trait ShipBehavior: std::fmt::Debug + Send {
    /// Returns the static descriptor template for this species.
    fn descriptor_template(&self) -> RaceDescTemplate;

    /// Per-frame preprocess hook (called before energy/turn/thrust processing).
    ///
    /// Default: no-op.
    fn preprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), super::types::ShipsError> {
        Ok(())
    }

    /// Per-frame postprocess hook (called after weapon/special handling).
    ///
    /// Default: no-op.
    fn postprocess(
        &mut self,
        _ship: &mut ShipState,
        _ctx: &BattleContext,
    ) -> Result<(), super::types::ShipsError> {
        Ok(())
    }

    /// Spawn weapon elements when the weapon fires successfully.
    ///
    /// Returns a vec of weapon element descriptors for the battle engine
    /// to place on the display list.
    ///
    /// Default: empty (no weapon).
    fn init_weapon(
        &mut self,
        _ship: &ShipState,
        _ctx: &BattleContext,
    ) -> Result<Vec<WeaponElement>, super::types::ShipsError> {
        Ok(vec![])
    }

    /// Compute AI input flags for this frame.
    ///
    /// Called each frame by the battle engine's AI system.  Returns the
    /// status flags the AI "presses" (THRUST, LEFT, RIGHT, WEAPON, SPECIAL).
    ///
    /// Default: empty flags (no input).
    fn intelligence(&mut self, _ship: &ShipState, _ctx: &BattleContext) -> StatusFlags {
        StatusFlags::empty()
    }

    /// Teardown hook called when the descriptor is being freed.
    ///
    /// Default: no-op.
    fn uninit(&mut self) {}

    /// Optional collision handler override.
    ///
    /// If `Some`, replaces the default ship collision function for this ship.
    ///
    /// Default: `None` (use standard collision).
    fn collision_override(&self) -> Option<CollisionHandler> {
        None
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ships::types::{
        Characteristics, FleetStuff, IntelStuff, RaceDescTemplate, ShipData, ShipFlags, ShipInfo,
    };

    /// Minimal test behavior that exercises all default methods.
    #[derive(Debug)]
    struct TestBehavior;

    impl ShipBehavior for TestBehavior {
        fn descriptor_template(&self) -> RaceDescTemplate {
            RaceDescTemplate {
                ship_info: ShipInfo::default(),
                fleet: FleetStuff::default(),
                characteristics: Characteristics::default(),
                ship_data: ShipData::default(),
                intel: IntelStuff::default(),
            }
        }
    }

    fn make_ship_state() -> ShipState {
        ShipState {
            crew_level: 10,
            max_crew: 20,
            energy_level: 15,
            max_energy: 30,
            ship_facing: 0,
            cur_status_flags: StatusFlags::empty(),
            old_status_flags: StatusFlags::empty(),
            player_nr: 0,
            position: (100, 200),
            velocity: (0, 0),
            element_ptr: std::ptr::null_mut(),
            starship_ptr: std::ptr::null_mut(),
            weapon_counter: 0,
            special_counter: 0,
            energy_counter: 0,
            ship_input_state: 0,
            thrust_wait: 0,
            turn_wait: 0,
            ship_sounds: 0,
            weapon_farray: std::ptr::null_mut(),
            special_farray: std::ptr::null_mut(),
        }
    }

    fn make_battle_ctx() -> BattleContext {
        BattleContext {
            hyperspace: false,
            frame_count: 0,
            gravity_center: None,
        }
    }

    #[test]
    fn default_preprocess_is_noop() {
        let mut b = TestBehavior;
        let mut state = make_ship_state();
        let ctx = make_battle_ctx();
        assert!(b.preprocess(&mut state, &ctx).is_ok());
    }

    #[test]
    fn default_postprocess_is_noop() {
        let mut b = TestBehavior;
        let mut state = make_ship_state();
        let ctx = make_battle_ctx();
        assert!(b.postprocess(&mut state, &ctx).is_ok());
    }

    #[test]
    fn default_init_weapon_returns_empty() {
        let mut b = TestBehavior;
        let state = make_ship_state();
        let ctx = make_battle_ctx();
        let weapons = b.init_weapon(&state, &ctx).unwrap();
        assert!(weapons.is_empty());
    }

    #[test]
    fn default_intelligence_returns_empty_flags() {
        let mut b = TestBehavior;
        let state = make_ship_state();
        let ctx = make_battle_ctx();
        let flags = b.intelligence(&state, &ctx);
        assert!(flags.is_empty());
    }

    #[test]
    fn default_uninit_is_noop() {
        let mut b = TestBehavior;
        b.uninit(); // Should not panic
    }

    #[test]
    fn default_collision_override_is_none() {
        let b = TestBehavior;
        assert!(b.collision_override().is_none());
    }

    #[test]
    fn trait_object_safety() {
        let b: Box<dyn ShipBehavior> = Box::new(TestBehavior);
        let _template = b.descriptor_template();
        // Trait is object-safe: can be stored in Box<dyn ShipBehavior>
    }

    #[test]
    fn ship_state_debug_format() {
        let state = make_ship_state();
        let debug = format!("{:?}", state);
        assert!(debug.contains("crew_level"));
    }

    #[test]
    fn battle_context_debug_format() {
        let ctx = make_battle_ctx();
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("hyperspace"));
    }

    #[test]
    fn weapon_element_clone() {
        let w = WeaponElement {
            offset: (10, 20),
            facing: 3,
            velocity: (5, -5),
            life_span: 60,
            hit_points: 1,
            damage: 4,
            mass: 2,
        };
        let w2 = w.clone();
        assert_eq!(w2.damage, 4);
        assert_eq!(w2.life_span, 60);
    }

    #[test]
    fn ship_flags_query_from_template() {
        let b = TestBehavior;
        let t = b.descriptor_template();
        assert_eq!(t.ship_info.ship_flags, ShipFlags::empty());
    }
}
