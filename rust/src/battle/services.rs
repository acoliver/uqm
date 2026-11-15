//! Battle-mode presentation supplied explicitly by application composition.
#[cfg(not(test))]
use crate::engine::battle::services::PrimitiveServices;
#[cfg(not(test))]
use crate::engine::battle::views::FrameSelection;
#[cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, ShipId};
#[cfg(not(test))]
use crate::graphics::native_abi::{POINT, RECT, SIZE};
#[cfg(not(test))]
use crate::ships::native::abi::OPT_PC;

#[cfg(not(test))]
pub fn install(kernel: &mut crate::engine::Kernel) {
    kernel.install_primitive_services(&ModeServices);
}
#[cfg(not(test))]
struct ModeServices;
#[cfg(not(test))]
impl PrimitiveServices for ModeServices {
    fn draw_statistics(&self, cx: &mut Cx<'_>, ship: ShipId, crew: i16, energy: i16) {
        if crew == 0 && energy == 0 {
            return;
        }
        let y = cx.status_offset(ship);
        let context = std::ptr::NonNull::new(cx.kernel.status_context)
            .map(crate::graphics::context_abi::ContextLease::retain)
            .and_then(|context| context.selected_context());
        let font = cx.kernel.battle_font();
        let mut info = cx.desc(ship).ship_info;
        crate::ui::shipstat::update_statistics(
            context.as_ref(),
            true,
            cx.kernel.configuration.preferences().menu == OPT_PC as i32,
            font.as_ref(),
            &mut info,
            y,
            (crew, energy),
        );
        let mut stats = cx.race_stats_mut(ship);
        stats.ship_info.crew_level = info.crew_level;
        stats.ship_info.energy_level = info.energy_level;
    }
    fn init_ship_status(&self, cx: &mut Cx<'_>, ship: ShipId, clip: Option<RECT>) {
        crate::ui::shipstat::draw_battle_status(cx, ship, clip);
        if clip.is_none() {
            let context = std::ptr::NonNull::new(cx.kernel.status_context)
                .map(crate::graphics::context_abi::ContextLease::retain);
            let height = (cx.screen_dimensions().1 >> 1) as SIZE;
            let pc_menu = cx.kernel.configuration.preferences().menu == OPT_PC as i32;
            crate::battle::status_native::draw_captains_window(
                cx,
                ship,
                context.as_ref(),
                height,
                pc_menu,
            );
        }
    }
    fn silhouette_at(
        &self,
        cx: &mut Cx<'_>,
        source: ElementId,
        origin: POINT,
        frame: FrameSelection,
        flags: u8,
    ) {
        crate::battle::weapon_native::modify_silhouette(cx, source, origin, frame, flags);
    }
}
