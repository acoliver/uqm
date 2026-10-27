//! Single storage authority for the legacy battle queues and display list.
#![allow(non_snake_case, non_upper_case_globals)]

use super::record::{PRIMITIVE, QUEUE};
use crate::ships::native::abi::{COUNT, MAX_DISPLAY_PRIMS, PRIM_LINKS};
use std::mem;

#[no_mangle]
// SAFETY: QUEUE contains only integers and nullable raw pointers.
pub static mut disp_q: QUEUE = unsafe { mem::zeroed() };
#[no_mangle]
// SAFETY: Both queues contain only integers and nullable raw pointers.
pub static mut race_q: [QUEUE; 2] = unsafe { mem::zeroed() };

#[no_mangle]
pub static mut DisplayFreeList: COUNT = 0;
#[no_mangle]
// SAFETY: Primitive fields and the PRIM_DESC union members admit all-zero bits.
pub static mut DisplayArray: [PRIMITIVE; MAX_DISPLAY_PRIMS as usize] = unsafe { mem::zeroed() };
#[no_mangle]
pub static mut DisplayLinks: PRIM_LINKS = 0;
