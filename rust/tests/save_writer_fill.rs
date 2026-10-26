use std::error::Error;

use uqm_rust::savegame::{decode_save, encode_save, runtime::prepare_summary_for_write};

const SAVE: &[u8] = include_bytes!("fixtures/savegame/legacy-save.00");

#[test]
fn live_writer_repeats_first_element_amount_without_changing_other_fields(
) -> Result<(), Box<dyn Error>> {
    for first in [0, 1, 0x1234, u16::MAX] {
        let mut save = decode_save(SAVE)?;
        save.summary.sis.element_amounts = [first, 2, 3, 4, 5, 6, 7, 8];
        let mut expected = save.clone();
        expected.summary.sis.element_amounts = [first; 8];
        prepare_summary_for_write(&mut save.summary);
        assert_eq!(save, expected);
        assert_eq!(decode_save(&encode_save(&save)?)?, expected);
    }
    Ok(())
}

#[test]
fn general_codec_preserves_distinct_element_amounts() -> Result<(), Box<dyn Error>> {
    let mut save = decode_save(SAVE)?;
    save.summary.sis.element_amounts = [0, 1, 2, 3, 4, 5, 6, u16::MAX];
    assert_eq!(decode_save(&encode_save(&save)?)?, save);
    Ok(())
}
