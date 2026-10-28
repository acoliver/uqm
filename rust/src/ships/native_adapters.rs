//! Ship runtime operations formerly supplied by rust_bridge_ships.c.
fn wrap(value: i32, extent: i32) -> u16 {
    (if value < 0 {
        value + extent
    } else if value >= extent {
        value - extent
    } else {
        value
    }) as u16
}

unsafe fn display_primitive(
    state: *const crate::navigation::abi::GAME_STATE,
    index: u16,
) -> *mut crate::ships::native::abi::PRIMITIVE {
    (*state)
        .DisplayArray
        .cast::<crate::ships::native::abi::PRIMITIVE>()
        .add(usize::from(index))
}

fn recover_ship_crew(descriptor: &mut crate::ships::native::abi::RACE_DESC, crew: u16) -> u16 {
    let mut remaining = descriptor.ship_info.crew_level;
    let maximum = descriptor.ship_info.max_crew;
    if remaining != 0 {
        remaining = if i32::from(crew) >= i32::from(maximum) - i32::from(remaining) {
            maximum
        } else {
            remaining.wrapping_add(crew)
        };
    }
    descriptor.ship_info.crew_level = remaining;
    remaining
}

#[cfg(not(test))]
mod live {
    #![allow(non_snake_case)]
    use super::{display_primitive, recover_ship_crew, wrap};
    use crate::ships::native::{abi::*, cosine, normalize_facing, sine};
    use std::{mem, ptr};

    unsafe fn queue() -> *mut crate::collections::queue::Queue {
        ptr::addr_of_mut!(crate::engine::battle::legacy::disp_q).cast()
    }
    unsafe fn width() -> i32 {
        (ScreenWidth - 64) << 5
    }
    unsafe fn height() -> i32 {
        ScreenHeight << 5
    }

