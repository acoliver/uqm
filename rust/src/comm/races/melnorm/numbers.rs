use super::phrases::*;
use crate::comm::races::descriptor::{NumberSpeech, SpeechDigit};

static DIGITS: [u16; 10] = [
    ENUMERATE_ZERO,
    ENUMERATE_ONE,
    ENUMERATE_TWO,
    ENUMERATE_THREE,
    ENUMERATE_FOUR,
    ENUMERATE_FIVE,
    ENUMERATE_SIX,
    ENUMERATE_SEVEN,
    ENUMERATE_EIGHT,
    ENUMERATE_NINE,
];
static TEENS: [u16; 10] = [
    ENUMERATE_TEN,
    ENUMERATE_ELEVEN,
    ENUMERATE_TWELVE,
    ENUMERATE_THIRTEEN,
    ENUMERATE_FOURTEEN,
    ENUMERATE_FIFTEEN,
    ENUMERATE_SIXTEEN,
    ENUMERATE_SEVENTEEN,
    ENUMERATE_EIGHTEEN,
    ENUMERATE_NINETEEN,
];
static TENS: [u16; 10] = [
    0,
    0,
    ENUMERATE_TWENTY,
    ENUMERATE_THIRTY,
    ENUMERATE_FOURTY,
    ENUMERATE_FIFTY,
    ENUMERATE_SIXTY,
    ENUMERATE_SEVENTY,
    ENUMERATE_EIGHTY,
    ENUMERATE_NINETY,
];
const NUMBER_SPEECH: NumberSpeech = NumberSpeech {
    digits: &[
        SpeechDigit::new(1000, 0, None, ENUMERATE_THOUSAND),
        SpeechDigit::new(100, 0, Some(&DIGITS), ENUMERATE_HUNDRED),
        SpeechDigit::new(10, 0, Some(&TENS), 0),
        SpeechDigit::new(1, 10, Some(&TEENS), 0),
        SpeechDigit::new(1, 0, Some(&DIGITS), 0),
    ],
};

pub(crate) fn descriptor() -> &'static NumberSpeech {
    &NUMBER_SPEECH
}
