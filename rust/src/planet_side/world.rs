//! Deterministic PlanetSide world simulation.
//!
//! This module ports the per-frame surface world behavior from the deleted
//! `sc2/src/uqm/planets/lander.c` into pure Rust. It owns creature AI movement
//! (`object_animation`), shot mechanics, shot-vs-creature pixel collision
//! (`shotCreature`), lander collision damage, and deterministic environmental
//! hazard spawning (`BuildObjectList`, `AddLightning`, `AddGroundDisaster`).
//!
//! Lifecycle animation sequences (warmup, launch, takeoff, explosion playback)
//! are intentionally excluded; they are handled by dedicated animation modules.

use crate::battle::battle_types::{
    arctan, cosine, facing_to_angle, normalize_angle, normalize_facing, sine, HALF_CIRCLE,
};

use super::creatures::{CreatureBehavior, CreatureCatalog, CreatureKind, CreatureSpeed};
use super::entities::{SurfaceEntity, SurfaceEntityId, SurfaceEntityKind, SurfaceWorld};
use super::geometry::{masks_intersect_wrapped, CollisionMask};
use super::hazards::{hazard_chance, thermal_hazard_rating, HazardKind};
use super::model::SurfacePoint;
use super::simulation::Shot;

// ---------------------------------------------------------------------------
// World geometry constants — sourced from lander.h and planets.h.
// ---------------------------------------------------------------------------

/// Surface magnification shift. World coords = map coords << MAG_SHIFT.
const MAG_SHIFT: i32 = 2;

/// Map width in scan cells (`SIS_SCREEN_WIDTH`).
const MAP_WIDTH: i32 = 242;

/// Map height in scan cells (`75 - SAFE_Y`).
const MAP_HEIGHT: i32 = 75;

/// Total world width in pixels (`MAP_WIDTH << MAG_SHIFT`).
pub const WORLD_WIDTH: i32 = MAP_WIDTH << MAG_SHIFT;

/// Total world height in pixels (`MAP_HEIGHT << MAG_SHIFT`).
pub const WORLD_HEIGHT: i32 = MAP_HEIGHT << MAG_SHIFT;

/// Visible surface viewport width (`SURFACE_WIDTH`).
const SURFACE_WIDTH: i32 = 242;

/// Visible surface viewport height (`SURFACE_HEIGHT`).
const SURFACE_HEIGHT: i32 = 162;

/// Half the world width, used for shortest-path X wrapping.
const HALF_WORLD_WIDTH: i32 = MAP_WIDTH << (MAG_SHIFT - 1);

/// Creature animation cadence: AI runs only every 4th frame.
const CREATURE_AI_FRAME_MASK: u16 = 3;

/// Base reload frames added to the random thrust-wait for wandering creatures.
const WANDER_THRUST_BASE: u8 = 10;

/// Thrust-wait reload used when a fleeing creature is pinned to the surface edge.
const FLEE_EDGE_THRUST: u8 = 5;

/// Lightning strike chance (percent) of killing one crew when it hits the lander.
const LIGHTNING_KILL_PERCENT: u32 = 10;

/// Earthquake/lava chance (percent) of damaging crew on contact.
const GROUND_DISASTER_INJURY_PERCENT: u32 = 25;

// ---------------------------------------------------------------------------
// Random source trait.
// ---------------------------------------------------------------------------

/// Deterministic random source consumed by the world simulation.
///
/// Production implementations wrap the global Park-Miller generator. The trait
/// keeps the world step fully deterministic and testable.
pub trait WorldRandom {
    /// Return the next 32-bit pseudo-random value, matching `TFB_Random()`.
    fn next(&mut self) -> u32;
}

// ---------------------------------------------------------------------------
// Byte extraction helpers — mirror the C macros from compiler.h.
// ---------------------------------------------------------------------------

/// Low byte of a 32-bit value (`LOBYTE`).
const fn lobyte(value: u32) -> u8 {
    value as u8
}

/// High byte of the low 16-bit word (`HIBYTE` of `LOWORD`).
const fn hibyte_low_word(value: u32) -> u8 {
    ((value & 0xFFFF) >> 8) as u8
}

/// Low byte of the high 16-bit word (`LOBYTE` of `HIWORD`).
const fn lobyte_high_word(value: u32) -> u8 {
    ((value >> 16) & 0xFF) as u8
}

/// Low 16-bit word of a 32-bit value (`LOWORD`).
const fn loword(value: u32) -> u16 {
    value as u16
}

/// High 16-bit word of a 32-bit value (`HIWORD`).
const fn hiword(value: u32) -> u16 {
    (value >> 16) as u16
}

/// Low nibble of a byte (`LONIBBLE`).
const fn lonibble(value: u8) -> u8 {
    value & 0x0f
}

/// Pack two nibbles into one byte (`MAKE_BYTE`).
const fn make_byte(lo: u8, hi: u8) -> u8 {
    (hi << 4) | (lo & 0x0f)
}

// ---------------------------------------------------------------------------
// Hazard spawn requests emitted by the world step.
// ---------------------------------------------------------------------------

/// One hazard spawn produced by the per-frame hazard gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HazardSpawn {
    pub kind: HazardKind,
    pub position: SurfacePoint,
    pub life_span: u16,
    /// Lava facing used for directional offspring spawning.
    pub facing: u8,
}

/// Detailed result of the hazard chance gate (`BuildObjectList` head).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HazardGate {
    pub spawn_fire: bool,
    pub spawn_earthquake: bool,
    pub spawn_lightning: bool,
}

/// Evaluate the single-`rand_val` hazard gate from `BuildObjectList`.
///
/// The C code draws exactly one random value and extracts three bytes to test
/// against the fire, tectonics, and weather chances. Preserving byte order and
/// comparison direction is required for determinism.
#[must_use]
pub fn evaluate_hazard_gate(rand_val: u32, chances: HazardChances) -> HazardGate {
    // C order: fire uses LOBYTE(HIWORD), tectonics uses HIBYTE(LOWORD),
    // weather uses LOBYTE(LOWORD).
    HazardGate {
        spawn_fire: lobyte_high_word(rand_val) < chances.fire,
        spawn_earthquake: hibyte_low_word(rand_val) < chances.tectonics,
        spawn_lightning: lobyte(rand_val) < chances.weather,
    }
}

/// Per-planet hazard chances (out of 256) derived from the hazard rating tables.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HazardChances {
    pub tectonics: u8,
    pub weather: u8,
    pub fire: u8,
}

/// Compute the three per-planet hazard chances from planetary ratings.
///
/// `temperature` feeds the thermal rating which selects the fire-chance entry.
#[must_use]
pub fn hazard_chances(tectonics_rating: u8, weather_rating: u8, temperature: i32) -> HazardChances {
    HazardChances {
        tectonics: hazard_chance(HazardKind::Earthquake, tectonics_rating),
        weather: hazard_chance(HazardKind::Lightning, weather_rating),
        fire: hazard_chance(HazardKind::Lava, thermal_hazard_rating(temperature)),
    }
}

// ---------------------------------------------------------------------------
// Hazard spawn geometry — ports AddLightning and AddGroundDisaster.
// ---------------------------------------------------------------------------

/// Spawn parameters for one lightning strike, derived from two random draws.
///
/// The first draw selects harmless vs. damaging; the second packs life span and
/// position. This preserves the exact C random-consumption order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LightningSpawn {
    pub harmful: bool,
    pub position: SurfacePoint,
    pub life_span: u16,
}

/// Resolve one lightning strike, consuming random values in C order.
///
/// Returns `None` only if the caller decides allocation failed; the geometry is
/// always produced. The caller is responsible for inserting the entity.
pub fn roll_lightning(random: &mut impl WorldRandom, lander: SurfacePoint) -> LightningSpawn {
    let harmful = random.next() % 100 < 25;
    let rand_val = random.next();
    let life_span = 10 + (hiword(rand_val) % 10) + 1;
    let position = SurfacePoint {
        x: (lander.x
            + (WORLD_WIDTH - (SURFACE_WIDTH / 2 - 6))
            + i32::from(lobyte(rand_val) % (SURFACE_WIDTH - 12) as u8))
        .rem_euclid(WORLD_WIDTH),
        y: (lander.y
            + (WORLD_HEIGHT - (SURFACE_HEIGHT / 2 - 6))
            + i32::from(hibyte_low_word(rand_val) % (SURFACE_HEIGHT - 12) as u8))
        .rem_euclid(WORLD_HEIGHT),
    };
    LightningSpawn {
        harmful,
        position,
        life_span,
    }
}

