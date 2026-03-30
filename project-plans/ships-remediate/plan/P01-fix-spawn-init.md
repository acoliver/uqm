# Phase 1: Fix spawn_ship and InitShips/UninitShips

## Purpose
`spawn_ship` creates a RaceDesc but never creates an ELEMENT.
`InitShips`/`UninitShips` don't set up the battle arena.
Fix both so Rust controls the full battle lifecycle.

## spawn_ship (lifecycle.rs)

Replace the dead `_element_config` with actual element creation:

```rust
pub fn spawn_ship(starship: &mut Starship, activity: u8) -> Result<SpawnResult, ShipsError> {
    // ... existing load + crew patch + counter clear ...

    // Create element via FFI
    let h_ship = if starship.h_ship == 0 {
        let h = unsafe { c_AllocElement() };
        if h == 0 { return Err(ShipsError::LoadFailed("AllocElement failed")); }
        unsafe { c_InsertElement(h, c_GetHeadElement()); }
        h
    } else {
        starship.h_ship
    };
    starship.h_ship = h_ship;

    // Configure element
    unsafe {
        let mut elem_ptr: *mut Element = std::ptr::null_mut();
        c_LockElement(h_ship, &mut elem_ptr);
        
        (*elem_ptr).playerNr = starship.player_nr;
        (*elem_ptr).crew_level = 0;
        (*elem_ptr).mass_points = desc.characteristics.ship_mass;
        (*elem_ptr).state_flags = APPEARING | PLAYER_SHIP | IGNORE_SIMILAR;
        (*elem_ptr).turn_wait = 0;
        (*elem_ptr).thrust_wait = 0;
        (*elem_ptr).life_span = NORMAL_LIFE;
        (*elem_ptr).colorCycleIndex = 0;
        
        // Set image
        c_SetPrimType(elem_ptr, STAMP_PRIM);
        (*elem_ptr).image_farray = desc.ship_data.ship[0];
        
        // Set position + facing
        // ... handle Sa-Matra, HyperSpace, normal random spawn ...
        
        // Set callbacks to C's ship_preprocess/ship_postprocess/ship_death/collision
        (*elem_ptr).preprocess_func = ship_preprocess;
        (*elem_ptr).postprocess_func = ship_postprocess;
        (*elem_ptr).death_func = ship_death;
        (*elem_ptr).collision_func = collision;
        
        // Zero velocity, set starship link
        c_ZeroVelocityComponents(elem_ptr);
        c_SetElementStarShip(elem_ptr, starship_c_ptr);
        (*elem_ptr).hTarget = 0;
        
        c_UnlockElement(h_ship);
    }
    
    starship.race_desc = Some(Box::new(desc));
    Ok(SpawnResult::Spawned)
}
```

## InitShips (lifecycle.rs or new init.rs)

Port the C `InitShips` logic:

```rust
pub fn init_ships(activity: u8) -> Result<u32, ShipsError> {
    init_space()?;
    
    unsafe {
        c_SetContext(c_StatusContext());
        c_SetContext(c_SpaceContext());
        c_InitDisplayList();
        c_InitGalaxy();
    }
    
    if in_hq_space(activity) {
        unsafe {
            c_ReinitQueue(&race_q[0]);
            c_ReinitQueue(&race_q[1]);
            c_BuildSIS();
            c_LoadHyperspace();
        }
        Ok(1)
    } else {
        // Set up battle arena
        unsafe {
            c_SetContextFGFrame(c_Screen());
            // ... set clip rect ...
            c_SetContextBackGroundColor(BLACK_COLOR);
            c_ClearDrawable();
        }
        
        if activity == IN_LAST_BATTLE {
            unsafe { c_free_gravity_well(); }
        } else {
            for _ in 0..5 { unsafe { c_spawn_asteroid(std::ptr::null_mut()); } }
            for _ in 0..1 { unsafe { c_spawn_planet(); } }
        }
        
        Ok(NUM_SIDES)
    }
}
```

Note: InitShips calls many C arena/display functions. These remain as C FFI calls
because the display system itself is not yet ported. The LOGIC is in Rust.

## UninitShips

Port the C cleanup: stop sound, uninit space, count floating crew,
iterate elements to find surviving ship, write back crew, free descriptors,
clear IN_BATTLE flag, handle encounter vs non-encounter.

## Re-enable USE_RUST_SHIPS

Restore the `#ifdef USE_RUST_SHIPS` guards in:
- `sc2/src/uqm/ship.c` — spawn_ship, ship_preprocess, ship_postprocess
- `sc2/src/uqm/init.c` — InitShips, UninitShips

## Verification
- `cargo check` passes
- Super melee: ships appear on screen (element created)
- Ships don't move/fight yet (behaviors still stubs) — that's Phase 3+
- Battle exits cleanly (UninitShips)
