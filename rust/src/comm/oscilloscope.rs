//! Oscilloscope display for audio waveform visualization
//!
//! Shows the audio waveform during alien speech playback.

/// Oscilloscope display width in pixels
pub const OSCILLOSCOPE_WIDTH: usize = 128;

/// Oscilloscope display height in pixels
pub const OSCILLOSCOPE_HEIGHT: usize = 64;

/// Oscilloscope display for audio waveform
#[derive(Debug)]
pub struct Oscilloscope {
    /// Waveform sample buffer
    samples: Vec<i16>,
    /// Write position in sample buffer
    write_pos: usize,
    /// Display buffer (scaled values 0-255)
    display: [u8; OSCILLOSCOPE_WIDTH],
    /// Whether the display is active
    active: bool,
    /// Peak value for normalization
    peak: i16,
    /// Decay rate for peak (for smoother display)
    peak_decay: f32,
}

impl Default for Oscilloscope {
    fn default() -> Self {
        Self::new()
    }
}

impl Oscilloscope {
    /// Create a new oscilloscope
    pub fn new() -> Self {
        Self {
            samples: vec![0; OSCILLOSCOPE_WIDTH * 4], // 4x oversampling
            write_pos: 0,
            display: [128; OSCILLOSCOPE_WIDTH], // Center line
            active: false,
            peak: 1, // Avoid divide by zero
            peak_decay: 0.995,
        }
    }