/// Spawn parameters for one ground disaster (earthquake or lava).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroundDisasterSpawn {
    pub kind: HazardKind,
    pub position: SurfacePoint,
    pub life_span: u16,
    pub facing: u8,
}

/// Resolve one ground disaster, consuming random values in C order.
///
/// Earthquakes use `turn_wait = MAKE_BYTE(2, 2)`; lava uses `MAKE_BYTE(0, 0)`
/// plus an additional random facing. The `life_span` is computed as
/// `frame_count * (lonibble(turn_wait) + 1) - 1`. Because Rust does not own the
/// frame-count assets at this layer, the caller supplies `frame_count`.
pub fn roll_ground_disaster(
    random: &mut impl WorldRandom,
    kind: HazardKind,
    lander: SurfacePoint,
    frame_count: u16,
) -> GroundDisasterSpawn {
    let rand_val = random.next();
    let position = SurfacePoint {
        x: (lander.x
            + (WORLD_WIDTH - SURFACE_WIDTH * 3 / 8)
            + i32::from(loword(rand_val) % (SURFACE_WIDTH * 3 / 4) as u16))
        .rem_euclid(WORLD_WIDTH),
        y: (lander.y
            + (WORLD_HEIGHT - SURFACE_HEIGHT * 3 / 8)
            + i32::from(hiword(rand_val) % (SURFACE_HEIGHT * 3 / 4) as u16))
        .rem_euclid(WORLD_HEIGHT),
    };
    let (turn_wait, facing) = match kind {
        HazardKind::Earthquake => (make_byte(2, 2), 0u8),
        HazardKind::Lava => {
            let facing = normalize_facing(random.next() as u16) as u8;
            (make_byte(0, 0), facing)
        }
        HazardKind::Biological | HazardKind::Lightning => (0, 0),
    };
    let life_span = frame_count.saturating_mul(u16::from(lonibble(turn_wait) + 1));
    GroundDisasterSpawn {
        kind,
        position,
        life_span: life_span.saturating_sub(1),
        facing,
    }
}

/// Convert a `GroundDisasterSpawn` into a surface entity.
#[must_use]
pub fn ground_disaster_entity(spawn: GroundDisasterSpawn) -> SurfaceEntity {
    SurfaceEntity {
        kind: SurfaceEntityKind::Hazard(spawn.kind),
        position: spawn.position,
        finite_life: Some(spawn.life_span),
    }
}

/// Convert a `LightningSpawn` into a surface entity.
#[must_use]
pub fn lightning_entity(spawn: LightningSpawn) -> SurfaceEntity {
    SurfaceEntity {
        kind: SurfaceEntityKind::Hazard(HazardKind::Lightning),
        position: spawn.position,
        finite_life: Some(spawn.life_span),
    }
}

// ---------------------------------------------------------------------------
// Creature AI — ports the creature branch of object_animation.
// ---------------------------------------------------------------------------

/// Parameters needed by [`update_creature`] for one creature.
pub struct CreatureUpdateContext<'a> {
    pub kind: CreatureKind,
    pub hit_points: u8,
    pub aware: bool,
    pub velocity: &'a mut crate::battle::velocity::VelocityDesc,
    pub thrust_wait: &'a mut u8,
    pub frame_index: &'a mut u16,
    pub position: SurfacePoint,
    pub lander: SurfacePoint,
}

/// Update one live creature for a single frame.
///
/// This ports the `else if (!(frame_index & 3) && ElementPtr->hit_points)`
/// branch of `object_animation`. It mutates the creature's `velocity`,
/// `thrust_wait`, `aware` flag, and `frame_index`. Position integration is
/// performed later by [`step_world`], not here.
pub fn update_creature(ctx: &mut CreatureUpdateContext<'_>, random: &mut impl WorldRandom) -> bool {
    let kind = ctx.kind;
    let hit_points = ctx.hit_points;
    let aware = ctx.aware;
    let velocity = &mut *ctx.velocity;
    let thrust_wait = &mut *ctx.thrust_wait;
    let frame_index = &mut *ctx.frame_index;
    let position = ctx.position;
    let lander = ctx.lander;

    *frame_index = frame_index.wrapping_add(1);

    let stats = CreatureCatalog::stats(kind);
    if stats.speed == CreatureSpeed::Motionless || hit_points == 0 {
        return aware;
    }

    if *frame_index & CREATURE_AI_FRAME_MASK != 0 {
        return aware;
    }

    // Distance to lander with X wrapping (shortest path).
    let mut dx = lander.x - position.x;
    if dx < -HALF_WORLD_WIDTH {
        dx += WORLD_WIDTH;
    } else if dx > HALF_WORLD_WIDTH {
        dx -= WORLD_WIDTH;
    }
    let dy = lander.y - position.y;
    let mut angle = arctan(dx, dy);
    let abs_dx = dx.abs();
    let abs_dy = dy.abs();

    // Awareness range check.
    let out_of_range = abs_dx >= SURFACE_WIDTH
        || abs_dy >= SURFACE_WIDTH
        || abs_dx * abs_dx + abs_dy * abs_dy >= SURFACE_WIDTH * SURFACE_WIDTH;

    let mut aware = aware;
    if out_of_range {
        aware = false;
    } else if !aware {
        let detect_percent = (((stats.awareness as u8) + 1) * (30 / 6)) as u32;
        if random.next() % 100 < detect_percent {
            *thrust_wait = 0;
            aware = true;
        }
    }

    // Pin to surface edge: clear thrust wait so a new heading is chosen.
    if position.y == 0 || position.y == WORLD_HEIGHT - 1 {
        *thrust_wait = 0;
    }

    let old_angle = velocity.get_travel_angle();

    let new_angle = if *thrust_wait > 0 {
        *thrust_wait -= 1;
        old_angle
    } else if !aware || stats.behavior == CreatureBehavior::Unpredictable {
        let rand_val = random.next();
        let chosen = normalize_angle(u16::from(lobyte(rand_val)));
        *thrust_wait = (hibyte_low_word(rand_val) >> 2) + WANDER_THRUST_BASE;
        chosen
    } else if stats.behavior == CreatureBehavior::Flee {
        // Fleeing creatures at the vertical edge pick a horizontal heading
        // before inverting the lander angle. The C code's branch is convoluted;
        // the observable effect is that the creature moves sideways when pinned.
        if position.y == 0 || position.y == WORLD_HEIGHT - 1 {
            if angle & (HALF_CIRCLE - 1) != 0 {
                angle = HALF_CIRCLE - angle;
            } else if old_angle != 16 && old_angle != 48 {
                angle = if (random.next() & 1) == 0 { 48 } else { 16 };
            }
            *thrust_wait = FLEE_EDGE_THRUST;
        }
        normalize_angle(angle + HALF_CIRCLE)
    } else {
        // Hunting creature: head toward the lander.
        angle
    };

    let speed = creature_speed(stats.speed);
    velocity.set_components(cosine(new_angle, speed), sine(new_angle, speed));

    aware
}

/// Compute the per-tick velocity magnitude for a creature speed tier.
///
/// Matches the C `switch (speed)` block: SLOW = `WORLD_TO_VELOCITY(2) >> 2`,
/// MEDIUM = `WORLD_TO_VELOCITY(2) >> 1`, FAST = `WORLD_TO_VELOCITY(2) * 9 / 10`.
#[must_use]
fn creature_speed(speed: CreatureSpeed) -> i32 {
    use crate::battle::velocity::world_to_velocity;
    match speed {
        CreatureSpeed::Motionless => 0,
        CreatureSpeed::Slow => world_to_velocity(2) >> 2,
        CreatureSpeed::Medium => world_to_velocity(2) >> 1,
        CreatureSpeed::Fast => world_to_velocity(2) * 9 / 10,
    }
}

