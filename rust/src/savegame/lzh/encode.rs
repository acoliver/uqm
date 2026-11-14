use super::{CodeDesc, F, N, R, T, THRESHOLD};
use std::io::{self, Seek, SeekFrom, Write};

pub(super) struct Tree {
    lson: [usize; N + 1],
    rson: [usize; N + 257],
    dad: [usize; N + 1],
    match_position: u16,
    match_length: usize,
}

impl Tree {
    fn new() -> Box<Self> {
        let mut result = Box::new(Self {
            lson: [0; N + 1],
            rson: [0; N + 257],
            dad: [0; N + 1],
            match_position: 0,
            match_length: 0,
        });
        result.rson[N + 1..=N + 256].fill(N);
        result.dad[..N].fill(N);
        result
    }

    fn insert(&mut self, r: usize, text: &[u8; N + F - 1]) {
        let mut cmp = 1;
        let mut p = N + 1 + text[r] as usize;
        self.rson[r] = N;
        self.lson[r] = N;
        self.match_length = 0;
        loop {
            if cmp >= 0 {
                if self.rson[p] != N {
                    p = self.rson[p];
                } else {
                    self.rson[p] = r;
                    self.dad[r] = p;
                    return;
                }
            } else if self.lson[p] != N {
                p = self.lson[p];
            } else {
                self.lson[p] = r;
                self.dad[r] = p;
                return;
            }
            let mut i = 1;
            while i < F {
                cmp = i32::from(text[r + i]) - i32::from(text[p + i]);
                if cmp != 0 {
                    break;
                }
                i += 1;
            }
            if i > THRESHOLD {
                let position = ((r.wrapping_sub(p) & (N - 1)) as u16).wrapping_sub(1);
                if i > self.match_length {
                    self.match_position = position;
                    self.match_length = i;
                    if i >= F {
                        break;
                    }
                } else if i == self.match_length && position < self.match_position {
                    self.match_position = position;
                }
            }
        }
        self.dad[r] = self.dad[p];
        self.lson[r] = self.lson[p];
        self.rson[r] = self.rson[p];
        self.dad[self.lson[p]] = r;
        self.dad[self.rson[p]] = r;
        if self.rson[self.dad[p]] == p {
            self.rson[self.dad[p]] = r;
        } else {
            self.lson[self.dad[p]] = r;
        }
        self.dad[p] = N;
    }

    fn delete(&mut self, p: usize) {
        if self.dad[p] == N {
            return;
        }
        let q = if self.rson[p] == N {
            self.lson[p]
        } else if self.lson[p] == N {
            self.rson[p]
        } else {
            let mut q = self.lson[p];
            if self.rson[q] != N {
                loop {
                    q = self.rson[q];
                    if self.rson[q] == N {
                        break;
                    }
                }
                self.rson[self.dad[q]] = self.lson[q];
                self.dad[self.lson[q]] = self.dad[q];
                self.lson[q] = self.lson[p];
                self.dad[self.lson[p]] = q;
            }
            self.rson[q] = self.rson[p];
            self.dad[self.rson[p]] = q;
            q
        };
        self.dad[q] = self.dad[p];
        if self.rson[self.dad[p]] == p {
            self.rson[self.dad[p]] = q;
        } else {
            self.lson[self.dad[p]] = q;
        }
        self.dad[p] = N;
    }
}

impl<S: Write + Seek> CodeDesc<S> {
    fn put_code(&mut self, len: u8, code: u16) -> io::Result<()> {
        self.workbuf |= code >> self.workbuflen;
        self.workbuflen += len;
        if self.workbuflen >= 8 {
            self.output((self.workbuf >> 8) as u8)?;
            self.stream_index += 1;
            self.workbuflen -= 8;
            if self.workbuflen >= 8 {
                self.output(self.workbuf as u8)?;
                self.stream_index += 1;
                self.workbuflen -= 8;
                self.workbuf = (u32::from(code) << (len - self.workbuflen)) as u16;
            } else {
                self.workbuf <<= 8;
            }
        }
        Ok(())
    }

    fn encode_char(&mut self, c: usize) -> io::Result<()> {
        let mut code = 0u16;
        let mut len = 0;
        let mut k = self.prnt[c + T] as usize;
        loop {
            code >>= 1;
            if k & 1 != 0 {
                code += 0x8000;
            }
            len += 1;
            k = self.prnt[k] as usize;
            if k == R {
                break;
            }
        }
        self.put_code(len, code)?;
        self.update(c + T);
        Ok(())
    }

    fn encode_position(&mut self, c: u16) -> io::Result<()> {
        let i = (c >> 6) as usize;
        self.put_code(P_LEN[i], u16::from(P_CODE[i]) << 8)?;
        self.put_code(6, (c & 0x3f) << 10)?;
        Ok(())
    }

