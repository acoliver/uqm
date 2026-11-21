#![forbid(unsafe_code)]
use uqm_rust::automation::script::{parse_script, validate_script, Action, MenuKey};

#[test]
fn missing_addon_proof_captures_restoration_before_cancel() {
    let path = "tests/fixtures/native/ship-spin-fallback-v1.json";
    let document = parse_script(
        include_bytes!("fixtures/native/ship-spin-fallback-v1.json"),
        path,
    )
    .unwrap();
    let script = validate_script(document, path).unwrap();
    let steps = script.steps();
    let before = steps
        .iter()
        .position(|step| matches!(step, Action::Capture(capture) if capture.label == "androsynth-picker-before-missing-addon-spin"))
        .unwrap();
    let spin = steps
        .iter()
        .position(|step| matches!(step, Action::TapMenuKey(tap) if tap.key == MenuKey::Special))
        .unwrap();
    let restored = steps
        .iter()
        .position(|step| matches!(step, Action::Capture(capture) if capture.label == "picker-restored-without-optional-video"))
        .unwrap();
    let navigate = steps
        .iter()
        .position(|step| matches!(step, Action::TapMenuKey(tap) if tap.key == MenuKey::Right))
        .unwrap();
    let cancel = steps
        .iter()
        .enumerate()
        .skip(spin + 1)
        .find_map(|(index, step)| {
            matches!(step, Action::TapMenuKey(tap) if tap.key == MenuKey::Cancel).then_some(index)
        })
        .unwrap();
    assert!(before < spin && spin < restored && restored < navigate && navigate < cancel);
    assert!(steps[navigate + 1..cancel].iter().any(|step| {
        matches!(step, Action::Capture(capture) if capture.label == "picker-navigation-after-missing-addon-spin" && capture.expect_change)
    }));
    assert!(steps[cancel + 1..].iter().any(|step| {
        matches!(step, Action::Capture(capture) if capture.label == "fleet-editor-after-restored-picker-cancel" && capture.expect_change)
    }));
}
