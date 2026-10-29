#[path = "support/battle.rs"]
mod battle;
#[path = "support/battle_context.rs"]
mod battle_context;

use battle::{Battle, Ship};

use uqm_rust::collections::observation::{self, Event};
use uqm_rust::engine::battle::{Cx, Selection, ShipId, WeaponOrigin, WeaponSpawn};
use uqm_rust::ships::native::abi::*;

fn alternate_pre(cx: &mut Cx<'_>, ship: ShipId) {
    cx.ship_state_mut(ship).energy_counter = 71;
}
fn alternate_post(cx: &mut Cx<'_>, ship: ShipId) {
    cx.ship_state_mut(ship).energy_counter = 39;
}
fn alternate_weapon(cx: &mut Cx<'_>, ship: ShipId, subject: WeaponOrigin) -> WeaponSpawn {
    cx.ship_state_mut(ship).weapon_counter = subject.turn_wait();
    vec![cx.alloc_element()]
}

#[test]
fn callback_modes_control_dispatch_metadata_and_observations() {
    battle_context::run(|| {
        let owner = Battle::new(8);
        let ship = Ship::bare(&owner, SPECIES_ID_VUX_ID, 0);
        // SAFETY: owner keeps the serialized test battle published until all fixtures drop.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                observation::enable();
                cx.set_preprocess(ship.ship_id, Selection::Disabled);
                observation::drain();
                assert!(!cx.invoke_ship_process(ship.ship_id, false));
                assert!(!observation::drain()
                    .iter()
                    .any(|e| matches!(e, Event::Hook { .. })));
                cx.set_preprocess(ship.ship_id, Selection::Override(alternate_pre));
                cx.set_postprocess(ship.ship_id, Selection::Override(alternate_post));
                assert!(cx.invoke_ship_process(ship.ship_id, false));
                assert_eq!(cx.ship_state(ship.ship_id).energy_counter, 71);
                assert!(cx.invoke_ship_process(ship.ship_id, true));
                assert_eq!(cx.ship_state(ship.ship_id).energy_counter, 39);
                cx.set_weapon(ship.ship_id, Selection::Override(alternate_weapon));
                let mut projected = cx.weapon_origin(ship.id);
                projected.set_turn_wait(23);
                let outputs = cx.spawn_weapons(ship.ship_id, projected);
                assert_eq!(cx.ship_state(ship.ship_id).weapon_counter, 23);
                cx.dispose(outputs[0].unwrap());
                cx.set_weapon(ship.ship_id, Selection::Disabled);
                assert!(cx.spawn_weapons(ship.ship_id, projected).is_empty());
                cx.set_preprocess(ship.ship_id, Selection::Default);
            });
            assert_ne!(ship.snapshot().flags & PLAYER_SHIP as u16, 0);
        }
    });
}

#[test]
fn ordered_lifecycle_preserves_reverse_order_exhaustion_and_primitive_reuse() {
    battle_context::run(|| {
        let _owner = Battle::new(3);
        // SAFETY: this test owns all element slots and the game-thread context.
        unsafe {
            uqm_rust::app::bridge::enter(|cx| {
                let a = cx.alloc_element().unwrap();
                let b = cx.alloc_element().unwrap();
                let c = cx.alloc_element().unwrap();
                assert!(cx.alloc_element().is_none());
                cx.put(a);
                cx.insert_before(b, Some(a));
                cx.insert_before(c, Some(a));
                assert_eq!(cx.elements().head(), Some(b));
                assert_eq!(cx.elements().succ(b), Some(c));
                assert_eq!(cx.elements().succ(c), Some(a));
                assert_eq!(cx.elements().tail(), Some(a));
                assert_eq!(cx.elements().pred(a), Some(c));
                assert_eq!(cx.elements().pred(c), Some(b));
                for id in [b, c, a] {
                    cx.unlink(id);
                    cx.dispose(id);
                    assert!(!cx.elements().contains(id));
                }
                assert!(cx.elements().head().is_none());
                for _ in 0..400 {
                    let id = cx.alloc_element().unwrap();
                    assert_ne!(id, a);
                    cx.dispose(id);
                }
            });
        }
    });
}

