use super::*;

pub(super) unsafe fn draw_blue_print(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut MENU_STATE,
    flagship: &crate::engine::world::SisState,
    context: &crate::graphics::context_abi::ContextLease,
) {
    // SAFETY: the shipyard retains its game-thread context, menu and loaded resources.
    unsafe {
        let frame = CaptureDrawable(LoadGraphicInstance(c"graphics.blueprints".as_ptr()).cast());
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: DecFrameIndex(frame),
        };
        SetContextForeGroundColor(rgb15(0, 0, 0x16));
        DrawFilledStamp(&mut stamp);
        for slot in 0..NUM_DRIVE_SLOTS {
            let piece = flagship.drive_slots[slot as usize];
            cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
        }
        for slot in 0..NUM_JET_SLOTS {
            let piece = flagship.jet_slots[slot as usize];
            cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
        }
        for slot in 0..NUM_MODULE_SLOTS {
            let piece = flagship.module_slots[slot as usize];
            if !(menu.CurState as u32 == SHIPYARD && piece as u32 == CREW_POD) {
                cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
            }
        }
        SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x1f));
        for slot in 0..NUM_MODULE_SLOTS {
            let piece = flagship.module_slots[slot as usize];
            if menu.CurState as u32 == SHIPYARD && piece as u32 == CREW_POD {
                cx.with_legacy(|| DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1));
            }
        }
        super::sis::capacity::draw_blueprint_cells(
            context,
            flagship,
            input.preferences().fonts == OPT_PC as i32,
        );
        DestroyDrawable(ReleaseDrawable(frame));
    }
}
