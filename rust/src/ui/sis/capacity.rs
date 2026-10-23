//! Flagship capacities and physical crew, mineral and fuel positions.

use super::*;

/// # Safety
/// Game state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetCrewCount() -> COUNT {
    (*sis()).CrewEnlisted
}
/// # Safety
/// None; moduleType is an SIS module identifier.
#[no_mangle]
pub unsafe extern "C" fn GetModuleCrewCapacity(module_type: BYTE) -> COUNT {
    crate::ui::sis_capacity_math::module_crew_capacity(module_type)
}
/// # Safety
/// SIS state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetCrewPodCapacity() -> COUNT {
    (*sis())
        .ModuleSlots
        .iter()
        .map(|&m| GetModuleCrewCapacity(m))
        .sum()
}
/// # Safety
/// ppt must be writable; SIS state and graphics context must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetCPodCapacity(ppt: *mut POINT) -> COUNT {
    let crew = GetCrewCount();
    let (slot, seat) = match crate::ui::sis_capacity_math::crew_pod_for(&(*sis()).ModuleSlots, crew)
    {
        Some(v) => v,
        None => return GetCrewPodCapacity(),
    };
    let row = i32::from(seat) / CREW_PER_ROW as i32;
    let col = i32::from(seat) % CREW_PER_ROW as i32;
    if optWhichFonts == OPT_PC {
        SetContextForeGroundColor(crew_color(row as usize));
    } else {
        SetContextForeGroundColor(rgb15(5, 0x10, 5));
    }
    (*ppt).x = (27 + (i32::from(slot) * SHIP_PIECE_OFFSET as i32) - (col * 2)) as SIZE;
    (*ppt).y = (34 - (row * 2)) as SIZE;
    GetCrewPodCapacity()
}
#[inline]
unsafe fn element_mass() -> COUNT {
    (*sis()).TotalElementMass
}
/// # Safety
/// None; moduleType is an SIS module identifier.
#[no_mangle]
pub unsafe extern "C" fn GetModuleStorageCapacity(module_type: BYTE) -> COUNT {
    crate::ui::sis_capacity_math::module_storage_capacity(module_type)
}
/// # Safety
/// SIS state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetStorageBayCapacity() -> COUNT {
    (*sis())
        .ModuleSlots
        .iter()
        .map(|&m| GetModuleStorageCapacity(m))
        .sum()
}
/// # Safety
/// ppt must be writable; SIS state and graphics context must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetSBayCapacity(ppt: *mut POINT) -> COUNT {
    let (slot, cell) =
        match crate::ui::sis_capacity_math::storage_cell_for(&(*sis()).ModuleSlots, element_mass())
        {
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
/// # Safety
/// None; moduleType is an SIS module identifier.
#[no_mangle]
pub unsafe extern "C" fn GetModuleFuelCapacity(module_type: BYTE) -> DWORD {
    use crate::starbase::sis_modules::ModuleType;
    ModuleType::from_u8(module_type).map_or(0, ModuleType::fuel_capacity)
}
/// # Safety
/// SIS state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn GetFuelTankCapacity() -> DWORD {
    (*sis())
        .ModuleSlots
        .iter()
        .fold(FUEL_RESERVE, |cap, &m| cap + GetModuleFuelCapacity(m))
}
unsafe fn fuel_slot_for(amount: DWORD) -> Option<(COUNT, DWORD)> {
    assert!(amount >= FUEL_RESERVE);
    let mut cap = FUEL_RESERVE;
    for slot in (0..NUM_MODULE_SLOTS as COUNT).rev() {
        cap += GetModuleFuelCapacity((*sis()).ModuleSlots[slot as usize]);
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
    let amount = (*sis()).FuelOnBoard;
    if amount < FUEL_RESERVE {
        return capacity;
    }
    let (slot, compartment) = match fuel_slot_for(amount) {
        Some(v) => v,
        None => return capacity,
    };
    let module = (*sis()).ModuleSlots[slot as usize];
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

/// # Safety
/// SIS state must be initialized.
#[no_mangle]
pub unsafe extern "C" fn CountSISPieces(piece_type: BYTE) -> COUNT {
    let s = sis();
    let slots: &[BYTE] = if u32::from(piece_type) == FUSION_THRUSTER {
        &(*s).DriveSlots
    } else if u32::from(piece_type) == TURNING_JETS {
        &(*s).JetSlots
    } else {
        &(*s).ModuleSlots
    };
    slots.iter().filter(|&&v| v == piece_type).count() as COUNT
}