    fn encode_match(&mut self, tree: &mut Tree, r: usize, limit: usize) -> io::Result<usize> {
        tree.match_length = tree.match_length.min(limit);
        if tree.match_length <= THRESHOLD {
            tree.match_length = 1;
            self.encode_char(self.text_buf[r] as usize)?;
        } else {
            self.encode_char(255 - THRESHOLD + tree.match_length)?;
            self.encode_position(tree.match_position)?;
        }
        Ok(tree.match_length)
    }

    fn write(&mut self, mut input: &[u8]) -> io::Result<()> {
        let mut r = self.buf_index as usize;
        let mut s = self.restart_index as usize;
        let mut left = self.bytes_left as usize;
        let old_length = self.stream_length as usize;
        self.stream_length = self.stream_length.wrapping_add(input.len() as u32);
        let mut tree = self.encoder.take().unwrap_or_else(Tree::new);
        if left == 0 && old_length < F {
            let mut i = old_length;
            while i < F && !input.is_empty() {
                self.text_buf[r + i] = input[0];
                input = &input[1..];
                i += 1;
            }
            if i < F {
                self.encoder = Some(tree);
                return Ok(());
            }
            for i in 1..=F {
                tree.insert(r - i, &self.text_buf);
            }
            tree.insert(r, &self.text_buf);
            if input.is_empty() {
                self.encoder = Some(tree);
                return Ok(());
            }
        }
        loop {
            if left == 0 {
                left = self.encode_match(&mut tree, r, F)?;
            }
            while left != 0 && !input.is_empty() {
                left -= 1;
                tree.delete(s);
                let c = input[0];
                input = &input[1..];
                self.text_buf[s] = c;
                if s < F - 1 {
                    self.text_buf[s + N] = c;
                }
                s = (s + 1) & (N - 1);
                r = (r + 1) & (N - 1);
                tree.insert(r, &self.text_buf);
            }
            if left != 0 {
                break;
            }
        }
        self.buf_index = r as u16;
        self.restart_index = s as u16;
        self.bytes_left = left as u16;
        self.encoder = Some(tree);
        Ok(())
    }

    fn finish_encode(&mut self) -> io::Result<()> {
        let mut tree = self.encoder.take().unwrap();
        let mut r = self.buf_index as usize;
        let mut s = self.restart_index as usize;
        let mut left = self.bytes_left as usize;
        let mut len = F;
        if self.stream_length < F as u32 {
            for i in 1..=F {
                tree.insert(r - i, &self.text_buf);
            }
            tree.insert(r, &self.text_buf);
            len = self.stream_length as usize;
        }
        loop {
            while left != 0 {
                left -= 1;
                tree.delete(s);
                len -= 1;
                if len == 0 {
                    if self.workbuflen != 0 {
                        self.output((self.workbuf >> 8) as u8)?;
                        self.stream_index += 1;
                    }
                    self.stream_index += 4;
                    self.stream
                        .seek(SeekFrom::Current(-i64::from(self.stream_index)))?;
                    for byte in self.stream_length.to_le_bytes() {
                        self.output(byte)?;
                    }
                    return Ok(());
                }
                s = (s + 1) & (N - 1);
                r = (r + 1) & (N - 1);
                tree.insert(r, &self.text_buf);
            }
            left = self.encode_match(&mut tree, r, len)?;
        }
    }
}

/// Streaming encoder. Finishing rewrites the length header at its original offset.
pub struct Encoder<W> {
    desc: CodeDesc<W>,
}

impl<W: Write + Seek> Encoder<W> {
    pub fn new(mut stream: W) -> io::Result<Self> {
        stream.write_all(&[0; 4])?;
        Ok(Self {
            desc: CodeDesc::new(stream),
        })
    }

    pub fn finish(mut self) -> io::Result<W> {
        if self.desc.encoder.is_some() {
            self.desc.finish_encode()?;
        }
        self.desc.stream.flush()?;
        Ok(self.desc.stream)
    }
}

impl<W: Write + Seek> Write for Encoder<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if !bytes.is_empty() {
            self.desc.write(bytes)?;
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        self.desc.stream.flush()
    }
}

const P_CODE: [u8; 64] = [
    0x00, 0x20, 0x30, 0x40, 0x50, 0x58, 0x60, 0x68, 0x70, 0x78, 0x80, 0x88, 0x90, 0x94, 0x98, 0x9C,
    0xA0, 0xA4, 0xA8, 0xAC, 0xB0, 0xB4, 0xB8, 0xBC, 0xC0, 0xC2, 0xC4, 0xC6, 0xC8, 0xCA, 0xCC, 0xCE,
    0xD0, 0xD2, 0xD4, 0xD6, 0xD8, 0xDA, 0xDC, 0xDE, 0xE0, 0xE2, 0xE4, 0xE6, 0xE8, 0xEA, 0xEC, 0xEE,
    0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9, 0xFA, 0xFB, 0xFC, 0xFD, 0xFE, 0xFF,
];

const P_LEN: [u8; 64] = [
    0x03, 0x04, 0x04, 0x04, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x05, 0x06, 0x06, 0x06, 0x06,
    0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07, 0x07,
    0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08, 0x08,
];
