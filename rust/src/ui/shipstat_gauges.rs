use super::*;

pub(crate) unsafe fn draw_gauge_frames(siptr: &SHIP_INFO, y: COORD) {
    let crew = (((std::cmp::min(siptr.max_crew, MAX_CREW_SIZE) + 1) & !1) + 1) as SIZE;
    let energy = ((((siptr.max_energy + 1) >> 1) << 1) + 1) as COORD;
    let bottom = GAUGE_YOFFS + y;
    let draw = |x, y, width, height| {
        let mut rect = RECT {
            corner: POINT { x, y },
            extent: EXTENT { width, height },
        };
        DrawFilledRectangle(&mut rect);
    };
    SetContextForeGroundColor(rgb15(0x08, 0x08, 0x08));
    draw(CREW_XOFFS - 1, bottom + 1, STAT_WIDTH + 2, 1);
    draw(ENERGY_XOFFS - 1, bottom + 1, STAT_WIDTH + 2, 1);
    draw(ENERGY_XOFFS + STAT_WIDTH, bottom + 1 - energy, 1, energy);
    draw(CREW_XOFFS + STAT_WIDTH, bottom + 1 - crew, 1, crew);
    SetContextForeGroundColor(rgb15(0x10, 0x10, 0x10));
    draw(CREW_XOFFS - 1, bottom - crew, STAT_WIDTH + 2, 1);
    draw(ENERGY_XOFFS - 1, bottom - energy, STAT_WIDTH + 2, 1);
    draw(ENERGY_XOFFS - 1, bottom - energy, 1, energy + 1);
    draw(CREW_XOFFS - 1, bottom - crew, 1, crew + 1);
    SetContextForeGroundColor(rgb15(0, 0, 0));
    draw(CREW_XOFFS, bottom + 1 - crew, STAT_WIDTH, crew);
    draw(ENERGY_XOFFS, bottom + 1 - energy, STAT_WIDTH, energy);
}
