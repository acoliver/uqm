//! Cyclic checksum storage for the negotiated delay window.
#[derive(Debug, Default)]
pub struct ChecksumBuffer {
    checksums: Vec<u32>,
    interval: usize,
}
impl ChecksumBuffer {
    pub fn new(delay: usize, interval: usize) -> Self {
        assert!(interval > 0);
        Self {
            checksums: vec![0; (2 * delay + 2).div_ceil(interval)],
            interval,
        }
    }
    pub fn len(&self) -> usize {
        self.checksums.len()
    }
    pub fn is_empty(&self) -> bool {
        self.checksums.is_empty()
    }
    fn index(&self, frame: u32) -> usize {
        assert!((frame as usize).is_multiple_of(self.interval));
        (frame as usize / self.interval) % self.checksums.len()
    }
    pub fn add(&mut self, frame: u32, checksum: u32) {
        let index = self.index(frame);
        self.checksums[index] = checksum;
    }
    pub fn get(&self, frame: u32) -> u32 {
        self.checksums[self.index(frame)]
    }
}