    /// Activate the oscilloscope
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate the oscilloscope
    pub fn deactivate(&mut self) {
        self.active = false;
        self.clear();
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Clear the display
    pub fn clear(&mut self) {
        self.samples.fill(0);
        self.display.fill(128);
        self.write_pos = 0;
        self.peak = 1;
    }

    /// Add audio samples to the oscilloscope
    pub fn add_samples(&mut self, samples: &[i16]) {
        if !self.active || samples.is_empty() {
            return;
        }

        for &sample in samples {
            // Track peak for normalization
            let abs_sample = sample.saturating_abs();
            if abs_sample > self.peak {
                self.peak = abs_sample;
            }

            // Store sample
            self.samples[self.write_pos] = sample;
            self.write_pos = (self.write_pos + 1) % self.samples.len();
        }
    }

    /// Update the display buffer
    pub fn update(&mut self) {
        if !self.active {
            return;
        }

        // Decay peak over time for smoother display
        self.peak = ((self.peak as f32 * self.peak_decay) as i16).max(1);

        // Calculate display values
        let samples_per_pixel = self.samples.len() / OSCILLOSCOPE_WIDTH;
        let peak_f = self.peak as f32;

        for (i, display_val) in self.display.iter_mut().enumerate() {
            let start = i * samples_per_pixel;
            let end = start + samples_per_pixel;

            // Average samples for this pixel
            let sum: i32 = self.samples[start..end].iter().map(|&s| s as i32).sum();
            let avg = sum / samples_per_pixel as i32;

            // Normalize to 0-255 range (128 = center)
            let normalized = (avg as f32 / peak_f * 64.0) + 128.0;
            *display_val = normalized.clamp(0.0, 255.0) as u8;
        }
    }

    /// Get the display buffer
    pub fn display(&self) -> &[u8; OSCILLOSCOPE_WIDTH] {
        &self.display
    }

    /// Get a scaled Y coordinate for a display column
    pub fn get_y(&self, x: usize) -> u8 {
        if x < OSCILLOSCOPE_WIDTH {
            self.display[x]
        } else {
            128
        }
    }

    /// Get the current peak value
    pub fn peak(&self) -> i16 {
        self.peak
    }

    /// Set peak decay rate (0.0 = instant, 1.0 = no decay)
    pub fn set_peak_decay(&mut self, decay: f32) {
        self.peak_decay = decay.clamp(0.0, 1.0);
    }

    /// Calculate waveform for a specific range of samples
    pub fn calculate_waveform(&self, width: usize, height: usize) -> Vec<u8> {
        let mut result = vec![height as u8 / 2; width];

        if self.samples.is_empty() || !self.active {
            return result;
        }

        let samples_per_pixel = self.samples.len().max(1) / width.max(1);
        let half_height = height as f32 / 2.0;
        let peak_f = self.peak as f32;

        for (i, y) in result.iter_mut().enumerate() {
            let start = i * samples_per_pixel;
            let end = (start + samples_per_pixel).min(self.samples.len());

            if start < self.samples.len() {
                // Find max absolute value in this range
                let max_abs = self.samples[start..end]
                    .iter()
                    .map(|&s| s.saturating_abs())
                    .max()
                    .unwrap_or(0);

                let normalized = (max_abs as f32 / peak_f * half_height) + half_height;
                *y = normalized.clamp(0.0, (height - 1) as f32) as u8;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oscilloscope_new() {
        let osc = Oscilloscope::new();
        assert!(!osc.is_active());
        assert_eq!(osc.peak(), 1);
    }

    #[test]
    fn test_activate_deactivate() {
        let mut osc = Oscilloscope::new();

        osc.activate();
        assert!(osc.is_active());

        osc.deactivate();
        assert!(!osc.is_active());
    }

    #[test]
    fn test_add_samples_inactive() {
        let mut osc = Oscilloscope::new();
        osc.add_samples(&[100, 200, 300]);

        // Should not affect peak when inactive
        assert_eq!(osc.peak(), 1);
    }

    #[test]
    fn test_add_samples_active() {
        let mut osc = Oscilloscope::new();
        osc.activate();
        osc.add_samples(&[100, -200, 300]);

        assert_eq!(osc.peak(), 300);
    }

    #[test]
    fn test_update_display() {
        let mut osc = Oscilloscope::new();
        osc.activate();

        // Add some samples
        let samples: Vec<i16> = (0..512).map(|i| (i * 100) as i16).collect();
        osc.add_samples(&samples);

        osc.update();

        // Display should have been updated
        let display = osc.display();
        assert_eq!(display.len(), OSCILLOSCOPE_WIDTH);
    }

    #[test]
    fn test_get_y() {
        let mut osc = Oscilloscope::new();

        // Default should be center
        assert_eq!(osc.get_y(0), 128);
        assert_eq!(osc.get_y(64), 128);

        // Out of range should return center
        assert_eq!(osc.get_y(1000), 128);
    }

    #[test]
    fn test_clear() {
        let mut osc = Oscilloscope::new();
        osc.activate();
        osc.add_samples(&[1000, 2000, 3000]);

        osc.clear();

        assert_eq!(osc.peak(), 1);
        // All display values should be center
        for &val in osc.display() {
            assert_eq!(val, 128);
        }
    }

    #[test]
    fn test_peak_decay() {
        let mut osc = Oscilloscope::new();
        osc.activate();
        osc.add_samples(&[10000]);

        let initial_peak = osc.peak();

        // Update should decay peak
        osc.update();
        assert!(osc.peak() < initial_peak);
    }

    #[test]
    fn test_set_peak_decay() {
        let mut osc = Oscilloscope::new();

        osc.set_peak_decay(0.5);
        assert!((osc.peak_decay - 0.5).abs() < 0.001);

        // Should clamp to 0-1
        osc.set_peak_decay(1.5);
        assert!((osc.peak_decay - 1.0).abs() < 0.001);

        osc.set_peak_decay(-0.5);
        assert!(osc.peak_decay >= 0.0);
    }

    #[test]
    fn test_calculate_waveform() {
        let mut osc = Oscilloscope::new();
        osc.activate();

        let samples: Vec<i16> = (0..512).map(|i| ((i % 100) * 100) as i16).collect();
        osc.add_samples(&samples);

        let waveform = osc.calculate_waveform(64, 32);
        assert_eq!(waveform.len(), 64);

        // All values should be within height range
        for &y in &waveform {
            assert!(y < 32);
        }
    }

    #[test]
    fn test_calculate_waveform_inactive() {
        let osc = Oscilloscope::new();
        let waveform = osc.calculate_waveform(64, 32);

        // Should return center values
        for &y in &waveform {
            assert_eq!(y, 16);
        }
    }

    #[test]
    fn test_negative_samples() {
        let mut osc = Oscilloscope::new();
        osc.activate();
        osc.add_samples(&[-5000, -10000, -15000]);

        assert_eq!(osc.peak(), 15000);
    }
}