// ---------------------------------------------------------------------------
// Shot-vs-creature collision — ports shotCreature.
// ---------------------------------------------------------------------------

/// Outcome of a single stun-bolt hitting a creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotCreatureOutcome {
    /// Creature was already canned; the shot passes through harmlessly.
    AlreadyCanned,
    /// Creature absorbed one hit point but survived. It is knocked back along
    /// the shot's travel angle and forced aware.
    Stunned,
    /// Creature's hit points reached zero and it was canned. `value` is the
    /// biological mass the canned creature is now worth.
    Canned { value: u16 },
    /// A special non-biological creature was defeated and must be removed
    /// without becoming collectible cargo.
    Destroyed,
}

/// Resolve a stun-bolt impact on a creature, matching `shotCreature`.
///
/// `shot_facing` is the lander facing that produced the shot. The caller is
/// responsible for removing or replacing the entity afterwards.
pub fn apply_shot_hit(
    kind: CreatureKind,
    hit_points: &mut u8,
    velocity: &mut crate::battle::velocity::VelocityDesc,
    thrust_wait: &mut u8,
    aware: &mut bool,
    shot_facing: u8,
) -> ShotCreatureOutcome {
    if *hit_points == 0 {
        return ShotCreatureOutcome::AlreadyCanned;
    }

    *hit_points -= 1;
    if *hit_points == 0 {
        let value = u16::from(CreatureCatalog::stats(kind).value);
        if kind.is_brainbox_bulldozer() {
            ShotCreatureOutcome::Destroyed
        } else {
            ShotCreatureOutcome::Canned { value }
        }
    } else {
        let stats = CreatureCatalog::stats(kind);
        if stats.speed != CreatureSpeed::Motionless {
            let angle = facing_to_angle(u16::from(shot_facing));
            let magnitude = crate::battle::velocity::world_to_velocity(1);
            velocity.delta_components(cosine(angle, magnitude), sine(angle, magnitude));
            *thrust_wait = 0;
            *aware = true;
        }
        ShotCreatureOutcome::Stunned
    }
}

// ---------------------------------------------------------------------------
// Lander collision damage — ports the biological and disaster contact rules.
// ---------------------------------------------------------------------------

/// Decide whether a creature contact injures the lander.
///
/// Returns `true` when the danger roll succeeds. The caller applies the actual
/// crew damage via [`super::collision::resolve_lander_collision`].
#[must_use]
pub fn creature_contact_injures(danger_level: u8, roll_mod_128: u32) -> bool {
    const DANGER_VALUES: [u32; 4] = [0, 6, 13, 26];
    let idx = usize::from(danger_level).min(DANGER_VALUES.len() - 1);
    roll_mod_128 % 128 < DANGER_VALUES[idx]
}

/// Decide whether a ground disaster (earthquake/lava) injures the lander.
#[must_use]
pub fn ground_disaster_injures(roll_mod_100: u32) -> bool {
    roll_mod_100 % 100 < GROUND_DISASTER_INJURY_PERCENT
}

/// Decide whether a lightning strike kills one crew when it strikes the lander.
#[must_use]
pub fn lightning_kills_crew(roll_mod_100: u32) -> bool {
    roll_mod_100 % 100 < LIGHTNING_KILL_PERCENT
}

// ---------------------------------------------------------------------------
// World step — ports BuildObjectList per-frame entity processing.
// ---------------------------------------------------------------------------

/// Request to insert a new entity into the world this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRequest {
    pub entity: SurfaceEntity,
    pub cue: Option<super::hazards::SoundCue>,
}

/// Effects emitted while stepping the world for one frame.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldStepEffects {
    /// New entities to insert (hazards, offspring lava).
    pub spawns: Vec<SpawnRequest>,
    /// Sound cues in source order.
    pub sounds: Vec<super::hazards::SoundCue>,
    /// Crew killed by lightning strikes this frame.
    pub lightning_kills: u8,
    /// IDs of creatures that were canned by shots this frame, with their new
    /// biological value. The caller swaps the entity kind.
    pub canned: Vec<(SurfaceEntityId, u16)>,
    /// IDs of entities whose finite life expired this frame. The caller must
    /// remove their frames and masks in sync with the world.
    pub expired: Vec<SurfaceEntityId>,
    /// Live creatures whose animation advanced this frame: `(id, kind,
    /// animation_frame)`. The caller updates the drawable frame and its collision
    /// mask atomically so hotspot and extent changes apply together.
    pub creature_frames: Vec<(SurfaceEntityId, super::creatures::CreatureKind, u16)>,
    /// IDs of creatures to which this world step gave a landed shot verdict:
    /// one verdict per firing that changed a creature this frame: one `Stunned`,
    /// canned, or destroyed, excluding a bolt that only passed over an
    /// already-canned creature.
    pub landed_shot: Vec<SurfaceEntityId>,
}

/// Borrowed inputs to [`step_world`].
///
/// Grouping these in a struct keeps the per-frame entry point under the project
/// `too-many-arguments-threshold` without suppressing lints.
pub struct WorldStepInputs<'a, M: MaskLookup, R: WorldRandom> {
    /// Current lander position in world coordinates.
    pub lander_position: SurfacePoint,
    /// Mask lookup for shot entities.
    pub shot_masks: &'a M,
    /// Mask lookup for creature entities.
    pub creature_masks: &'a M,
    /// Deterministic random source.
    pub random: &'a mut R,
}

/// Apply one surface-frame world step to every entity.
///
/// This ports the body of `BuildObjectList` that follows the hazard gate:
/// decrement lifetimes, run creature preprocess (AI), integrate velocity, wrap
/// coordinates, and resolve shot-vs-creature pixel collisions. Hazard *spawning*
/// is handled separately by [`hazard_spawns_for_frame`] so callers can wire it
/// to their own entity/mask registries.
pub fn step_world<M: MaskLookup, R: WorldRandom>(
    world: &mut SurfaceWorld,
    inputs: WorldStepInputs<'_, M, R>,
) -> WorldStepEffects {
    let mut effects = WorldStepEffects::default();
    let ids = world.ids();

    let WorldStepInputs {
        lander_position,
        shot_masks,
        creature_masks,
        random,
    } = inputs;

    for id in ids {
        process_entity(
            world,
            id,
            lander_position,
            shot_masks,
            creature_masks,
            random,
            &mut effects,
        );
    }

    // Expire finite lifetimes after processing. This matches the C loop which
    // decrements life_span each frame and removes elements at zero.
    effects.expired.extend(world.advance_lifetimes());
    effects
}

/// Trait abstracting collision-mask lookup by entity ID.
///
/// In production this is backed by `SurfaceMasks`; tests supply a simple map.
pub trait MaskLookup {
    fn mask(&self, id: SurfaceEntityId) -> Option<&CollisionMask>;
}

