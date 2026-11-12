//! Immutable definitions used by conversation content.

#[derive(Debug, Clone, Copy)]
pub struct DigitName {
    pub divider: i32,
    pub max_remainder: i32,
    pub phrase: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct SpeechDigit {
    pub divider: i32,
    pub subtrahend: i32,
    pub strings: Option<&'static [u16; 10]>,
    pub names: &'static [DigitName],
    pub common: u16,
}

impl SpeechDigit {
    pub const fn new(
        divider: i32,
        subtrahend: i32,
        strings: Option<&'static [u16; 10]>,
        common: u16,
    ) -> Self {
        Self {
            divider,
            subtrahend,
            strings,
            names: &[],
            common,
        }
    }
}

#[derive(Debug)]
pub struct NumberSpeech {
    pub digits: &'static [SpeechDigit],
}
