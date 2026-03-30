# Phase 2: Weapon Creation Bridge

## Purpose
Almost every ship creates projectiles via `initialize_missile` or `initialize_laser`.
These C functions allocate an ELEMENT, set its velocity/image/callbacks, and return
an element handle. Ships need Rust wrappers to call these.

## MissileBlock repr(C)

```rust
#[repr(C)]
pub struct MissileBlock {
    pub cx: i32,            // origin x (world coords)
    pub cy: i32,            // origin y
    pub farray: FrameArrayPtr, // weapon animation frames
    pub face: u16,          // facing direction
    pub index: u16,         // animation frame index
    pub sender: i16,        // playerNr of shooter
    pub flags: u16,         // element state flags
    pub pixoffs: u16,       // pixel offset from ship center
    pub speed: u16,         // projectile speed
    pub hit_points: u8,     // projectile HP
    pub damage: u8,         // damage dealt on hit
    pub life: u16,          // frames until expiry
    pub preprocess_func: Option<extern "C" fn(*mut Element)>,
    pub blast_offs: u16,    // blast animation offset
}
```

## LaserBlock repr(C)

```rust
#[repr(C)]
pub struct LaserBlock {
    pub cx: i32,
    pub cy: i32,
    pub face: u16,
    pub ex: i32,            // endpoint x (relative)
    pub ey: i32,            // endpoint y (relative)
    pub sender: i16,
    pub flags: u16,
    pub pixoffs: u16,
    pub color: u32,         // BUILD_COLOR value
}
```

## Safe Rust wrappers

```rust
pub fn create_missile(block: &MissileBlock) -> Option<HElement> {
    let h = unsafe { c_initialize_missile(block as *const MissileBlock) };
    if h == 0 { None } else { Some(h) }
}

pub fn create_laser(block: &LaserBlock) -> Option<HElement> {
    let h = unsafe { c_initialize_laser(block as *const LaserBlock) };
    if h == 0 { None } else { Some(h) }
}
```

## ShipBehavior trait update

`init_weapon` currently returns `Vec<WeaponElement>` (a Rust-only struct).
This needs to change: weapons must create real C ELEMENTs via the missile/laser
bridge. Options:

**Option A**: Change return to `Vec<HElement>` — ship creates elements directly.
**Option B**: Keep `Vec<WeaponElement>` and have runtime convert to C elements.

Option A is simpler and matches C: C's `init_weapon_func` fills `HELEMENT Weapon[6]`.
The `ShipBehavior::init_weapon` should do the same.

```rust
fn init_weapon(
    &mut self,
    ship: &ShipState,
    ctx: &BattleContext,
) -> Result<Vec<HElement>, ShipsError> {
    Ok(vec![])
}
```

## Verification
- `cargo check` passes
- MissileBlock/LaserBlock layout matches C (static_assert size/alignment)
- create_missile/create_laser callable from test with null pointers gracefully
