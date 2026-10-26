use std::error::Error;

use uqm_save::{EncodeError, SaveFormatError, SaveNameError, SlotError};

fn diagnostic(error: impl Error, expected: &str) {
    assert_eq!(error.to_string(), expected);
}

#[test]
fn codec_and_slot_diagnostics_retain_their_error_context() {
    diagnostic(
        SaveFormatError::UnexpectedTag {
            expected: 1,
            found: 2,
        },
        "UnexpectedTag { expected: 1, found: 2 }",
    );
    diagnostic(
        EncodeError::CountOverflow {
            tag: 123,
            count: 65536,
        },
        "CountOverflow { tag: 123, count: 65536 }",
    );
    diagnostic(SaveNameError::TooLong { len: 64 }, "TooLong { len: 64 }");
    diagnostic(
        SlotError::Format {
            slot: 7,
            error: SaveFormatError::BadMagic { found: 0 },
        },
        "Format { slot: 7, error: BadMagic { found: 0 } }",
    );
}
