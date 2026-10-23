//! Flagship screen border, ported from uqm/border.c.
#![allow(non_snake_case)]
use super::util::live::DrawStarConBox;
use crate::navigation::abi::*;
fn color(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
/// # Safety
/// Initialized graphics and SIS contexts must be accessed on the game thread.
pub unsafe fn InitSISContexts() {
    let mut r: RECT = std::mem::zeroed();

    SetContext(StatusContext);

    SetContext(SpaceContext);
    SetContextFGFrame(Screen);

    r.corner.x = 7;
    r.corner.y = 10;
    r.extent.width = (ScreenWidth - 78) as i16;
    r.extent.height = (ScreenHeight - 13) as i16;
    SetContextClipRect(&mut r);
}

/// # Safety
/// Initialized graphics and SIS contexts must be accessed on the game thread.
pub unsafe fn DrawSISFrame() {
    let mut r: RECT = std::mem::zeroed();

    SetContext(ScreenContext);

    BatchGraphics();
    {
        SetContextForeGroundColor(color(0x0A, 0x0A, 0x0A));
        r.corner.x = 0;
        r.corner.y = 0;
        r.extent.width = 7 + ((ScreenWidth - 78) as i16) + 1;
        r.extent.height = 10 - 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = 0;
        r.corner.y = 0;
        r.extent.width = 7 - 1;
        r.extent.height = 10 + ((ScreenHeight - 13) as i16) + 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = 0;
        r.corner.y = r.extent.height;
        r.extent.width = 7 + ((ScreenWidth - 78) as i16) + 1;
        r.extent.height = (ScreenHeight as i16) - 10 + ((ScreenHeight - 13) as i16);
        DrawFilledRectangle(&mut r);
        r.corner.x = 7 + ((ScreenWidth - 78) as i16) + 1;
        r.corner.y = 0;
        r.extent.width = (ScreenWidth as i16) - r.corner.x;
        r.extent.height = ScreenHeight as i16;
        DrawFilledRectangle(&mut r);

        r.corner.x = 7 - 1;
        r.corner.y = 10 - 1;
        r.extent.width = ((ScreenWidth - 78) as i16) + 2;
        r.extent.height = ((ScreenHeight - 13) as i16) + 2;
        DrawStarConBox(
            &mut r,
            1,
            color(0x10, 0x10, 0x10),
            color(0x08, 0x08, 0x08),
            1,
            color(0, 0, 0),
        );

        r.corner.y = 0;
        r.extent.height = 10;

        r.corner.x = 7;
        r.extent.width = (ScreenWidth - 147) as i16;
        DrawStarConBox(
            &mut r,
            1,
            color(0x00, 0x00, 0x0E),
            color(0x00, 0x01, 0x1C),
            1,
            color(0x00, 0x00, 0x14),
        );

        r.extent.width = 57;
        r.corner.x = 7 + ((ScreenWidth - 78) as i16) - 57;
        DrawStarConBox(
            &mut r,
            1,
            color(0x00, 0x00, 0x0E),
            color(0x00, 0x01, 0x1C),
            1,
            color(0x00, 0x00, 0x14),
        );

        SetContextForeGroundColor(color(0, 0, 0));
        r.corner.x = ((ScreenWidth - 64) as i16) - 1;
        r.corner.y = 0;
        r.extent.width = 1;
        r.extent.height = ScreenHeight as i16;
        DrawFilledRectangle(&mut r);
        r.corner.x = (ScreenWidth - 64) as i16;
        r.corner.y = 139;
        DrawPoint(&mut r.corner);
        r.corner.x = (ScreenWidth as i16) - 1;
        DrawPoint(&mut r.corner);

        SetContextForeGroundColor(color(0x10, 0x10, 0x10));
        r.corner.y = 1;
        r.extent.width = 1;
        r.extent.height = 8;
        r.corner.x = 7 - 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = 7 + ((ScreenWidth - 78) as i16) - 57 - 1;
        DrawFilledRectangle(&mut r);

        r.corner.x = 0;
        r.corner.y = (ScreenHeight as i16) - 1;
        r.extent.width = ((ScreenWidth - 64) as i16) - 1;
        r.extent.height = 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = ((ScreenWidth - 64) as i16) - 2;
        r.corner.y = 0;
        r.extent.width = 1;
        r.extent.height = (ScreenHeight as i16) - 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = (ScreenWidth as i16) - 1;
        r.corner.y = 0;
        r.extent.width = 1;
        r.extent.height = 139;
        DrawFilledRectangle(&mut r);
        r.corner.x = (ScreenWidth - 64) as i16;
        r.corner.y = (ScreenHeight as i16) - 1;
        r.extent.width = (ScreenWidth as i16) - r.corner.x;
        r.extent.height = 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = (ScreenWidth as i16) - 1;
        r.corner.y = 140;
        r.extent.width = 1;
        r.extent.height = ((ScreenHeight as i16) - 1) - r.corner.y;
        DrawFilledRectangle(&mut r);

        SetContextForeGroundColor(color(0x08, 0x08, 0x08));
        r.corner.y = 1;
        r.extent.width = 1;
        r.extent.height = 8;
        r.corner.x = 7 + ((ScreenWidth - 147) as i16);
        DrawFilledRectangle(&mut r);
        r.corner.x = 7 + ((ScreenWidth - 78) as i16);
        r.extent.height += 1;
        DrawFilledRectangle(&mut r);
        r.corner.y = 0;
        r.extent.width = (((ScreenWidth - 64) as i16) - 2) - r.corner.x;
        r.extent.height = 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = 0;
        r.extent.width = 7 - r.corner.x;
        DrawFilledRectangle(&mut r);
        r.corner.x = 7 + ((ScreenWidth - 147) as i16);
        r.extent.width = 12;
        DrawFilledRectangle(&mut r);

        r.corner.x = 0;
        r.corner.y = 1;
        r.extent.width = 1;
        r.extent.height = ((ScreenHeight as i16) - 1) - r.corner.y;
        DrawFilledRectangle(&mut r);
        r.corner.x = (ScreenWidth - 64) as i16;
        r.corner.y = 0;
        r.extent.width = 1;
        r.extent.height = 139;
        DrawFilledRectangle(&mut r);
        r.corner.x = ((ScreenWidth - 64) as i16) + 1;
        r.corner.y = 139;
        r.extent.width = 64 - 2;
        r.extent.height = 1;
        DrawFilledRectangle(&mut r);
        r.corner.x = (ScreenWidth - 64) as i16;
        r.corner.y = 140;
        r.extent.width = 1;
        r.extent.height = (ScreenHeight as i16) - r.corner.y;
        DrawFilledRectangle(&mut r);
    }

    InitSISContexts();
    ClearSISRect(DRAW_SIS_DISPLAY as u8);

    UnbatchGraphics();
}
