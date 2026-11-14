//! Legacy LZSS/adaptive-Huffman streams from LZHUF.C (Okumura/Yoshizaki,
//! English version by Rikitake), as adapted by Reiche and Ford for UQM.
//! The 16-byte lookahead, match ordering and 16-bit bit buffer are format rules.

mod decode;
mod encode;
mod huffman;
#[cfg(test)]
mod tests;

use std::io::{self, Read, Seek, Write};

pub use decode::Decoder;
pub use encode::Encoder;

const N: usize = 4096;
const F: usize = 16;
const THRESHOLD: usize = 2;
const N_CHAR: usize = 256 - THRESHOLD + F;
const T: usize = N_CHAR * 2 - 1;
const R: usize = T - 1;

struct CodeDesc<S> {
    buf_index: u16,
    restart_index: u16,
    bytes_left: u16,
    text_buf: [u8; N + F - 1],
    freq: [u16; T + 1],
    prnt: [u16; T + N_CHAR],
    son: [u16; T],
    workbuf: u16,
    workbuflen: u8,
    stream: S,
    stream_index: u32,
    stream_length: u32,
    encoder: Option<Box<encode::Tree>>,
}

impl<S> CodeDesc<S> {
    fn new(stream: S) -> Self {
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
            stream,
            stream_index: 0,
            stream_length: 0,
            encoder: None,
        };
        result.text_buf[..N - F].fill(b' ');
        result.start_huff();
        result
    }
}

impl<S: Read> CodeDesc<S> {
    fn input(&mut self) -> io::Result<u8> {
        let mut byte = [0];
        // The original FILE_STREAM decoder zero-extends its bit-buffer lookahead at EOF.
        match self.stream.read_exact(&mut byte) {
            Ok(()) => Ok(byte[0]),
            Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => Ok(0),
            Err(error) => Err(error),
        }
    }
}

impl<S: Write + Seek> CodeDesc<S> {
    fn output(&mut self, byte: u8) -> io::Result<()> {
        self.stream.write_all(&[byte])
    }
}
