# Phase 0: Battle Engine FFI Bridge

## Purpose
Ships call ~15 C battle engine functions to create projectiles, play sounds,
manipulate elements, etc. Rust wrappers for these must exist before any ship
behavior can be ported. Many already exist in `battle/c_bridge.rs` — this phase
fills the gaps.

## Files to modify
- `rust/src/battle/c_bridge.rs` — add missing extern declarations + safe wrappers
- `rust/src/ships/c_bridge.rs` — may need additions for ship-specific APIs

## Functions to add (those NOT already in c_bridge.rs)

### Element manipulation
```rust
extern "C" {
    fn AllocElement() -> HElement;
    fn PutElement(h: HElement);
    fn InsertElement(h: HElement, after: HElement);
    fn LockElement(h: HElement, ptr: *mut *mut Element);
    fn UnlockElement(h: HElement);
    fn GetHeadElement() -> HElement;
    fn GetTailElement() -> HElement;
    fn GetSuccElement(elem: *const Element) -> HElement;
    fn GetPredElement(elem: *const Element) -> HElement;
    fn RemoveElement(h: HElement);
    fn GetElementStarShip(elem: *const Element, ss: *mut *mut CStarship);
    fn SetElementStarShip(elem: *mut Element, ss: *mut CStarship);
}
```

### Weapon creation
```rust
extern "C" {
    fn initialize_missile(block: *const MissileBlock) -> HElement;
    fn initialize_laser(block: *const LaserBlock) -> HElement;
}
```

### Sound
```rust
extern "C" {
    fn ProcessSound(sound: uintptr_t, elem: *const Element);
    fn SetAbsSoundIndex(sounds: uintptr_t, index: u32) -> uintptr_t;
    fn GetSoundCount(sounds: uintptr_t) -> u32;
}
```

### Random
```rust
extern "C" {
    fn TFB_Random() -> u32;
}
```

### Coordinate macros (port as Rust const fn)
```rust
pub const fn display_to_world(x: i32) -> i32 { x << 1 }  // DISPLAY_TO_WORLD
pub const fn world_to_display(x: i32) -> i32 { x >> 1 }  // WORLD_TO_DISPLAY
pub const fn normalize_facing(f: i32) -> u16 { (f & 15) as u16 }  // NORMALIZE_FACING
pub const fn facing_to_angle(f: u16) -> u16 { f * (256/16) }  // FACING_TO_ANGLE
```

### AI framework
```rust
extern "C" {
    fn ship_intelligence(ship: *mut Element, objects: *mut EvaluateDesc, count: u32);
}
```

## Repr(C) structs needed
- `MissileBlock` — matches C `MISSILE_BLOCK`
- `LaserBlock` — matches C `LASER_BLOCK`
- `EvaluateDesc` — matches C `EVALUATE_DESC`

## Verification
- `cargo check` passes
- `cargo test` passes (existing tests don't break)
- Each FFI function has a `#[cfg(test)]` stub or is behind `#[cfg(not(test))]`
