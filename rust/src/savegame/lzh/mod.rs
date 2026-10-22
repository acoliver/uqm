//! Legacy LZSS/adaptive-Huffman streams from LZHUF.C (Okumura/Yoshizaki,
//! English version by Rikitake), as adapted by Reiche and Ford for UQM.
//! The 16-byte lookahead, match ordering and 16-bit bit buffer are format rules.

mod decode;
mod encode;
mod huffman;
#[cfg(test)]
mod tests;

use std::ffi::c_void;
use std::ptr;

use crate::resource::ffi_bridge::{GetResFileChar, PutResFileChar};

const N: usize = 4096;
const F: usize = 16;
const THRESHOLD: usize = 2;
const N_CHAR: usize = 256 - THRESHOLD + F;
const T: usize = N_CHAR * 2 - 1;
const R: usize = T - 1;

/// The prefix matches the former private `lzh.h` layout. Public callers use
/// an opaque DECODE_REF; the final field owns the encoder's dictionary tree.
#[repr(C)]
pub struct CodeDesc {
    buf_index: u16,
    restart_index: u16,
    bytes_left: u16,
    text_buf: [u8; N + F - 1],
    freq: [u16; T + 1],
    prnt: [u16; T + N_CHAR],
    son: [u16; T],
    workbuf: u16,
    workbuflen: u8,
    stream_type: u8,
    stream: *mut c_void,
    stream_index: u32,
    stream_length: u32,
    stream_mode: u8,
    cleanup: Option<unsafe extern "C" fn()>,
    encoder: Option<Box<encode::Encoder>>,
}

#[no_mangle]
pub static mut _lpCurCodeDesc: *mut CodeDesc = ptr::null_mut();
#[no_mangle]
pub static mut _StreamType: u8 = 0;
#[no_mangle]
pub static mut _Stream: *mut u8 = ptr::null_mut();
#[no_mangle]
pub static mut _workbuf: u16 = 0;
#[no_mangle]
pub static mut _workbuflen: u8 = 0;

impl CodeDesc {
    fn new(stream: *mut c_void, stream_type: u8, stream_mode: u8) -> Self {
        let mut result = Self {
            buf_index: (N - F) as u16,
            restart_index: 0,
            bytes_left: 0,
            text_buf: [0; N + F - 1],
            freq: [0; T + 1],
            prnt: [0; T + N_CHAR],
            son: [0; T],
            workbuf: 0,
            workbuflen: 0,
            stream_type,
            stream,
            stream_index: 0,
            stream_length: 0,
            stream_mode,
            cleanup: None,
            encoder: None,
        };
        result.text_buf[..N - F].fill(b' ');
        result.start_huff();
        result
    }

    unsafe fn input(&mut self) -> i32 {
        if self.stream_type == 0 {
            GetResFileChar(self.stream)
        } else {
            let result = *self.stream.cast::<u8>();
            self.stream = self.stream.cast::<u8>().add(1).cast();
            i32::from(result)
        }
    }

    unsafe fn output(&mut self, byte: u8) {
        if self.stream_type == 0 {
            PutResFileChar(byte as _, self.stream);
        } else {
            *self.stream.cast::<u8>() = byte;
            self.stream = self.stream.cast::<u8>().add(1).cast();
        }
    }

    unsafe fn publish(&self) {
        _StreamType = self.stream_type;
        _Stream = self.stream.cast();
        _workbuf = self.workbuf;
        _workbuflen = self.workbuflen;
    }
}

pub use decode::{cclose, cfilelength, copen, cread};
pub use encode::cwrite;
pub use huffman::{_update, StartHuff};
