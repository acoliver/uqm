//! Flagship capacities and physical crew, mineral and fuel positions.

use super::*;

#[cfg(not(test))]
fn crew_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 10] = [
        (10, 30, 9),
        (0, 30, 0),
        (0, 27, 0),
        (0, 24, 0),
        (0, 21, 0),
        (0, 18, 0),
        (0, 16, 0),
        (0, 13, 0),
        (0, 10, 0),
        (0, 7, 0),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}
fn storage_color(i: usize) -> Color {
    const V: [u8; 10] = [31, 28, 24, 21, 18, 16, 13, 10, 8, 5];
    rgb15(V[i], V[i], V[i])
}
fn fuel_color(i: usize) -> Color {
    const RGB: [(u8, u8, u8); 11] = [
        (15, 0, 0),
        (19, 0, 0),
        (23, 0, 0),
        (27, 0, 0),
        (31, 3, 0),
        (31, 7, 0),
        (31, 10, 0),
        (31, 14, 0),
        (31, 17, 0),
        (31, 21, 0),
        (31, 24, 0),
    ];
    let (r, g, b) = RGB[i];
    rgb15(r, g, b)
}

#[no_mangle]
pub extern "C" fn GetCrewCount() -> COUNT {
    sis().read(|owner| owner.crew_enlisted)
}
#[no_mangle]
pub extern "C" fn GetModuleCrewCapacity(module_type: BYTE) -> COUNT {
    crate::ui::sis_capacity_math::module_crew_capacity(module_type)
}
#[no_mangle]
pub extern "C" fn GetCrewPodCapacity() -> COUNT {
    sis().read(|state| {
        state
            .module_slots
            .iter()
            .map(|&m| GetModuleCrewCapacity(m))
            .sum()
    })
}
/// # Safety
/// ppt must be writable; SIS state and graphics context must be initialized.
#[cfg(not(test))]
pub unsafe fn get_c_pod_capacity(pc_fonts: bool, ppt: *mut POINT) -> COUNT {
    let crew = GetCrewCount();
    let modules = sis().read(|state| state.module_slots);
    let (slot, seat) = match crate::ui::sis_capacity_math::crew_pod_for(&modules, crew) {
        Some(v) => v,
        None => return GetCrewPodCapacity(),
    };
    let row = i32::from(seat) / CREW_PER_ROW as i32;
    let col = i32::from(seat) % CREW_PER_ROW as i32;
    if pc_fonts {
        SetContextForeGroundColor(crew_color(row as usize));
    } else {
        SetContextForeGroundColor(rgb15(5, 0x10, 5));
    }
    (*ppt).x = (27 + (i32::from(slot) * SHIP_PIECE_OFFSET as i32) - (col * 2)) as SIZE;
    (*ppt).y = (34 - (row * 2)) as SIZE;
    GetCrewPodCapacity()
}
#[inline]
fn element_mass() -> COUNT {
    sis().read(|owner| owner.total_element_mass)
}
#[no_mangle]
pub extern "C" fn GetModuleStorageCapacity(module_type: BYTE) -> COUNT {
    crate::ui::sis_capacity_math::module_storage_capacity(module_type)
}
#[no_mangle]
pub extern "C" fn GetStorageBayCapacity() -> COUNT {
    sis().read(|state| {
        state
            .module_slots
            .iter()
            .map(|&m| GetModuleStorageCapacity(m))
            .sum()
    })
}
/// # Safety
/// ppt must be writable; SIS state and graphics context must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetSBayCapacity(ppt: *mut POINT) -> COUNT {
    let modules = sis().read(|state| state.module_slots);
    let (slot, cell) =
        match crate::ui::sis_capacity_math::storage_cell_for(&modules, element_mass()) {
            Some(v) => v,
            None => return GetStorageBayCapacity(),
        };
    let mut row = i32::from(cell) / SBAY_MASS_PER_ROW as i32;
    if row == 0 {
        SetContextForeGroundColor(rgb15(0, 0, 0));
    } else {
        row -= 1;
        SetContextForeGroundColor(storage_color(row as usize));
    }
    (*ppt).x = (19 + i32::from(slot) * SHIP_PIECE_OFFSET as i32) as SIZE;
    (*ppt).y = (34 - row * 2) as SIZE;
    GetStorageBayCapacity()
}
#[no_mangle]
pub extern "C" fn GetModuleFuelCapacity(module_type: BYTE) -> DWORD {
    use crate::starbase::sis_modules::ModuleType;
    ModuleType::from_u8(module_type).map_or(0, ModuleType::fuel_capacity)
}
#[no_mangle]
pub extern "C" fn GetFuelTankCapacity() -> DWORD {
    sis().read(|state| {
        state
            .module_slots
            .iter()
            .fold(FUEL_RESERVE, |cap, &m| cap + GetModuleFuelCapacity(m))
    })
}
fn fuel_slot_for(amount: DWORD) -> Option<(COUNT, DWORD)> {
    assert!(amount >= FUEL_RESERVE);
    let mut cap = FUEL_RESERVE;
    for slot in (0..NUM_MODULE_SLOTS as COUNT).rev() {
        cap += GetModuleFuelCapacity(sis().read(|owner| owner.module_slots[slot as usize]));
        if amount < cap {
            return Some((slot, cap - amount));
        }
    }
    None
}
/// # Safety
/// ppt must be writable; SIS state and graphics context must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetFTankCapacity(ppt: *mut POINT) -> DWORD {
    let capacity = GetFuelTankCapacity();
    let amount = sis().read(|owner| owner.fuel_on_board);
    if amount < FUEL_RESERVE {
        return capacity;
    }
    let (slot, compartment) = match fuel_slot_for(amount) {
        Some(v) => v,
        None => return capacity,
    };
    let module = sis().read(|owner| owner.module_slots[slot as usize]);
    let volume = GetModuleFuelCapacity(module);
    let row = ((volume - compartment) * MAX_FUEL_BARS / HEFUEL_TANK_CAPACITY) as COUNT;
    (*ppt).x = (21 + i32::from(slot) * SHIP_PIECE_OFFSET as i32) as SIZE;
    (*ppt).y = if volume == FUEL_TANK_CAPACITY {
        (27 - i32::from(row)) as SIZE
    } else {
        (30 - i32::from(row)) as SIZE
    };
    assert!(i32::from(row) + 1 < 11);
    SetContextForeGroundColor(fuel_color(row as usize));
    SetContextBackGroundColor(fuel_color(row as usize + 1));
    capacity
}

#[no_mangle]
pub extern "C" fn CountSISPieces(piece_type: BYTE) -> COUNT {
    sis().read(|state| {
        let slots: &[BYTE] = if u32::from(piece_type) == FUSION_THRUSTER {
            &state.drive_slots
        } else if u32::from(piece_type) == TURNING_JETS {
            &state.jet_slots
        } else {
            &state.module_slots
        };
        slots.iter().filter(|&&v| v == piece_type).count() as COUNT
    })
}