    macro_rules! operation {
    ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block) => {
        /// # Safety
        /// Called on the game thread with live queue, element, ship and resource handles.
        pub unsafe fn $name($($arg: $ty),*) $(-> $ret)? $body
    };
}
    operation!(uqm_get_current_activity_lobyte() -> u8 { crate::navigation::globals::GlobData.Game_state.CurrentActivity as u8 });
    operation!(rust_bridge_PutElement(handle: HELEMENT) { crate::collections::queue::PutQueue(queue(), handle); });
    operation!(rust_bridge_InsertElement(handle: HELEMENT, before: HELEMENT) { crate::collections::queue::InsertQueue(queue(), handle, before); });
    operation!(rust_bridge_GetHeadElement() -> HELEMENT { (*queue()).head });
    operation!(rust_bridge_GetTailElement() -> HELEMENT { (*queue()).tail });
    operation!(rust_bridge_LockElement(handle: HELEMENT, out: *mut *mut ELEMENT) { *out = handle.cast(); });
    operation!(rust_bridge_UnlockElement(_handle: HELEMENT) {});
    operation!(rust_bridge_GetPredElement(element: *mut ELEMENT) -> HELEMENT { (*element).pred });
    operation!(rust_bridge_GetSuccElement(element: *mut ELEMENT) -> HELEMENT { (*element).succ });
    operation!(rust_bridge_GetFrameIndex(frame: FRAME) -> u16 { crate::graphics::pixmap_abi::GetFrameIndex(frame.cast()) });
    operation!(rust_bridge_initialize_missile(block: *mut MISSILE_BLOCK) -> HELEMENT { crate::battle::weapon_native::initialize_missile(block) });
    operation!(rust_bridge_initialize_laser(block: *mut LASER_BLOCK) -> HELEMENT { crate::battle::weapon_native::initialize_laser(block) });
    operation!(rust_bridge_ship_intelligence(ship: *mut ELEMENT, objects: *mut EVALUATE_DESC, count: u16) { crate::battle::cyborg_native::ship_intelligence(ship, objects, count); });
    operation!(rust_bridge_SetAbsSoundIndex(sound: usize, index: u16) -> usize { crate::strings::string_table::SetAbsStringTableIndex(sound as *mut _, index) as usize });
    operation!(rust_bridge_DISPLAY_TO_WORLD(value: i32) -> i32 { value.wrapping_shl(2) });
    operation!(rust_bridge_WORLD_TO_DISPLAY(value: i32) -> i32 { value >> 2 });
    operation!(rust_bridge_NORMALIZE_FACING(value: u16) -> u16 { normalize_facing(i32::from(value)) });
    operation!(rust_bridge_FACING_TO_ANGLE(value: u16) -> u16 { value.wrapping_shl(2) });
    operation!(rust_bridge_SINE(angle: u16, magnitude: i16) -> i32 { i32::from(sine(i32::from(angle), i32::from(magnitude))) });
    operation!(rust_bridge_COSINE(angle: u16, magnitude: i16) -> i32 { i32::from(cosine(i32::from(angle), i32::from(magnitude))) });
    operation!(rust_bridge_ARCTAN(dx: i32, dy: i32) -> u16 { crate::math::arctan(dx as i16, dy as i16) });
    operation!(rust_bridge_WRAP_X(value: i32) -> i32 { i32::from(wrap(value, width())) });
    operation!(rust_bridge_WRAP_Y(value: i32) -> i32 { i32::from(wrap(value, height())) });
    operation!(rust_bridge_CollidingElement(element: *mut ELEMENT) -> u32 { u32::from((*element).state_flags & (NONSOLID | DISAPPEARING) as u16 == 0) });
    operation!(rust_bridge_OBJECT_CLOAKED(element: *mut ELEMENT) -> u32 { {
    let primitive = &*display_primitive(ptr::addr_of!(crate::navigation::globals::GlobData.Game_state), (*element).PrimIndex);
    u32::from(primitive.Type as u32 >= gfx_object_NUM_PRIMS || (primitive.Type as u32 == gfx_object_STAMPFILL_PRIM && primitive.color.r == 0 && primitive.color.g == 0 && primitive.color.b == 0 && primitive.color.a == 255))
} });
    operation!(rust_bridge_DeltaEnergy(element: *mut ELEMENT, delta: i16) -> u32 { crate::battle::status_native::DeltaEnergy(element, delta) });
    operation!(rust_bridge_DeltaCrew(element: *mut ELEMENT, delta: i16) -> u32 { crate::battle::status_native::DeltaCrew(element, delta) });
    operation!(rust_bridge_GetElementStarShip(element: *mut ELEMENT, ship: *mut *mut STARSHIP) { *ship = (*element).pParent.cast(); });
    operation!(rust_bridge_SetElementStarShip(element: *mut ELEMENT, ship: *mut STARSHIP) { (*element).pParent = ship.cast(); });
    operation!(rust_bridge_TrackShip(element: *mut ELEMENT, facing: *mut u16) -> i16 { crate::battle::weapon_native::TrackShip(element, facing) });
    operation!(rust_bridge_Untarget(element: *mut ELEMENT) { crate::battle::process_native::Untarget(element); });
    operation!(rust_bridge_ModifySilhouette(element: *mut ELEMENT, stamp: *mut STAMP, flags: u8) -> FRAME { crate::battle::weapon_native::ModifySilhouette(element, stamp, flags) });
    operation!(rust_bridge_ProcessSound(sound: usize, source: *mut ELEMENT) { crate::battle::sounds::ProcessSound(sound as *mut _, source); });
    operation!(rust_bridge_weapon_collision(e0: *mut ELEMENT, p0: *mut POINT, e1: *mut ELEMENT, p1: *mut POINT) -> HELEMENT { crate::battle::weapon_native::weapon_collision(e0, p0, e1, p1) });

    operation!(rust_bridge_spawn_element(ship: *mut STARSHIP, descriptor: *mut RACE_DESC, mass: u8, activity: u8) -> u32 {
        debug_assert_eq!(activity, uqm_get_current_activity_lobyte());
        let mut handle = (*ship).hShip;
        if handle.is_null() {
            handle = crate::battle::process_native::AllocElement();
            if !handle.is_null() { rust_bridge_InsertElement(handle, rust_bridge_GetHeadElement()); }
        }
        (*ship).hShip = handle;
        if handle.is_null() { return 0; }
        let element = handle.cast::<ELEMENT>();
        (*element).playerNr = (*ship).playerNr;
        (*element).__bindgen_anon_2.crew_level = 0;
        (*element).__bindgen_anon_3.mass_points = mass;
        (*element).state_flags = (APPEARING | PLAYER_SHIP | IGNORE_SIMILAR) as u16;
        (*element).__bindgen_anon_4.turn_wait = 0;
        (*element).__bindgen_anon_5.thrust_wait = 0;
        (*element).__bindgen_anon_1.life_span = NORMAL_LIFE as u16;
        (*element).colorCycleIndex = 0;
        let primitive = display_primitive(ptr::addr_of!(crate::navigation::globals::GlobData.Game_state), (*element).PrimIndex);
        (*primitive).Type = gfx_object_STAMP_PRIM as u8;
        let frames = crate::ships::ffi::rust_race_desc_get_ship_frames(descriptor.cast()).cast::<FRAME>();
        crate::collections::observation::image!(set; (*element).current.image.farray, frames);
        if (*element).playerNr == NPC_PLAYER_NUM as i16 && activity == IN_LAST_BATTLE as u8 {
            (*ship).ShipFacing = 0;
            crate::collections::observation::image!(frame; (*element).current.image.frame, SetAbsFrameIndex(*frames, 0));
            (*element).current.location.x = (width() >> 1) as i16;
            (*element).current.location.y = (height() >> 1) as i16;
            (*element).__bindgen_anon_1.life_span += 1;
        } else {
            (*ship).ShipFacing = normalize_facing(i32::from(crate::math::random() as u16));
            if inHQSpace() != 0 { (*ship).ShipFacing = crate::navigation::globals::GlobData.Game_state.ShipFacing.saturating_sub(1); }
            crate::collections::observation::image!(frame; (*element).current.image.frame, SetAbsFrameIndex(*frames, (*ship).ShipFacing));
            loop {
                (*element).current.location.x = wrap(((i32::from(crate::math::random() as u16) % width()) as i16 & !3) as i32, width()) as i16;
                (*element).current.location.y = wrap(((i32::from(crate::math::random() as u16) % height()) as i16 & !3) as i32, height()) as i16;
                if CalculateGravity(element) == 0 && TimeSpaceMatterConflict(element) == 0 { break; }
            }
        }
        (*element).preprocess_func = Some(ElementHook::Legacy(crate::battle::ship_native::ship_preprocess));
        (*element).postprocess_func = Some(ElementHook::Legacy(crate::battle::ship_native::ship_postprocess));
        (*element).death_func = Some(ElementHook::Legacy(crate::battle::tactrans_native::ship_death));
        (*element).collision_func = Some(CollisionHook::Legacy(crate::battle::ship_native::collision));
        (*element).velocity = mem::zeroed();
        (*element).pParent = ship.cast();
        (*element).hTarget = ptr::null_mut();
        1
    });

    operation!(rust_bridge_init_battle_arena() -> i16 {
        if crate::game_init::space_resources::InitSpace() == 0 { return 0; }
        SetContext(StatusContext); SetContext(SpaceContext);
        InitDisplayList(); crate::battle::galaxy::InitGalaxy();
        let races = ptr::addr_of_mut!(crate::engine::battle::legacy::race_q).cast::<QUEUE>();
        if inHQSpace() != 0 {
            ReinitQueue(races); ReinitQueue(races.add(1));
            let flagship = Build(races, SPECIES_ID_SIS_SHIP_ID).cast::<STARSHIP>();
            if !flagship.is_null() { (*flagship).playerNr = RPG_PLAYER_NUM as i16; (*flagship).captains_name_index = 0; }
            crate::navigation::hyper_data::LoadHyperspace();
            1
        } else {
            crate::graphics::drawable_abi::SetContextFGFrame(Screen.cast());
            let mut rect = crate::graphics::native_abi::RECT { corner: crate::graphics::native_abi::POINT { x: 0, y: 0 }, extent: crate::graphics::native_abi::EXTENT { width: (ScreenWidth - 64) as i16, height: ScreenHeight as i16 } };
            crate::graphics::context_abi::SetContextClipRect(&mut rect);
            SetContextBackGroundColor(Color {r:0,g:0,b:0,a:255});
            let old = SetContext(ScreenContext);
            SetContextBackGroundColor(Color {r:0,g:0,b:0,a:255}); ClearDrawable(); SetContext(old);
            if uqm_get_current_activity_lobyte() == IN_LAST_BATTLE as u8 { crate::game_init::world_resources::free_gravity_well(); }
            else { for _ in 0..5 { crate::battle::world_elements::spawn_asteroid(ptr::null_mut()); } crate::battle::world_elements::spawn_planet(); }
            2
        }
    });

    operation!(rust_bridge_uninit_ships() {
        StopSound(); crate::game_init::space_resources::UninitSpace();
        let mut ships: [*mut STARSHIP; 2] = [ptr::null_mut(); 2];
        let mut crew = 0u16;
        let mut handle = rust_bridge_GetHeadElement();
        while !handle.is_null() { let element = &*handle.cast::<ELEMENT>(); if element.state_flags & CREW_OBJECT as u16 != 0 { crew = crew.wrapping_add(1); } handle = element.succ; }
        handle = rust_bridge_GetHeadElement();
        while !handle.is_null() {
            let element = &mut *handle.cast::<ELEMENT>(); handle = element.succ;
            if element.state_flags & PLAYER_SHIP as u16 != 0 || element.death_func.is_some_and(|callback| callback.address() == crate::battle::tactrans_native::new_ship as *const () as usize) {
                let ship = element.pParent.cast::<STARSHIP>();
                if ship.is_null() || (*ship).RaceDescPtr.is_null() { continue; }
                let descriptor = (*ship).RaceDescPtr;
                (*ship).crew_level = recover_ship_crew(&mut *descriptor, crew);
                ships[(*ship).playerNr as usize] = ship;
                crate::ships::loader_native::free_ship(descriptor, 1, 1);
                (*ship).RaceDescPtr = ptr::null_mut();
            }
        }
        crate::navigation::globals::GlobData.Game_state.CurrentActivity &= !(IN_BATTLE as u16);
        if uqm_get_current_activity_lobyte() == IN_ENCOUNTER as u8 && crate::navigation::globals::GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 == 0 {
            for i in (0..2).rev() { if !ships[i].is_null() && FleetIsInfinite(i as u16) == 0 { UpdateShipFragCrew(ships[i]); } }
        }
        if uqm_get_current_activity_lobyte() != IN_ENCOUNTER as u8 {
            let races = ptr::addr_of_mut!(crate::engine::battle::legacy::race_q).cast::<QUEUE>();
            ReinitQueue(races); ReinitQueue(races.add(1));
            if inHQSpace() != 0 { crate::navigation::hyper_data::FreeHyperspace(); }
        }
    });

    operation!(rust_bridge_get_race_desc_layout(out: *mut crate::ships::ffi_contract::RaceDescLayout) {
        *out = crate::ships::ffi_contract::RaceDescLayout {
            race_desc_size: mem::size_of::<RACE_DESC>(), ship_data_offset: mem::offset_of!(RACE_DESC, ship_data), ship_info_offset: mem::offset_of!(RACE_DESC, ship_info), characteristics_offset: mem::offset_of!(RACE_DESC, characteristics), ship_data_ship_offset: mem::offset_of!(DATA_STUFF, ship), ship_info_crew_offset: mem::offset_of!(SHIP_INFO, crew_level), ship_info_max_crew_offset: mem::offset_of!(SHIP_INFO, max_crew), characteristics_mass_offset: mem::offset_of!(CHARACTERISTIC_STUFF, ship_mass),
        };
    });
}
#[cfg(not(test))]
pub use live::*;

