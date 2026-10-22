use super::*;
use crate::c_bindings::global_state::{NUMBER_SPEECH_DESC, SPEECH_DIGIT};

static DIGITS: [u16; 10] = [
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
static TEENS: [u16; 10] = [
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
static TENS: [u16; 10] = [
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

const fn digit(divider: i32, subtrahend: i32, strings: *const u16, common: u16) -> SPEECH_DIGIT {
    SPEECH_DIGIT {
        Divider: divider,
        Subtrahend: subtrahend,
        StrDigits: strings.cast_mut(),
        Names: std::ptr::null_mut(),
        CommonNameIndex: common,
    }
}
// NPCNumber only reads this descriptor and the phrase tables it borrows.
static mut NUMBERS: NUMBER_SPEECH_DESC = NUMBER_SPEECH_DESC {
    NumDigits: 5,
    Digits: [
        digit(1000, 0, std::ptr::null(), ENUMERATE_THOUSAND as u16),
        digit(100, 0, DIGITS.as_ptr(), ENUMERATE_HUNDRED as u16),
        digit(10, 0, TENS.as_ptr(), 0),
        digit(1, 10, TEENS.as_ptr(), 0),
        digit(1, 0, DIGITS.as_ptr(), 0),
        digit(0, 0, std::ptr::null(), 0),
        digit(0, 0, std::ptr::null(), 0),
    ],
};
pub(super) fn descriptor() -> *const NUMBER_SPEECH_DESC {
    std::ptr::addr_of!(NUMBERS)
}
