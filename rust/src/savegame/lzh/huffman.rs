use super::{CodeDesc, _LP_CUR_CODE_DESC, N_CHAR, R, T};

impl CodeDesc {
    pub(super) fn start_huff(&mut self) {
        for i in 0..N_CHAR {
            self.freq[i] = 1;
            self.son[i] = (i + T) as u16;
            self.prnt[i + T] = i as u16;
        }
        let mut i = 0;
        for j in N_CHAR..=R {
            self.freq[j] = self.freq[i] + self.freq[i + 1];
            self.son[j] = i as u16;
            self.prnt[i] = j as u16;
            self.prnt[i + 1] = j as u16;
            i += 2;
        }
        self.freq[T] = 0xffff;
        self.prnt[R] = 0;
    }

    fn reconstruct(&mut self) {
        let mut j = 0;
        for i in 0..T {
            if self.son[i] as usize >= T {
                self.freq[j] = (self.freq[i] + 1) >> 1;
                self.son[j] = self.son[i];
                j += 1;
            }
        }
        for (i, j) in (N_CHAR..T).enumerate() {
            let i = i * 2;
            let f = self.freq[i] + self.freq[i + 1];
            self.freq[j] = f;
            let mut k = j;
            while k > 0 && f < self.freq[k - 1] {
                k -= 1;
            }
            self.freq.copy_within(k..j, k + 1);
            self.freq[k] = f;
            self.son.copy_within(k..j, k + 1);
            self.son[k] = i as u16;
        }
        for i in 0..T {
            let j = self.son[i] as usize;
            self.prnt[j] = i as u16;
            if j < T {
                self.prnt[j + 1] = i as u16;
            }
        }
    }

    pub(super) fn update(&mut self, leaf: usize) {
        if self.freq[R] == 0x8000 {
            self.reconstruct();
        }
        let mut c = self.prnt[leaf] as usize;
        loop {
            self.freq[c] += 1;
            let mut i = self.freq[c] as usize;
            let mut l = c + 1;
            if i > self.freq[l] as usize {
                l += 1;
                while i > self.freq[l] as usize {
                    l += 1;
                }
                l -= 1;
                self.freq[c] = self.freq[l];
                self.freq[l] = i as u16;
                i = self.son[c] as usize;
                let j = self.son[l] as usize;
                self.son[l] = i as u16;
                self.son[c] = j as u16;
                self.prnt[i] = l as u16;
                if i < T {
                    self.prnt[i + 1] = l as u16;
                }
                self.prnt[j] = c as u16;
                if j < T {
                    self.prnt[j + 1] = c as u16;
                }
                c = l;
            }
            c = self.prnt[c] as usize;
            if c == 0 {
                break;
            }
        }
    }
}

/// # Safety
/// The current descriptor must have been selected by the stream API.
pub unsafe extern "C" fn start_huff() {
    (*_LP_CUR_CODE_DESC).start_huff();
}

/// # Safety
/// The current descriptor must be live and `c` must be a Huffman leaf index.
pub unsafe extern "C" fn _update(c: u16) {
    (*_LP_CUR_CODE_DESC).update(c as usize);
}