#[cfg(test)]
mod tests {
    use super::{display_primitive, wrap};
    #[test]
    fn display_primitive_uses_the_pointed_to_array() {
        use crate::ships::native::abi::{gfx_object_STAMP_PRIM, PRIMITIVE};
        let mut primitives: [PRIMITIVE; 3] = unsafe { std::mem::zeroed() };
        let mut state: crate::navigation::abi::GAME_STATE = unsafe { std::mem::zeroed() };
        state.DisplayArray = primitives.as_mut_ptr().cast();
        let original = state.DisplayArray;
        unsafe {
            (*display_primitive(&state, 2)).Type = gfx_object_STAMP_PRIM as u8;
        }
        assert_eq!(state.DisplayArray, original);
        assert_eq!(primitives[0].Type, 0);
        assert_eq!(primitives[1].Type, 0);
        assert_eq!(primitives[2].Type, gfx_object_STAMP_PRIM as u8);
    }

    #[test]
    fn world_wrap_uses_one_adjustment_not_modulo() {
        assert_eq!(wrap(-1, 8192), 8191);
        assert_eq!(wrap(8191, 8192), 8191);
        assert_eq!(wrap(8192, 8192), 0);
        assert_eq!(wrap(16385, 8192), 8193);
    }
}

#[cfg(test)]
mod crew_writeback_tests {
    use super::recover_ship_crew;
    use crate::ships::native::abi::RACE_DESC;

    #[test]
    fn battle_survivor_retains_native_descriptor_crew_and_recovers_pods() {
        let mut descriptor: RACE_DESC = unsafe { std::mem::zeroed() };
        descriptor.ship_info.crew_level = 7;
        descriptor.ship_info.max_crew = 18;
        assert_eq!(recover_ship_crew(&mut descriptor, 3), 10);
        assert_eq!(descriptor.ship_info.crew_level, 10);
        assert_eq!(descriptor.ship_info.max_crew, 18);
        assert_eq!(recover_ship_crew(&mut descriptor, 20), 18);
        descriptor.ship_info.crew_level = 0;
        assert_eq!(recover_ship_crew(&mut descriptor, 20), 0);
    }
}