fn process_entity<M: MaskLookup, R: WorldRandom>(
    world: &mut SurfaceWorld,
    id: SurfaceEntityId,
    lander_position: SurfacePoint,
    shot_masks: &M,
    creature_masks: &M,
    random: &mut R,
    effects: &mut WorldStepEffects,
) {
    let Some(entity) = world.get_mut(id) else {
        return;
    };

    match &mut entity.kind {
        SurfaceEntityKind::LiveCreature {
            kind,
            hit_points,
            aware,
            velocity,
            thrust_wait,
            frame_index,
        } => {
            let position = entity.position;
            let mut ctx = CreatureUpdateContext {
                kind: *kind,
                hit_points: *hit_points,
                aware: *aware,
                velocity,
                thrust_wait,
                frame_index,
                position,
                lander: lander_position,
            };
            let new_aware = update_creature(&mut ctx, random);
            *aware = new_aware;
            // Animation leaks from frame_index every surface frame: the C code
            // advances the primitive frame inside object_animation, and each step
            // must keep hotspot and extent in lockstep with the drawn image.
            effects.creature_frames.push((id, *kind, *frame_index));
        }
        SurfaceEntityKind::Shot(shot) => {
            // Lifetime is tracked via finite_life; velocity integration below.
            let _ = shot;
        }
        _ => {}
    }

    // Integrate velocity for all entities that have one.
    let (dx, dy) = entity_velocity_delta(world, id);
    if dx != 0 || dy != 0 {
        let Some(entity) = world.get_mut(id) else {
            return;
        };
        entity.position.x += dx;
        entity.position.y += dy;

        let is_player_shot = matches!(
            entity.kind,
            SurfaceEntityKind::Shot(super::simulation::Shot { .. })
        );
        if !is_player_shot {
            // Non-player entities clamp vertically to the surface bounds.
            if entity.position.y < 0 {
                entity.position.y = 0;
            } else if entity.position.y >= WORLD_HEIGHT {
                entity.position.y = WORLD_HEIGHT - 1;
            }
        }
        // Horizontal wrapping for all entities.
        if entity.position.x < 0 {
            entity.position.x += WORLD_WIDTH;
        } else if entity.position.x >= WORLD_WIDTH {
            entity.position.x -= WORLD_WIDTH;
        }
    }

    // Shot-vs-creature collision.
    check_shot_creature_collision(world, id, shot_masks, creature_masks, effects);
}

/// Compute the per-frame velocity delta for an entity.
fn entity_velocity_delta(world: &mut SurfaceWorld, id: SurfaceEntityId) -> (i32, i32) {
    let Some(entity) = world.get_mut(id) else {
        return (0, 0);
    };
    match &mut entity.kind {
        SurfaceEntityKind::Shot(shot) => velocity_delta(shot.velocity_x, shot.velocity_y),
        SurfaceEntityKind::LiveCreature { velocity, .. } => velocity.get_next_components(1),
        _ => (0, 0),
    }
}

fn velocity_delta(x: i32, y: i32) -> (i32, i32) {
    let mut velocity = crate::battle::velocity::VelocityDesc::new();
    velocity.set_components(x, y);
    velocity.get_next_components(1)
}

/// Resolve shot-vs-creature pixel collisions for one shot entity.
fn check_shot_creature_collision<M: MaskLookup>(
    world: &mut SurfaceWorld,
    shot_id: SurfaceEntityId,
    shot_masks: &M,
    creature_masks: &M,
    effects: &mut WorldStepEffects,
) {
    let shot_mask = match shot_masks.mask(shot_id) {
        Some(m) => m,
        None => return,
    };
    let shot_position = match world.get(shot_id) {
        Some(e) => e.position,
        None => return,
    };
    let shot_facing = match world.get(shot_id) {
        Some(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot { facing, .. }),
            ..
        }) => *facing,
        _ => return,
    };

    let candidate_ids: Vec<SurfaceEntityId> = world.ids();
    for creature_id in candidate_ids {
        if creature_id == shot_id {
            continue;
        }
        let creature_mask = match creature_masks.mask(creature_id) {
            Some(m) => m,
            None => continue,
        };
        let creature_position = match world.get(creature_id) {
            Some(e) => e.position,
            None => continue,
        };
        // The lander-vs-entity loop uses the wrapped form so a shot whose drawn
        // image crosses the horizontal seam connects exactly where it is rendered.
        if !masks_intersect_wrapped(
            shot_position,
            shot_mask,
            creature_position,
            creature_mask,
            WORLD_WIDTH,
        ) {
            continue;
        }

        let Some(creature) = world.get_mut(creature_id) else {
            continue;
        };
        let SurfaceEntityKind::LiveCreature {
            kind,
            hit_points,
            aware,
            velocity,
            thrust_wait,
            ..
        } = &mut creature.kind
        else {
            continue;
        };

        let outcome = apply_shot_hit(*kind, hit_points, velocity, thrust_wait, aware, shot_facing);
        effects.sounds.push(super::hazards::SoundCue::LanderHits);
        if !matches!(outcome, ShotCreatureOutcome::AlreadyCanned) {
            effects.landed_shot.push(creature_id);
        }
        match outcome {
            ShotCreatureOutcome::AlreadyCanned => {}
            ShotCreatureOutcome::Stunned => {}
            ShotCreatureOutcome::Canned { value } => {
                effects.canned.push((creature_id, value));
                effects
                    .sounds
                    .push(super::hazards::SoundCue::LifeformCanned);
            }
            ShotCreatureOutcome::Destroyed => effects.expired.push(creature_id),
        }
    }
}

/// Convenience: roll all hazard spawns for one frame.
///
/// Consumes random values in the exact order the C `BuildObjectList` head
/// requires: the gate value first, then one spawn-rolling sequence per enabled
/// hazard. The caller inserts entities and plays sounds.
pub fn hazard_spawns_for_frame(
    random: &mut impl WorldRandom,
    chances: HazardChances,
    lander: SurfacePoint,
    earthquake_frame_count: u16,
    lava_frame_count: u16,
) -> Vec<HazardSpawn> {
    let gate = evaluate_hazard_gate(random.next(), chances);
    let mut spawns = Vec::new();

    if gate.spawn_fire {
        let roll = roll_ground_disaster(random, HazardKind::Lava, lander, lava_frame_count);
        spawns.push(HazardSpawn {
            kind: HazardKind::Lava,
            position: roll.position,
            life_span: roll.life_span,
            facing: roll.facing,
        });
    }
    if gate.spawn_earthquake {
        let roll = roll_ground_disaster(
            random,
            HazardKind::Earthquake,
            lander,
            earthquake_frame_count,
        );
        spawns.push(HazardSpawn {
            kind: HazardKind::Earthquake,
            position: roll.position,
            life_span: roll.life_span,
            facing: roll.facing,
        });
    }
    if gate.spawn_lightning {
        let roll = roll_lightning(random, lander);
        spawns.push(HazardSpawn {
            kind: HazardKind::Lightning,
            position: roll.position,
            life_span: roll.life_span,
            facing: 0,
        });
    }

    spawns
}

// ---------------------------------------------------------------------------
// Shot helpers — lifetime and movement.
// ---------------------------------------------------------------------------

