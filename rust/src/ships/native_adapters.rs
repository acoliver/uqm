//! Ship runtime operations formerly supplied by rust_bridge_ships.c.

#[cfg(not(test))]
use crate::engine::Kernel as KernelOwner;
#[cfg(not(test))]
fn remaining_battle_crew(cx: &crate::engine::battle::Cx<'_>) -> u16 {
    let mut crew = 0u16;
    let mut next = cx.elements().head();
    while let Some(id) = next {
        if cx.element(id).flags & crate::ships::native::abi::CREW_OBJECT as u16 != 0 {
            crew = crew.wrapping_add(1);
        }
        next = cx.elements().succ(id);
    }
    crew
}

#[cfg(not(test))]
fn retire_element_ship(
    cx: &mut crate::engine::battle::Cx<'_>,
    id: crate::engine::battle::ElementId,
    crew: u16,
    replacement: crate::engine::battle::ElementHook,
) -> Option<(
    crate::engine::battle::ShipId,
    crate::engine::battle::RaceDesc,
)> {
    use crate::engine::battle::Parent;
    let element = cx.element(id);
    if element.flags & crate::ships::native::abi::PLAYER_SHIP as u16 == 0
        && !element
            .death
            .is_some_and(|callback| callback.address() == replacement.address())
    {
        return None;
    }
    let Parent::Ship(ship) = cx.parent(id) else {
        return None;
    };
    let mut descriptor = cx.take_descriptor(ship)?;
    cx.ship_state_mut(ship).crew = descriptor.recover_crew(crew);
    Some((ship, descriptor))
}

/// # Safety
/// Call on the game thread while the battle queues and their resources remain live.
#[cfg(not(test))]
pub unsafe fn uninit_ships() {
    use crate::ships::native::abi::*;

    StopSound();
    KernelOwner::current().write(|kernel| crate::game_init::space_resources::uninit_space(kernel));
    crate::app::bridge::enter(|cx| {
        let mut ships = [None; 2];
        let crew = remaining_battle_crew(cx);
        let replacement =
            crate::engine::battle::ElementHook::Native(crate::battle::tactrans_native::new_ship);
        let mut next = cx.elements().head();
        while let Some(id) = next {
            next = cx.elements().succ(id);
            if let Some((ship, descriptor)) = retire_element_ship(cx, id, crew, replacement) {
                ships[cx.ship_state(ship).player as usize] = Some(ship);
                crate::ships::loader_native::unload_ship(cx.world, descriptor, 1, 1);
            }
        }
        let activity = cx.world.activity() & !(IN_BATTLE as u16);
        cx.world.set_activity(activity);
        if activity as u8 == IN_ENCOUNTER as u8 && activity & CHECK_ABORT as u16 == 0 {
            for i in (0..2).rev() {
                if let Some(ship) = ships[i].filter(|_| fleet_is_infinite(cx.world, i as u16) == 0)
                {
                    crate::battle::encount_native::engine::update_ship_frag_crew(cx, ship);
                }
            }
        }
        for side in 0..2 {
            let mut next = cx.first_ship(side);
            while let Some(ship) = next {
                next = cx.next_ship(ship);
                if let Some(descriptor) = cx.take_descriptor(ship) {
                    crate::ships::loader_native::unload_ship(cx.world, descriptor, 1, 1);
                }
            }
        }
        if cx.world.activity() as u8 != IN_ENCOUNTER as u8 {
            cx.reset_ships(0);
            cx.reset_ships(1);
            if in_hqspace(cx.world) != 0 {
                crate::navigation::hyper_data::FreeHyperspace();
            }
        }
        cx.complete_ship_reset();
    });
}

#[cfg(not(test))]
mod live {
    use crate::engine::World as WorldOwner;
    use crate::engine::{Kernel as KernelOwner, WorldData as WorldFields};
    use crate::ships::native::abi::*;

    macro_rules! operation {
    ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? $body:block) => {
        /// # Safety
        /// Called on the game thread with live queue, element, ship and resource handles.
        pub unsafe fn $name($($arg: $ty),*) $(-> $ret)? $body
    };
}
    operation!(uqm_get_current_activity_lobyte() -> u8 { (**WorldFields::current()).game.current_activity as u8 });

    operation!(rust_bridge_init_battle_arena() -> i16 {
        if KernelOwner::current().write(|kernel| crate::game_init::space_resources::init_space(kernel, WorldOwner::current().read(|world|world.activity()))) == 0 { return 0; }
        SetContext((**KernelOwner::current()).status_context); SetContext((**KernelOwner::current()).space_context);
        crate::app::bridge::enter(|cx| crate::battle::process_native::init_display_list(cx, optMeleeScale == 0)); crate::app::bridge::enter(crate::battle::galaxy::init_galaxy);
        if WorldOwner::current().read(in_hqspace) != 0 {
            crate::app::bridge::enter(|cx| {
                cx.reset_ships(0); cx.reset_ships(1);
                let mut ships = crate::engine::battle::link_queue::ships(0);
                if let Some(flagship) = ships.build(SPECIES_ID_SIS_SHIP_ID) { ships.edit(flagship, |ship| { ship.player = RPG_PLAYER_NUM as i16; ship.captain = 0; }); }
            });
            crate::navigation::hyper_data::LoadHyperspace();
            1
        } else {
            crate::graphics::drawable_abi::SetContextFGFrame((**KernelOwner::current()).screen.cast());
            let mut rect = crate::graphics::native_abi::RECT { corner: crate::graphics::native_abi::POINT { x: 0, y: 0 }, extent: crate::graphics::native_abi::EXTENT { width: (ScreenWidth - 64) as i16, height: ScreenHeight as i16 } };
            crate::graphics::context_abi::SetContextClipRect(&mut rect);
            SetContextBackGroundColor(Color {r:0,g:0,b:0,a:255});
            let old = SetContext((**KernelOwner::current()).screen_context);
            SetContextBackGroundColor(Color {r:0,g:0,b:0,a:255}); ClearDrawable(); SetContext(old);
            if uqm_get_current_activity_lobyte() == IN_LAST_BATTLE as u8 { KernelOwner::current().write(|kernel| crate::game_init::world_resources::free_gravity_well(kernel)); }
            else { crate::app::bridge::enter(|cx| { for _ in 0..5 { crate::battle::world_elements::create_asteroid(cx, None); } crate::battle::world_elements::spawn_planet(cx); }); }
            2
        }
    });
}
#[cfg(not(test))]
pub use live::*;

#[cfg(test)]
mod crew_writeback_tests {
    use crate::engine::battle::record::recover_ship_crew;
    use crate::ships::native::abi::RaceDescriptor;

    #[test]
    fn battle_survivor_retains_native_descriptor_crew_and_recovers_pods() {
        let mut descriptor: RaceDescriptor = unsafe { std::mem::zeroed() };
        descriptor.ship_info.crew_level = 7;
        descriptor.ship_info.max_crew = 18;
        assert_eq!(
            recover_ship_crew(
                &mut descriptor.ship_info.crew_level,
                descriptor.ship_info.max_crew,
                3
            ),
            10
        );
        assert_eq!(descriptor.ship_info.crew_level, 10);
        assert_eq!(descriptor.ship_info.max_crew, 18);
        assert_eq!(
            recover_ship_crew(
                &mut descriptor.ship_info.crew_level,
                descriptor.ship_info.max_crew,
                20
            ),
            18
        );
        descriptor.ship_info.crew_level = 0;
        assert_eq!(
            recover_ship_crew(
                &mut descriptor.ship_info.crew_level,
                descriptor.ship_info.max_crew,
                20
            ),
            0
        );
    }
}
