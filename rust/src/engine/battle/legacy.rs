//! Single storage authority for the legacy battle queues and display list.

use super::record::{MAX_DISPLAY_PRIMS, PRIMITIVE, QUEUE};
use crate::graphics::native_abi::{COUNT, PRIM_LINKS};
use std::{mem, ptr};

const EMPTY_QUEUE: QUEUE = QUEUE {
    head: ptr::null_mut(),
    tail: ptr::null_mut(),
    pq_tab: ptr::null_mut(),
    free_list: ptr::null_mut(),
    object_size: 0,
    num_objects: 0,
};

#[no_mangle]
pub static mut disp_q: QUEUE = EMPTY_QUEUE;
#[no_mangle]
pub static mut race_q: [QUEUE; 2] = [EMPTY_QUEUE; 2];

#[no_mangle]
pub static mut DisplayFreeList: COUNT = 0;
#[no_mangle]
// SAFETY: Primitive fields and the PRIM_DESC union members admit all-zero bits.
pub static mut DisplayArray: [PRIMITIVE; MAX_DISPLAY_PRIMS as usize] = unsafe { mem::zeroed() };
#[no_mangle]
pub static mut DisplayLinks: PRIM_LINKS = 0;