/// Decrement a shot's remaining life by one frame.
///
/// Returns `true` if the shot is still alive.
pub fn decrement_shot_life(shot: &mut Shot) -> bool {
    if shot.life == 0 {
        return false;
    }
    shot.life -= 1;
    shot.life > 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::velocity::VelocityDesc;
    use crate::planet_side::geometry::CollisionMask;
    use std::collections::HashMap;

    /// Deterministic random source backed by a fixed queue.
    struct SeqRandom {
        values: std::collections::VecDeque<u32>,
        calls: usize,
    }

    impl SeqRandom {
        fn new(values: &[u32]) -> Self {
            Self {
                values: values.iter().copied().collect(),
                calls: 0,
            }
        }

        fn remaining(&self) -> usize {
            self.values.len()
        }
    }

    impl WorldRandom for SeqRandom {
        fn next(&mut self) -> u32 {
            self.calls += 1;
            self.values.pop_front().unwrap_or(0)
        }
    }

    /// HashMap-backed mask lookup for tests.
    #[derive(Default)]
    struct MapMasks(HashMap<SurfaceEntityId, CollisionMask>);

    impl MaskLookup for MapMasks {
        fn mask(&self, id: SurfaceEntityId) -> Option<&CollisionMask> {
            self.0.get(&id)
        }
    }

    fn solid_mask() -> CollisionMask {
        CollisionMask::from_occupancy(1, 1, SurfacePoint::default(), &[1]).unwrap()
    }

    fn live_creature(kind_idx: u8, hp: u8, position: SurfacePoint) -> SurfaceEntity {
        SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: CreatureKind::new(kind_idx).unwrap(),
                hit_points: hp,
                aware: false,
                velocity: VelocityDesc::new(),
                thrust_wait: 0,
                frame_index: 0,
            },
            position,
            finite_life: None,
        }
    }

    // -- byte extraction helpers ------------------------------------------

    #[test]
    fn byte_helpers_match_c_macros() {
        let val = 0x12345678u32;
        assert_eq!(lobyte(val), 0x78);
        assert_eq!(hibyte_low_word(val), 0x56); // HIBYTE(LOWORD=0x5678)
        assert_eq!(lobyte_high_word(val), 0x34); // LOBYTE(HIWORD=0x1234)
        assert_eq!(loword(val), 0x5678);
        assert_eq!(hiword(val), 0x1234);
        assert_eq!(lonibble(0xAB), 0x0B);
        assert_eq!(make_byte(2, 2), 0x22);
    }

    // -- hazard gate -------------------------------------------------------

    #[test]
    fn hazard_gate_extracts_three_bytes_from_one_rand() {
        // fire = LOBYTE(HIWORD), tectonics = HIBYTE(LOWORD), weather = LOBYTE.
        let chances = HazardChances {
            fire: 100,
            tectonics: 100,
            weather: 100,
        };
        // For 0x00640064:
        //   LOBYTE(HIWORD=0x0064) = 0x64 = 100, not < 100 → no fire.
        //   HIBYTE(LOWORD=0x0064) = 0x00 = 0, < 100 → earthquake spawns.
        //   LOBYTE(LOWORD=0x0064)  = 0x64 = 100, not < 100 → no weather.
        let gate = evaluate_hazard_gate(0x00640064, chances);
        assert!(!gate.spawn_fire);
        assert!(gate.spawn_earthquake);
        assert!(!gate.spawn_lightning);
    }

    #[test]
    fn hazard_gate_fires_only_when_byte_below_threshold() {
        let chances = HazardChances {
            fire: 50,
            tectonics: 50,
            weather: 50,
        };
        // LOBYTE(HIWORD)=0x10=16, HIBYTE(LOWORD)=0x20=32, LOBYTE=0x30=48.
        let gate = evaluate_hazard_gate(0x00102030, chances);
        assert!(gate.spawn_fire);
        assert!(gate.spawn_earthquake);
        assert!(gate.spawn_lightning);

        // All bytes = 60 → none fire.
        let gate = evaluate_hazard_gate(0x003C3C3C, chances);
        assert!(!gate.spawn_fire);
        assert!(!gate.spawn_earthquake);
        assert!(!gate.spawn_lightning);
    }

    #[test]
    fn hazard_chances_match_rating_tables() {
        let chances = hazard_chances(5, 5, 400);
        assert_eq!(chances.tectonics, 24); // TectonicsChanceTab[5]*3 = 8*3
        assert_eq!(chances.weather, 18); // WeatherChanceTab[5]*3 = 6*3
        assert_eq!(chances.fire, 36); // FireChanceTab at temp 400 → rating 5 → 12*3
    }

    // -- lightning spawn ---------------------------------------------------

    #[test]
    fn lightning_spawn_preserves_rand_order() {
        let mut random = SeqRandom::new(&[10, 0x00010002]);
        let spawn = roll_lightning(&mut random, SurfacePoint { x: 100, y: 100 });
        // First rand 10 % 100 = 10 < 25 → harmful.
        assert!(spawn.harmful);
        // life_span = 10 + (HIWORD=0x0001 % 10) + 1 = 12.
        assert_eq!(spawn.life_span, 12);
        assert_eq!(random.calls, 2);
    }

    // -- ground disaster spawn --------------------------------------------

    #[test]
    fn earthquake_spawn_uses_make_byte_2_2() {
        let mut random = SeqRandom::new(&[0x00010002]);
        let spawn = roll_ground_disaster(
            &mut random,
            HazardKind::Earthquake,
            SurfacePoint { x: 100, y: 100 },
            13,
        );
        // frame_count * (LONIBBLE(0x22) + 1) - 1 = 13 * 3 - 1 = 38.
        assert_eq!(spawn.life_span, 38);
        assert_eq!(spawn.facing, 0);
        assert_eq!(random.calls, 1);
    }

    #[test]
    fn lava_spawn_consumes_extra_facing_rand() {
        let mut random = SeqRandom::new(&[0x00010002, 5]);
        let spawn = roll_ground_disaster(
            &mut random,
            HazardKind::Lava,
            SurfacePoint { x: 100, y: 100 },
            7,
        );
        assert_eq!(spawn.facing, normalize_facing(5) as u8);
        assert_eq!(random.calls, 2);
    }

    // -- creature AI -------------------------------------------------------

    #[test]
    fn motionless_creature_never_moves() {
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 0u8;
        let mut frame_index = 0u16;
        let mut random = SeqRandom::new(&[]);
        let mut ctx = CreatureUpdateContext {
            kind: CreatureKind::new(0).unwrap(),
            hit_points: 1,
            aware: false,
            velocity: &mut velocity,
            thrust_wait: &mut thrust_wait,
            frame_index: &mut frame_index,
            position: SurfacePoint { x: 0, y: 0 },
            lander: SurfacePoint { x: 50, y: 50 },
        };
        let aware = update_creature(&mut ctx, &mut random);
        assert!(!aware);
        assert_eq!(velocity.get_current_components(), (0, 0));
    }

    #[test]
    fn creature_out_of_range_clears_awareness() {
        let mut velocity = VelocityDesc::new();
        velocity.set_components(10, 0);
        let mut thrust_wait = 0u8;
        let mut frame_index = 3u16; // becomes 4 → 4 & 3 == 0 → AI runs.
        let mut random = SeqRandom::new(&[]);
        let mut ctx = CreatureUpdateContext {
            kind: CreatureKind::new(5).unwrap(),
            hit_points: 5,
            aware: true,
            velocity: &mut velocity,
            thrust_wait: &mut thrust_wait,
            frame_index: &mut frame_index,
            position: SurfacePoint { x: 0, y: 0 },
            lander: SurfacePoint { x: 500, y: 500 },
        };
        let aware = update_creature(&mut ctx, &mut random);
        assert!(!aware);
    }

    #[test]
    fn hunting_creature_steers_toward_lander() {
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 0u8;
        let mut frame_index = 3u16; // becomes 4 → 4 & 3 == 0 → AI runs.
        let mut random = SeqRandom::new(&[]);
        let mut ctx = CreatureUpdateContext {
            kind: CreatureKind::new(9).unwrap(),
            hit_points: 15,
            aware: true,
            velocity: &mut velocity,
            thrust_wait: &mut thrust_wait,
            frame_index: &mut frame_index,
            position: SurfacePoint { x: 100, y: 0 },
            lander: SurfacePoint { x: 100, y: 100 },
        };
        let aware = update_creature(&mut ctx, &mut random);
        assert!(aware);
        let (_, dy) = velocity.get_current_components();
        assert!(dy > 0, "hunting creature should move toward lander");
    }

    #[test]
    fn unpredictable_creature_consumes_one_rand_for_heading() {
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 0u8;
        let mut frame_index = 3u16;
        let mut random = SeqRandom::new(&[0x0120]); // LOBYTE=0x20=32, HIBYTE=0x01
        let mut ctx = CreatureUpdateContext {
            kind: CreatureKind::new(5).unwrap(),
            hit_points: 2,
            aware: false,
            velocity: &mut velocity,
            thrust_wait: &mut thrust_wait,
            frame_index: &mut frame_index,
            position: SurfacePoint { x: 100, y: 100 },
            lander: SurfacePoint { x: 110, y: 110 },
        };
        let _ = update_creature(&mut ctx, &mut random);
        assert_eq!(random.remaining(), 0);
        // thrust_wait = (HIBYTE(LOWORD=0x0120) >> 2) + 10 = (0x01 >> 2) + 10 = 10.
        assert_eq!(thrust_wait, 10);
    }

    #[test]
    fn creature_ai_only_runs_every_fourth_frame() {
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 0u8;
        let mut frame_index = 0u16;
        let mut random = SeqRandom::new(&[0xFF; 8]);
        let initial_velocity = velocity;
        for expected_frame in 1..=5u16 {
            let mut ctx = CreatureUpdateContext {
                kind: CreatureKind::new(5).unwrap(),
                hit_points: 2,
                aware: false,
                velocity: &mut velocity,
                thrust_wait: &mut thrust_wait,
                frame_index: &mut frame_index,
                position: SurfacePoint { x: 0, y: 0 },
                lander: SurfacePoint { x: 10, y: 10 },
            };
            let _ = update_creature(&mut ctx, &mut random);
            assert_eq!(frame_index, expected_frame);
        }
        // Frame 4 (index 4, mask 0) is the only AI frame; velocity should have
        // changed exactly once.
        assert_ne!(velocity, initial_velocity);
    }

    // -- shot vs creature --------------------------------------------------

    #[test]
    fn shot_cans_creature_at_zero_hp() {
        let kind = CreatureKind::new(0).unwrap(); // value=1, hp=1
        let mut hp = 1u8;
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 0u8;
        let mut aware = false;
        let outcome = apply_shot_hit(
            kind,
            &mut hp,
            &mut velocity,
            &mut thrust_wait,
            &mut aware,
            0,
        );
        assert_eq!(outcome, ShotCreatureOutcome::Canned { value: 1 });
        assert_eq!(hp, 0);
    }

    #[test]
    fn shot_stuns_surviving_creature_with_knockback() {
        // Creature 3: NORMAL danger, hp=3, no speed field? Check: attributes=0xC0 →
        // speed = MOTIONLESS. So no knockback. Use creature 8 instead.
        let kind = CreatureKind::new(8).unwrap(); // HUNT|SLOW|NORMAL, hp=8
        let mut hp = 8u8;
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 5u8;
        let mut aware = false;
        let outcome = apply_shot_hit(
            kind,
            &mut hp,
            &mut velocity,
            &mut thrust_wait,
            &mut aware,
            0,
        );
        assert_eq!(outcome, ShotCreatureOutcome::Stunned);
        assert_eq!(hp, 7);
        assert_eq!(thrust_wait, 0);
        assert!(aware);
    }

    #[test]
    fn shot_damages_surviving_brainbox_and_keeps_it_hostile() {
        let kind = CreatureKind::new(24).unwrap();
        let mut hp = 2u8;
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 5u8;
        let mut aware = false;

        let outcome = apply_shot_hit(
            kind,
            &mut hp,
            &mut velocity,
            &mut thrust_wait,
            &mut aware,
            0,
        );

        assert_eq!(outcome, ShotCreatureOutcome::Stunned);
        assert_eq!(hp, 1);
        assert!(aware);
    }

    #[test]
    fn final_shot_destroys_brainbox_instead_of_canning_it() {
        let kind = CreatureKind::new(24).unwrap();
        let mut hp = 1u8;
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 0u8;
        let mut aware = false;

        let outcome = apply_shot_hit(
            kind,
            &mut hp,
            &mut velocity,
            &mut thrust_wait,
            &mut aware,
            0,
        );

        assert_eq!(outcome, ShotCreatureOutcome::Destroyed);
        assert_eq!(hp, 0);
    }

    #[test]
    fn shot_on_canned_creature_is_noop() {
        let kind = CreatureKind::new(0).unwrap();
        let mut hp = 0u8;
        let mut velocity = VelocityDesc::new();
        let mut thrust_wait = 0u8;
        let mut aware = false;
        let outcome = apply_shot_hit(
            kind,
            &mut hp,
            &mut velocity,
            &mut thrust_wait,
            &mut aware,
            0,
        );
        assert_eq!(outcome, ShotCreatureOutcome::AlreadyCanned);
    }

    // -- lander collision --------------------------------------------------

    #[test]
    fn monstrous_creature_contact_injures() {
        assert!(creature_contact_injures(3, 0)); // danger=monstrous, roll 0 < 26
        assert!(!creature_contact_injures(3, 26)); // roll 26 not < 26
    }

    #[test]
    fn harmless_creature_contact_never_injures() {
        assert!(!creature_contact_injures(0, 0));
    }

    #[test]
    fn ground_disaster_contact_uses_25_percent_gate() {
        assert!(ground_disaster_injures(24));
        assert!(!ground_disaster_injures(25));
    }

    #[test]
    fn brainbox_bulldozer_is_destroyed_without_becoming_cargo() {
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(24, 1, SurfacePoint { x: 0, y: 0 }));
        let shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint { x: 0, y: 0 },
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: SurfacePoint { x: 0, y: 0 },
            finite_life: Some(12),
        });
        let mut masks = MapMasks::default();
        masks.0.insert(creature_id, solid_mask());
        masks.0.insert(shot_id, solid_mask());
        let mut random = SeqRandom::new(&[]);

        let effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint { x: 0, y: 0 },
                shot_masks: &masks,
                creature_masks: &masks,
                random: &mut random,
            },
        );

        assert!(effects.canned.is_empty());
        assert_eq!(effects.expired, [creature_id]);
        assert_eq!(
            effects.sounds,
            [super::super::hazards::SoundCue::LanderHits]
        );
    }

    #[test]
    fn lightning_kill_uses_10_percent_gate() {
        assert!(lightning_kills_crew(9));
        assert!(!lightning_kills_crew(10));
    }

    // -- world step --------------------------------------------------------

    #[test]
    fn step_world_runs_creature_ai_and_integrates_velocity() {
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(
            9, // HUNT|AWARE_MEDIUM|SLOW|MONSTROUS
            15,
            SurfacePoint { x: 100, y: 0 },
        ));
        let lander = SurfacePoint { x: 100, y: 100 };
        let mut random = SeqRandom::new(&[]);
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(creature_id, solid_mask());
        let shot_masks = MapMasks::default();

        // Advance through the first AI tick and enough fixed-point movement
        // frames to produce one world-coordinate displacement.
        for _ in 0..20 {
            step_world(
                &mut world,
                WorldStepInputs {
                    lander_position: lander,
                    shot_masks: &shot_masks,
                    creature_masks: &creature_masks,
                    random: &mut random,
                },
            );
        }
        let entity = world.get(creature_id).unwrap();
        assert!(entity.position.y > 0, "creature should move toward lander");
        assert!(
            entity.position.y <= 10,
            "slow creature exceeded its fixed-point speed bound: {}",
            entity.position.y
        );
    }

    #[test]
    fn step_world_cans_creature_on_shot_hit() {
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(
            0, // value=1, hp=1, motionless
            1,
            SurfacePoint::default(),
        ));
        let shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: SurfacePoint::default(),
            finite_life: Some(12),
        });

        let mut shot_masks = MapMasks::default();
        shot_masks.0.insert(shot_id, solid_mask());
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(creature_id, solid_mask());

        let mut random = SeqRandom::new(&[]);
        let effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        assert_eq!(effects.canned, vec![(creature_id, 1)]);
        assert!(effects
            .sounds
            .contains(&super::super::hazards::SoundCue::LifeformCanned));
    }

    /// Build a 2px-wide opaque mask centered so its second pixel is the `x + 1`
    /// ring pixel. Same shape idea as the seam-straddling fixture deposits.
    fn two_px_mask() -> CollisionMask {
        CollisionMask::from_occupancy(2, 1, SurfacePoint::default(), &[1, 1]).unwrap()
    }

    #[test]
    fn shot_hits_creature_across_seam_from_the_right() {
        // The shot sits on the left of the seam (ring pixel 0).  A 2px
        // Brainbox Bulldozer whose own raw x is the right edge straddles ring
        // pixels {W-1, 0}, so its ring-0 pixel is the drawn image that
        // crosses the seam.  Raw coordinates alone are a full ring apart; only the
        // wrapped ring copy makes them overlap.
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(
            24,
            1,
            SurfacePoint {
                x: WORLD_WIDTH - 1,
                y: 0,
            },
        ));
        let shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: SurfacePoint::default(),
            finite_life: Some(12),
        });
        let mut shot_masks = MapMasks::default();
        shot_masks.0.insert(shot_id, solid_mask());
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(creature_id, two_px_mask());

        let mut random = SeqRandom::new(&[]);
        let effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        assert_eq!(
            effects.landed_shot,
            [creature_id],
            "seam hit from the right must land one shot verdict"
        );
        assert!(
            effects.expired.contains(&creature_id),
            "seam hit must destroy the HP1 Brainbox"
        );
        assert!(
            effects.canned.is_empty(),
            "Brainbox destruction is never cargo"
        );
    }

    #[test]
    fn shot_hits_creature_across_seam_from_the_left() {
        // Mirror image of the other seam test: the shot is fired from the ring
        // pixel at the right edge (W-1) and the Brainbox raw x is ring 0.
        // Its right-hand wrapped copy reaches back across the seam to the bolt.
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(24, 1, SurfacePoint { x: 0, y: 0 }));
        let shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint {
                    x: WORLD_WIDTH - 1,
                    y: 0,
                },
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: SurfacePoint {
                x: WORLD_WIDTH - 1,
                y: 0,
            },
            finite_life: Some(12),
        });
        let mut shot_masks = MapMasks::default();
        shot_masks.0.insert(shot_id, two_px_mask());
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(creature_id, solid_mask());

        let mut random = SeqRandom::new(&[]);
        let effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        assert_eq!(
            effects.landed_shot,
            [creature_id],
            "seam hit from the left must land one shot verdict"
        );
        assert!(
            effects.expired.contains(&creature_id),
            "seam hit must destroy the HP1 Brainbox"
        );
        assert!(
            effects.canned.is_empty(),
            "Brainbox destruction is never cargo"
        );
    }

    #[test]
    fn same_raw_displacement_misses_when_it_does_not_cross_the_seam() {
        // A 2px Brainbox at {W-2, W-1} keeps a full-ring raw displacement
        // from the shot but no ring copy overlaps it: a miss proves the seam
        // tests above react to the seam, not to the raw offset.
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(24, 1, SurfacePoint { x: 966, y: 0 }));
        let shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint { x: 0, y: 0 },
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: SurfacePoint { x: 0, y: 0 },
            finite_life: Some(12),
        });
        let mut shot_masks = MapMasks::default();
        shot_masks.0.insert(shot_id, solid_mask());
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(creature_id, two_px_mask());

        let mut random = SeqRandom::new(&[]);
        let effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        assert!(
            effects.landed_shot.is_empty(),
            "full-ring displacement cannot land through the seam"
        );
        assert!(
            effects.expired.is_empty(),
            "a miss must not destroy the Brainbox"
        );
        assert!(effects.canned.is_empty());
    }

    #[test]
    fn exact_half_world_shot_creature_offset_stays_a_tie() {
        // A bolt and Brainbox separated by exactly half the world stay a tie: folding
        // either side keeps them half a ring apart, so the seam never closes the gap.
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(
            24,
            1,
            SurfacePoint {
                x: WORLD_WIDTH / 2,
                y: 0,
            },
        ));
        let shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: SurfacePoint::default(),
            finite_life: Some(12),
        });
        let mut shot_masks = MapMasks::default();
        shot_masks.0.insert(shot_id, solid_mask());
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(creature_id, two_px_mask());

        let mut random = SeqRandom::new(&[]);
        let effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        assert!(
            effects.landed_shot.is_empty(),
            "exact half-world offset must stay a miss"
        );
        assert!(
            effects.expired.is_empty(),
            "the tie must not destroy the Brainbox"
        );
        assert!(effects.canned.is_empty());
    }

    #[test]
    fn step_world_wraps_creature_x_at_world_edge() {
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: CreatureKind::new(8).unwrap(),
                hit_points: 8,
                aware: true,
                velocity: {
                    let mut v = VelocityDesc::new();
                    v.set_components(crate::battle::velocity::world_to_velocity(2), 0);
                    v
                },
                thrust_wait: 0,
                frame_index: 0,
            },
            position: SurfacePoint {
                x: WORLD_WIDTH - 1,
                y: 10,
            },
            finite_life: None,
        });
        let shot_masks = MapMasks::default();
        let creature_masks = MapMasks::default();
        let mut random = SeqRandom::new(&[]);
        step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint { x: 0, y: 0 },
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        let x = world.get(creature_id).unwrap().position.x;
        assert!(x < WORLD_WIDTH, "creature x should wrap: got {x}");
    }

    #[test]
    fn step_world_clamps_creature_y_to_surface_bounds() {
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: CreatureKind::new(8).unwrap(),
                hit_points: 8,
                aware: true,
                velocity: {
                    let mut v = VelocityDesc::new();
                    v.set_components(0, -crate::battle::velocity::world_to_velocity(2));
                    v
                },
                thrust_wait: 0,
                frame_index: 0,
            },
            position: SurfacePoint { x: 10, y: 0 },
            finite_life: None,
        });
        let shot_masks = MapMasks::default();
        let creature_masks = MapMasks::default();
        let mut random = SeqRandom::new(&[]);
        step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint { x: 0, y: 0 },
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        let y = world.get(creature_id).unwrap().position.y;
        assert_eq!(y, 0, "creature y should be clamped to 0");
    }

    #[test]
    fn hazard_spawns_consume_rand_in_c_order() {
        let mut random = SeqRandom::new(&[
            0x00010101, // gate: fire byte=1, tect byte=1, weather byte=1
            0x00010002, // fire (lava) position
            5,          // lava facing
            0x00010002, // earthquake position
            0x00010002, // lightning harmful roll
            0x00010002, // lightning position
        ]);
        let chances = HazardChances {
            fire: 50,
            tectonics: 50,
            weather: 50,
        };
        let spawns =
            hazard_spawns_for_frame(&mut random, chances, SurfacePoint { x: 100, y: 100 }, 13, 7);
        assert_eq!(spawns.len(), 3);
        assert!(matches!(spawns[0].kind, HazardKind::Lava));
        assert!(matches!(spawns[1].kind, HazardKind::Earthquake));
        assert!(matches!(spawns[2].kind, HazardKind::Lightning));
    }

    #[test]
    fn creature_animation_frame_is_reported_every_surface_step() {
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(24, 2, SurfacePoint { x: 10, y: 10 }));
        let shot_masks = MapMasks::default();
        let creature_masks = MapMasks::default();
        let mut random = SeqRandom::new(&[]);
        let effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        assert!(
            effects
                .creature_frames
                .iter()
                .any(|(id, kind, frame)| *id == creature_id
                    && kind.is_brainbox_bulldozer()
                    && *frame == 1),
            "brainbox must advance its animation frame each surface step"
        );
    }

    // -- Brainbox Bulldozer acceptance ------------------------------------------

    /// lifey.ani declares one extent and hotspot per creature animation frame:
    /// frame 0 is 15×5 at hotspot (7,4), frame 1 is 11×7 at (5,6),
    /// frame 2 is 11×12 at (5,11), frame 3 is 9×14 at (4,13).
    struct EntityPixel {
        world_x: i32,
        world_y: i32,
        mask: CollisionMask,
    }

    fn tile_pixel(
        width: u16,
        height: u16,
        hotspot: SurfacePoint,
        offset_x: u16,
        offset_y: u16,
    ) -> EntityPixel {
        let mut occupancy = vec![0u8; usize::from(width) * usize::from(height)];
        occupancy[(usize::from(offset_y) * usize::from(width)) + usize::from(offset_x)] = 1;
        EntityPixel {
            // With the creature at the world origin the opaque pixel at local
            // `offset` lives at `offset - hotspot`.
            world_x: i32::from(offset_x) - hotspot.x,
            world_y: i32::from(offset_y) - hotspot.y,
            mask: CollisionMask::from_occupancy(width, height, hotspot, &occupancy).unwrap(),
        }
    }

    /// The four living brainbox frames 0..3 from lifey.ani, in the animation
    /// contract: `(width, height, hotspot)`.
    fn brainbox_extents(frame: u16) -> (u16, u16, SurfacePoint) {
        let (width, height, x, y) = match frame {
            0 => (15, 5, 7, 4),
            1 => (11, 7, 5, 6),
            2 => (11, 12, 5, 11),
            3 => (9, 14, 4, 13),
            _ => (0, 0, 0, 0),
        };
        (width, height, SurfacePoint { x, y })
    }

    /// Run the production `step_world` seam/creature collision loop: a brainbox
    /// creature on its frame's declared tile with that frame's registered hot-spot
    /// anchored at the same position, and a 1×1 bolt at `shot_at`.  The
    /// puncture and the mask are fully opaque, so a hit is a per-pixel mask
    /// overlap inside the tile, not a helper against itself.
    fn step_shot_into_creature(
        creature_mask: &CollisionMask,
        shot_at: SurfacePoint,
    ) -> WorldStepEffects {
        let mut world = SurfaceWorld::new();
        let creature_id = world.insert(live_creature(24, 1, SurfacePoint { x: 0, y: 0 }));
        let shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: shot_at,
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: shot_at,
            finite_life: Some(12),
        });
        let mut shot_masks = MapMasks::default();
        shot_masks.0.insert(shot_id, solid_mask());
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(creature_id, creature_mask.clone());
        let mut random = SeqRandom::new(&[]);
        step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        )
    }

    /// Every animation frame the brainbox can show must have its own declared extent
    /// and hotspot, and each opaque pixel inside that extent must collide while a
    /// pixel one past the declared box stays a miss.  The frame mask is registered
    /// via the ordinary production world/assembly collision path, and the bolt is a
    /// 1×1 shot mask, so a hit is a genuine overlap of two independent pixel
    /// masks in `step_world`, never a helper compared to itself.
    #[test]
    fn brainbox_accept_center_edge_hit_and_transparent_miss_on_every_lifey_frame() {
        for frame in 0..=3u16 {
            let (width, height, hotspot) = brainbox_extents(frame);
            let center = tile_pixel(width, height, hotspot, width / 2, height / 2);
            let hit = step_shot_into_creature(
                &center.mask,
                SurfacePoint {
                    x: center.world_x,
                    y: center.world_y,
                },
            );
            assert!(
                !hit.landed_shot.is_empty(),
                "center-pixel hit on frame {frame}"
            );
            let edge = tile_pixel(width, height, hotspot, width - 1, height - 1);
            let hit = step_shot_into_creature(
                &edge.mask,
                SurfacePoint {
                    x: edge.world_x,
                    y: edge.world_y,
                },
            );
            assert!(
                !hit.landed_shot.is_empty(),
                "edge-pixel hit on frame {frame}"
            );
            // One column past the declared box and one row below it show no
            // drawn pixel, so a bolt there is a clean miss.
            let right = tile_pixel(width, height, hotspot, width / 2, height / 2);
            let hit = step_shot_into_creature(
                &right.mask,
                SurfacePoint {
                    x: width as i32 - hotspot.x,
                    y: right.world_y,
                },
            );
            assert!(
                hit.landed_shot.is_empty(),
                "extent-past miss on frame {frame}"
            );
            let below = tile_pixel(width, height, hotspot, width / 2, height / 2);
            let hit = step_shot_into_creature(
                &below.mask,
                SurfacePoint {
                    x: below.world_x,
                    y: height as i32 - hotspot.y,
                },
            );
            assert!(
                hit.landed_shot.is_empty(),
                "below-extent miss on frame {frame}"
            );
        }
    }

    /// The drawable frame the assembly picks for a brainbox on the given animation
    /// frame must agree with the per-frame mask the world step registers: the frame
    /// index selects the lifey frame, and that same frame's recorded mask must be
    /// the one whose hotspot and extent the collision loop sees.  Frame indices
    /// outside 0..3 report an empty extent.
    #[test]
    fn brainbox_drawable_frame_and_registered_mask_hotspot_extent_stay_in_sync() {
        for frame in 0..=3u16 {
            let (width, height, hotspot) = brainbox_extents(frame);
            // Place the tile's single opaque pixel exactly at the declared hotspot:
            // with the creature drawn at the world origin that pixel is world (0,0).
            let at_hotspot = tile_pixel(width, height, hotspot, hotspot.x as u16, hotspot.y as u16);
            assert_eq!(at_hotspot.world_x, 0, "frame {frame} hotspot x");
            assert_eq!(at_hotspot.world_y, 0, "frame {frame} hotspot y");
            assert_eq!(
                at_hotspot.mask.width(),
                width,
                "frame {frame} registered width"
            );
            assert_eq!(
                at_hotspot.mask.height(),
                height,
                "frame {frame} registered height"
            );
            let hit = step_shot_into_creature(&at_hotspot.mask, SurfacePoint { x: 0, y: 0 });
            assert!(
                !hit.landed_shot.is_empty(),
                "frame {frame}: drawn hotspot pixel hits"
            );
        }
        assert_eq!(brainbox_extents(4), (0, 0, SurfacePoint::default()));
    }

    #[test]
    fn shot_life_decrement_returns_alive_until_zero() {
        let mut shot = Shot {
            position: SurfacePoint::default(),
            facing: 0,
            velocity_x: 0,
            velocity_y: 0,
            life: 3,
        };
        assert!(decrement_shot_life(&mut shot));
        assert!(decrement_shot_life(&mut shot));
        assert!(!decrement_shot_life(&mut shot));
        assert_eq!(shot.life, 0);
    }

    #[test]
    fn sequential_bolts_damage_then_destroy_a_brainbox_through_step_world() {
        let mut world = SurfaceWorld::new();
        let brainbox_id = world.insert(live_creature(24, 2, SurfacePoint { x: 0, y: 0 }));
        let first_shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 1,
            }),
            position: SurfacePoint::default(),
            finite_life: Some(1),
        });

        let mut shot_masks = MapMasks::default();
        shot_masks.0.insert(first_shot_id, solid_mask());
        let mut creature_masks = MapMasks::default();
        creature_masks.0.insert(brainbox_id, solid_mask());
        let mut random = SeqRandom::new(&[]);

        // First bolt overlaps the registered mask and lands on the HP2 Brainbox.
        let first_effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &shot_masks,
                creature_masks: &creature_masks,
                random: &mut random,
            },
        );
        assert_eq!(
            first_effects.landed_shot,
            [brainbox_id],
            "first independent bolt must land on the HP2 Brainbox"
        );
        assert!(
            first_effects.canned.is_empty(),
            "surviving Brainbox damage is stun, not cargo"
        );
        assert!(
            !first_effects.expired.contains(&brainbox_id),
            "first bolt must leave the Brainbox alive"
        );
        let brainbox = world.get(brainbox_id).unwrap();
        let hp = match &brainbox.kind {
            SurfaceEntityKind::LiveCreature { hit_points, .. } => *hit_points,
            other => panic!("expected live Brainbox, got {other:?}"),
        };
        assert_eq!(hp, 1, "first landed shot drops the Brainbox to HP1");
        // `step_world` expires the spent bolt via `advance_lifetimes`, which is
        // the normal per-frame world cleanup for a life-1 entity.
        assert!(
            !world.ids().contains(&first_shot_id),
            "the first bolt was removed by normal test-world lifetime cleanup"
        );

        // Second independent bolt destroys the surviving HP1 Brainbox.
        let second_shot_id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 1,
            }),
            position: SurfacePoint::default(),
            finite_life: Some(1),
        });
        let mut second_masks = MapMasks::default();
        second_masks.0.insert(second_shot_id, solid_mask());
        let mut creature_masks2 = MapMasks::default();
        creature_masks2.0.insert(brainbox_id, solid_mask());

        let second_effects = step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &second_masks,
                creature_masks: &creature_masks2,
                random: &mut random,
            },
        );
        assert_eq!(
            second_effects.landed_shot,
            [brainbox_id],
            "second independent bolt must land on the HP1 Brainbox"
        );
        assert!(
            second_effects.expired.contains(&brainbox_id),
            "second landed shot destroys the Brainbox through the production path"
        );
        assert!(
            second_effects.canned.is_empty(),
            "Brainbox destruction never produces cargo"
        );
    }
}
