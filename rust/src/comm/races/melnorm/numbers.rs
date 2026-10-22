use super::phrases::*;
use crate::c_bindings::{assert_c_layout, global_state};
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Digit {
    divider: i32,
    subtrahend: i32,
    strings: *const u16,
    names: *const c_void,
    common: u16,
}
#[repr(C)]
struct NumberSpeech {
    count: u16,
    digits: [Digit; 7],
}
assert_c_layout!(
    Digit,
    global_state::SPEECH_DIGIT {
        divider: Divider,
        subtrahend: Subtrahend,
        strings: StrDigits,
        names: Names,
        common: CommonNameIndex
    }
);
assert_c_layout!(
    NumberSpeech,
    global_state::NUMBER_SPEECH_DESC {
        count: NumDigits,
        digits: Digits
    }
);
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
const fn digit(divider: i32, subtrahend: i32, strings: *const u16, common: u16) -> Digit {
    Digit {
        divider,
        subtrahend,
        strings,
        names: std::ptr::null(),
        common,
    }
}
// The engine borrows this immutable descriptor on the game thread.
static mut SPEECH: NumberSpeech = NumberSpeech {
    count: 5,
    digits: [
        digit(1000, 0, std::ptr::null(), ENUMERATE_THOUSAND),
        digit(100, 0, DIGITS.as_ptr(), ENUMERATE_HUNDRED),
        digit(10, 0, TENS.as_ptr(), 0),
        digit(1, 10, TEENS.as_ptr(), 0),
        digit(1, 0, DIGITS.as_ptr(), 0),
        digit(0, 0, std::ptr::null(), 0),
        digit(0, 0, std::ptr::null(), 0),
    ],
};
pub(super) fn descriptor() -> *const c_void {
    std::ptr::addr_of!(SPEECH).cast()
}
