use super::*;
use crate::comm::races::descriptor::{NumberSpeech, SpeechDigit};

const DIGITS: [u16; 10] = [
    ENUMERATE_ZERO as u16,
    ENUMERATE_ONE as u16,
    ENUMERATE_TWO as u16,
    ENUMERATE_THREE as u16,
    ENUMERATE_FOUR as u16,
    ENUMERATE_FIVE as u16,
    ENUMERATE_SIX as u16,
    ENUMERATE_SEVEN as u16,
    ENUMERATE_EIGHT as u16,
    ENUMERATE_NINE as u16,
];
const TEENS: [u16; 10] = [
    ENUMERATE_TEN as u16,
    ENUMERATE_ELEVEN as u16,
    ENUMERATE_TWELVE as u16,
    ENUMERATE_THIRTEEN as u16,
    ENUMERATE_FOURTEEN as u16,
    ENUMERATE_FIFTEEN as u16,
    ENUMERATE_SIXTEEN as u16,
    ENUMERATE_SEVENTEEN as u16,
    ENUMERATE_EIGHTEEN as u16,
    ENUMERATE_NINETEEN as u16,
];
const TENS: [u16; 10] = [
    0,
    0,
    ENUMERATE_TWENTY as u16,
    ENUMERATE_THIRTY as u16,
    ENUMERATE_FOURTY as u16,
    ENUMERATE_FIFTY as u16,
    ENUMERATE_SIXTY as u16,
    ENUMERATE_SEVENTY as u16,
    ENUMERATE_EIGHTY as u16,
    ENUMERATE_NINETY as u16,
];

const NUMBER_SPEECH: NumberSpeech = NumberSpeech {
    digits: &[
        SpeechDigit::new(1000, 0, None, ENUMERATE_THOUSAND as u16),
        SpeechDigit::new(100, 0, Some(&DIGITS), ENUMERATE_HUNDRED as u16),
        SpeechDigit::new(10, 0, Some(&TENS), 0),
        SpeechDigit::new(1, 10, Some(&TEENS), 0),
        SpeechDigit::new(1, 0, Some(&DIGITS), 0),
    ],
};

pub(crate) fn descriptor() -> &'static NumberSpeech {
    &NUMBER_SPEECH
}