#[test]
fn projected_autoaim_uses_both_positions_without_mutating_live_target() {
    battle_context::run(|| {
        let owner = Battle::new(8);
        // SAFETY: fixtures retain the frame and descriptor resources for these hook calls.
        unsafe {
            let ship = Ship::new(&owner, SPECIES_ID_ARILOU_ID, 0);
            let enemy = Ship::new(&owner, SPECIES_ID_VUX_ID, 1);
            uqm_rust::app::bridge::enter(|cx| {
                cx.element_mut(enemy.id).current.location = POINT { x: 4000, y: 4300 };
                cx.element_mut(enemy.id).next.location = POINT { x: 4000, y: 4300 };
                cx.set_target(ship.id, Some(enemy.id));
                let live = cx.element(ship.id);
                let mut subject = cx.weapon_origin(ship.id);
                subject.current.location = POINT { x: 3800, y: 4300 };
                subject.next.location = POINT { x: 3800, y: 4300 };
                let mut facing = 0;
                assert_eq!(cx.track_subject(&mut subject, &mut facing), 4);
                assert_eq!(facing, 1);
                let weapons = cx.spawn_weapons(ship.ship_id, subject);
                let laser = cx.element(weapons[0].unwrap());
                let (dx, dy) = laser.velocity.get_current_components();
                assert!(dx > 0);
                assert_eq!(dy, 0);
                assert_eq!(laser.current.location.y, 4300);
                assert_eq!(cx.target(ship.id), Some(enemy.id));
                assert_eq!(
                    cx.element(ship.id).current.location.x,
                    live.current.location.x
                );
                assert_eq!(
                    cx.element(ship.id).current.location.y,
                    live.current.location.y
                );
                assert_eq!(cx.element(ship.id).next.location.x, live.next.location.x);
                assert_eq!(cx.element(ship.id).next.location.y, live.next.location.y);
                cx.dispose(weapons[0].unwrap());
            });
        }
    });
}

