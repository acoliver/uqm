use std::error::Error;
use std::fs;
use std::path::Path;

use uqm_rust::savegame::{decode_save, encode_save};

fn roundtrip(path: &Path) -> Result<(), Box<dyn Error>> {
    let original = fs::read(path)?;
    let decoded = decode_save(&original)?;
    let encoded = encode_save(&decoded)?;
    assert_eq!(encoded.len(), original.len(), "{}: length", path.display());
    assert!(
        encoded == original,
        "{}: save bytes changed",
        path.display()
    );
    assert_eq!(decode_save(&encoded)?, decoded, "{}", path.display());
    Ok(())
}

#[test]
fn all_committed_saves_roundtrip_byte_identically() -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut saves = fs::read_dir(root.join("scripts/saves"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    saves.retain(|path| path.extension().is_some_and(|extension| extension == "sav"));
    saves.sort();
    assert_eq!(saves.len(), 73, "committed campaign save corpus");
    saves.push(root.join("tests/fixtures/savegame/legacy-save.00"));
    for path in &saves {
        roundtrip(path)?;
    }
    println!(
        "V3a: {} of {} saves byte-identical",
        saves.len(),
        saves.len()
    );
    Ok(())
}