#[test]
fn native_races_cannot_escape_the_real_accessor_boundary() -> Result<(), Box<dyn std::error::Error>>
{
    use std::process::Command;

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let executable = std::env::current_exe()?;
    let deps = executable
        .parent()
        .ok_or("test executable has no directory")?;
    let profile = deps.parent().ok_or("deps has no profile directory")?;
    let scratch = tempfile::Builder::new()
        .prefix("native-boundary-")
        .tempdir_in(profile)?;
    let source = std::fs::read_to_string(root.join("src/lib.rs"))?;
    let mut actual_modules = String::new();
    for line in source.lines() {
        if let Some(module) = line
            .strip_prefix("pub mod ")
            .and_then(|s| s.strip_suffix(';'))
        {
            let file = root.join(format!("src/{module}.rs"));
            let file = if file.exists() {
                file
            } else {
                root.join(format!("src/{module}/mod.rs"))
            };
            actual_modules.push_str(&format!("#[path = {file:?}]\n"));
        }
        actual_modules.push_str(line);
        actual_modules.push('\n');
    }
    // Match Cargo's exact dependency versions and feature builds.
    let hash = executable
        .file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.rsplit_once('-'))
        .ok_or("test executable has no Cargo hash")?
        .1;
    let fingerprints = profile.join(".fingerprint");
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(fingerprints.join(
        format!("uqm-{hash}/test-integration-test-native_race_api.json"),
    ))?)?;
    let libraries = std::fs::read_dir(&fingerprints)?
        .map(|entry| entry.map(|e| e.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    let mut externs = Vec::new();
    for dependency in manifest["deps"]
        .as_array()
        .ok_or("missing Cargo dependencies")?
    {
        let name = dependency[1].as_str().ok_or("dependency has no name")?;
        if name == "uqm_rust" {
            continue;
        }
        let fingerprint: String = dependency[3]
            .as_u64()
            .ok_or("dependency has no fingerprint")?
            .to_le_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let library = libraries
            .iter()
            .find(|p| {
                std::fs::read_to_string(p.join(format!("lib-{name}")))
                    .is_ok_and(|value| value == fingerprint)
            })
            .ok_or_else(|| format!("missing Cargo fingerprint for {name}"))?;
        let hash = library
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.rsplit_once('-'))
            .ok_or("dependency has no Cargo hash")?
            .1;
        externs.push(format!(
            "{name}={}",
            deps.join(format!("lib{name}-{hash}.rlib")).display()
        ));
    }
    let probes = [
        ("control", "fn probe(cx: &mut Cx<'_>, ship: ShipId, id: ElementId) { cx.ship_state_mut(ship).crew = 1; cx.element_mut(id).life = 1; }", ""),
        ("context_free_world_read", "fn probe() { let _ = crate::battle::flagship_config(); }", "E0425"),
        ("context_free_world_write", "fn probe() { crate::battle::set_flagship_crew(1); }", "E0425"),
        ("untyped_story", "fn probe(cx: &mut Cx<'_>) { let _ = cx.story_state(\"SHOFIXTI_RECRUITED\"); }", "E0308"),
        ("ship_record", "fn probe(cx: &mut Cx<'_>, ship: ShipId) { cx.ship_state_mut(ship).record.RaceDescPtr = std::ptr::NonNull::dangling().as_ptr(); let _ = cx.race_stats(ship); }", "E0616"),
        ("element_record", "fn probe(cx: &mut Cx<'_>, id: ElementId) { cx.element_mut(id).record.pParent = std::ptr::NonNull::dangling().as_ptr(); let _ = cx.element_frame_set_first(id, true); }", "E0616"),
        ("table", "fn probe(cx: &mut Cx<'_>, id: ElementId) { cx.b.elements[id].next.image.farray = std::ptr::NonNull::dangling().as_ptr(); let _ = cx.element_frame_set_first(id, true); }", "E0616"),
        ("frame", "fn probe() { let _ = crate::engine::battle::views::Frame(std::ptr::NonNull::dangling().as_ptr()); }", "E0603"),
        ("frames", "fn probe() { let _ = crate::engine::battle::views::Frames(std::ptr::NonNull::dangling().as_ptr()); }", "E0603"),
        ("descriptor", "fn probe() { let _ = crate::engine::battle::RaceDesc::from_legacy(crate::engine::battle::record::RACE_DESC::EMPTY); }", "E0624"),
        ("projection", "fn probe(cx: &mut Cx<'_>, p: std::ptr::NonNull<u8>) { *cx.project_mut(p) = 1; }", "E0624"),
        ("primitive", "fn probe(cx: &mut Cx<'_>, id: ElementId) { cx.primitive(id).Type = 0; }", "E0624"),
        ("raw_frame", "fn probe(cx: &mut Cx<'_>) { cx.abs_frame(std::ptr::NonNull::dangling().as_ptr(), 0); }", "E0624"),
        ("element_operand", "fn probe() { let _ = crate::engine::battle::record::ElementOperand(std::ptr::null_mut::<u8>()); }", "E0603"),
        ("collision_operands", "fn probe() { let _ = crate::engine::battle::record::CollisionOperands(std::ptr::null_mut::<u8>(), std::ptr::null_mut::<u8>(), std::ptr::null_mut::<u8>(), std::ptr::null_mut::<u8>()); }", "E0603"),
        ("decoder", "fn probe(raw: &crate::engine::battle::record::ELEMENT) { let _ = crate::engine::battle::views::ElementState::decode(raw); }", "E0624"),
        ("legacy_table", "fn probe(cx: &mut Cx<'_>, id: ElementId) { cx.records()[id].next.image.farray = std::ptr::NonNull::dangling().as_ptr(); }", "E0133"),
        ("legacy_subject", "fn probe(cx: &mut Cx<'_>, raw: &crate::engine::battle::record::ELEMENT) { let _ = cx.legacy_subject(raw); }", "E0133"),
        ("sound", "fn probe(cx: &mut Cx<'_>) { cx.queue_sound(1, None); }", "E0624"),
        ("subject", "fn probe(mut subject: crate::engine::battle::Subject) { subject.intersect.IntersectStamp.frame = std::ptr::NonNull::dangling().as_ptr(); }", "E0616"),
    ];
    for (name, probe, error) in probes {
        let input = scratch.path().join(format!("{name}.rs"));
        let output = scratch.path().join(format!("{name}.rmeta"));
        std::fs::write(&input, format!("{actual_modules}\nmod native_race_probe {{ use crate::engine::battle::{{Cx, ElementId, ShipId}}; {probe} }}"))?;
        let mut command = Command::new("rustc");
        command
            .args([
                "--edition=2021",
                "--crate-name=uqm_rust",
                "--crate-type=lib",
                "--emit=metadata",
                "--cap-lints=allow",
            ])
            .arg("--cfg")
            .arg("feature=\"audio_heart\"")
            .arg("--cfg")
            .arg("feature=\"debug-process\"")
            .arg("-L")
            .arg(format!("dependency={}", deps.display()))
            .arg("-o")
            .arg(output)
            .arg(input);
        for dependency in &externs {
            command.arg("--extern").arg(dependency);
        }
        let result = command.output()?;
        let errors = String::from_utf8(result.stderr)?;
        assert_eq!(
            result.status.success(),
            error.is_empty(),
            "{name}: {errors}"
        );
        if !error.is_empty() {
            assert!(
                errors.contains(&format!("error[{error}]")),
                "{name}: {errors}"
            );
            assert_eq!(errors.matches("error[E").count(), 1, "{name}: {errors}");
        }
    }
    Ok(())
}
