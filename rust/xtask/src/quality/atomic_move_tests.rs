use super::{baseline::Baseline, budgets, run, tool, walk};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Fixture {
    _directory: tempfile::TempDir,
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let directory = tool::fixture_directory();
        let root = directory.path().join("work");
        std::fs::create_dir(&root).unwrap();
        tool::status(
            Command::new("git")
                .current_dir(directory.path())
                .args(["init", "-q"]),
        )
        .unwrap();
        tool::status(
            Command::new("git")
                .current_dir(directory.path())
                .args(["config", "core.worktree"])
                .arg(&root),
        )
        .unwrap();
        Self {
            _directory: directory,
            root,
        }
    }
    fn path(&self) -> &Path {
        &self.root
    }
}

fn write(root: &Path, path: &str, text: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn arguments(pairs: &[(&str, &str)]) -> Vec<String> {
    ["move", "--prefix", "--base", "HEAD"]
        .into_iter()
        .chain(pairs.iter().flat_map(|(old, new)| [*old, *new]))
        .map(str::to_string)
        .collect()
}

fn relocate(root: &Path, old: &str, new: &str) {
    std::fs::rename(
        root.join(format!("rust/src/battle/context_access/{old}.rs")),
        root.join(format!("rust/src/battle/context_access/{new}.rs")),
    )
    .unwrap();
    let lib = root.join("rust/src/battle/context_access/mod.rs");
    let text = std::fs::read_to_string(&lib).unwrap();
    std::fs::write(lib, text.replace(old, new)).unwrap();
}

fn staged_fixture() -> Fixture {
    let root = Fixture::new();
    write(
        root.path(),
        "rust/Cargo.toml",
        "[workspace]\n[package]\nname=\"uqm\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    );
    write(
        root.path(),
        "rust/Cargo.lock",
        "version = 4\n[[package]]\nname = \"uqm\"\nversion = \"0.1.0\"\n",
    );
    write(root.path(), "rust/src/lib.rs", "pub mod battle;\n");
    write(
        root.path(),
        "rust/src/battle/mod.rs",
        "pub mod context_access;\n",
    );
    write(
        root.path(),
        "rust/src/battle/context_access/mod.rs",
        "pub mod left; pub mod right;\n",
    );
    for name in ["left", "right"] {
        write(
            root.path(),
            &format!("rust/src/battle/context_access/{name}.rs"),
            "pub unsafe fn execute(pointer: *mut u8) { unsafe { *pointer = 7; } }\n",
        );
    }
    write(
        root.path(),
        "rust/xtask/src/quality/macros.rs",
        "const HANDLERS: &[(&str, &str)] = &[];",
    );
    let mut baseline = Baseline::default();
    budgets::set_scoped(
        &mut baseline.budget,
        "stage_b_access_unsafe",
        2,
        [
            (
                "unsafe|unsafe_blocks|uqm::battle::context_access::left::execute".into(),
                1,
            ),
            (
                "unsafe|unsafe_block_lines|uqm::battle::context_access::left::execute".into(),
                1,
            ),
        ]
        .into(),
        false,
    )
    .unwrap();
    let measured = walk::measure(root.path(), None, false).unwrap();
    baseline.entries = budgets::scoped_entries(&measured, &baseline.budget);
    baseline
        .write(&root.path().join("rust/xtask/quality-baseline.toml"))
        .unwrap();
    for args in [
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "base",
        ],
    ] {
        tool::status(Command::new("git").current_dir(root.path()).args(args)).unwrap();
    }
    relocate(root.path(), "left", "staged_left");
    run(
        root.path(),
        &arguments(&[(
            "uqm::battle::context_access::left",
            "uqm::battle::context_access::staged_left",
        )]),
    )
    .unwrap();
    relocate(root.path(), "right", "staged_right");
    run(
        root.path(),
        &arguments(&[(
            "uqm::battle::context_access::right",
            "uqm::battle::context_access::staged_right",
        )]),
    )
    .unwrap();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let mut baseline = Baseline::read(&path).unwrap();
    for (host, left, right) in [("linux", 3, 4), ("macos", 5, 6)] {
        baseline.lint.insert(
            host.into(),
            [
                (
                    "clippy::unwrap_used|uqm::battle::context_access::staged_left".into(),
                    left,
                ),
                (
                    "clippy::unwrap_used|uqm::battle::context_access::staged_right".into(),
                    right,
                ),
            ]
            .into(),
        );
    }
    baseline.write(&path).unwrap();
    relocate(root.path(), "staged_left", "engine_left");
    relocate(root.path(), "staged_right", "engine_right");
    root
}

#[test]
fn related_destinations_move_atomically_without_credit_growth() {
    let root = staged_fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let before = std::fs::read(&path).unwrap();
    let error = run(
        root.path(),
        &arguments(&[(
            "uqm::battle::context_access::staged_left",
            "uqm::battle::context_access::engine_left",
        )]),
    )
    .unwrap_err();
    assert!(
        error.contains("uqm::battle::context_access::staged_right"),
        "{error}"
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    run(
        root.path(),
        &arguments(&[
            (
                "uqm::battle::context_access::staged_left",
                "uqm::battle::context_access::engine_left",
            ),
            (
                "uqm::battle::context_access::staged_right",
                "uqm::battle::context_access::engine_right",
            ),
        ]),
    )
    .unwrap();
    let baseline = Baseline::read(&path).unwrap();
    let measured = walk::measure(root.path(), None, false).unwrap();
    for (host, left, right) in [("linux", 3, 4), ("macos", 5, 6)] {
        assert_eq!(
            baseline.lint[host],
            [
                (
                    "clippy::unwrap_used|uqm::battle::context_access::engine_left".into(),
                    left
                ),
                (
                    "clippy::unwrap_used|uqm::battle::context_access::engine_right".into(),
                    right
                ),
            ]
            .into()
        );
    }
    let nonzero = |entries: std::collections::BTreeMap<String, u64>| {
        entries
            .into_iter()
            .filter(|(_, value)| *value != 0)
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    assert_eq!(
        nonzero(baseline.entries.clone()),
        nonzero(budgets::scoped_entries(&measured, &baseline.budget))
    );
    assert_eq!(
        baseline.moved["uqm::battle::context_access::left"],
        "uqm::battle::context_access::engine_left"
    );
    assert_eq!(
        baseline.moved["uqm::battle::context_access::right"],
        "uqm::battle::context_access::engine_right"
    );
    assert_eq!(baseline.budget["stage_b_access_unsafe"].limit, 2);
    assert!(baseline.budget["stage_b_access_unsafe"]
        .scopes
        .keys()
        .all(|key| key.contains("uqm::battle::context_access::engine_left")));
}

#[test]
fn invalid_batch_members_leave_the_baseline_unchanged() {
    let root = staged_fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let before = std::fs::read(&path).unwrap();
    let left = "uqm::battle::context_access::staged_left";
    let right = "uqm::battle::context_access::staged_right";
    let target = "uqm::battle::context_access::engine_left";
    let right_target = "uqm::battle::context_access::engine_right";
    for (tail, expected) in [
        (
            vec![(right, "uqm::battle::context_access::absent")],
            "is absent",
        ),
        (
            vec![(right, right_target), ("uqm::missing", "uqm::invented")],
            "does not exist at base",
        ),
        (
            vec![(right, target)],
            "move target uqm::battle::context_access::engine_left already credited",
        ),
        (
            vec![
                (right, right_target),
                ("uqm::battle::context_access::left", "uqm::other"),
            ],
            "already credited",
        ),
    ] {
        let mut pairs = vec![(left, target)];
        pairs.extend(tail);
        for approved in [false, true] {
            let mut args = arguments(&pairs);
            if approved {
                args.push("--allow-growth".into());
            }
            let error = run(root.path(), &args).unwrap_err();
            assert!(error.contains(expected), "{error}");
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
    }
}

#[test]
fn retained_sources_and_non_declaration_members_are_rejected() {
    let root = staged_fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let before = std::fs::read(&path).unwrap();
    let mut args = arguments(&[
        (
            "uqm::battle::context_access::left::execute",
            "uqm::battle::context_access::engine_left::execute",
        ),
        ("uqm::battle::context_access", "uqm::not_a_declaration"),
    ]);
    args.retain(|arg| arg != "--prefix");
    assert!(run(root.path(), &args)
        .unwrap_err()
        .contains("source is not a declaration"));
    let source = root.path().join("rust/src/battle/context_access");
    std::fs::copy(source.join("engine_right.rs"), source.join("right.rs")).unwrap();
    let lib = source.join("mod.rs");
    let mut text = std::fs::read_to_string(&lib).unwrap();
    text.push_str("pub mod right;\n");
    std::fs::write(lib, text).unwrap();
    let error = run(
        root.path(),
        &arguments(&[
            (
                "uqm::battle::context_access::staged_left",
                "uqm::battle::context_access::engine_left",
            ),
            (
                "uqm::battle::context_access::staged_right",
                "uqm::battle::context_access::engine_right",
            ),
        ]),
    )
    .unwrap_err();
    assert!(error.contains("still exists"), "{error}");
    assert_eq!(std::fs::read(path).unwrap(), before);
}

#[test]
fn incomplete_pair_lists_are_rejected_before_editing() {
    let root = staged_fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let before = std::fs::read(&path).unwrap();
    for pairs in [vec![], vec!["old"], vec!["old", "new", "unpaired"]] {
        let args = ["move"]
            .into_iter()
            .chain(pairs)
            .map(str::to_string)
            .collect::<Vec<_>>();
        assert!(run(root.path(), &args)
            .unwrap_err()
            .contains("usage: quality move"));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let mut baseline = Baseline::read(&path).unwrap();
    assert!(baseline
        .add_moves(
            &[],
            &Default::default(),
            &Default::default(),
            &Default::default()
        )
        .unwrap_err()
        .contains("at least one"));
    assert_eq!(baseline, Baseline::read(&path).unwrap());
}

#[test]
fn packaged_accessors_keep_only_their_existing_scoped_credit() {
    let mut scopes = std::collections::BTreeMap::new();
    let old = "uqm::engine::battle::access::project";
    budgets::set_scoped(
        &mut scopes,
        "stage_b_access_unsafe",
        2,
        [
            (format!("unsafe|unsafe_blocks|{old}"), 1),
            (format!("unsafe|unsafe_block_lines|{old}"), 1),
        ]
        .into(),
        false,
    )
    .unwrap();
    let moved = budgets::relocated(
        &scopes,
        &[("uqm::engine".into(), "uqm-engine".into())].into(),
    )
    .unwrap();
    assert_eq!(moved["stage_b_access_unsafe"].limit, 2);
    assert!(moved["stage_b_access_unsafe"]
        .scopes
        .keys()
        .all(|key| key.contains("uqm-engine::battle::access::project")));
    assert_eq!(
        budgets::parse(&budgets::write(&moved).unwrap()).unwrap(),
        moved
    );
    assert!(budgets::relocated(
        &scopes,
        &[("uqm::engine".into(), "uqm-content".into())].into()
    )
    .is_err());
    assert!(budgets::relocated(
        &scopes,
        &[(
            "uqm::engine::battle::access".into(),
            "uqm-engine::world".into()
        )]
        .into()
    )
    .is_err());
}

#[test]
fn whole_file_move_keeps_expect_proofs_with_their_host_counters() {
    let root = Fixture::new();
    for (path, text) in [
        ("rust/Cargo.toml", "[workspace]\n[package]\nname=\"uqm\"\nversion=\"0.1.0\"\nedition=\"2021\"\n"),
        ("rust/Cargo.lock", "version = 4\n[[package]]\nname = \"uqm\"\nversion = \"0.1.0\"\n"),
        ("rust/xtask/src/quality/macros.rs", "const HANDLERS: &[(&str, &str)] = &[];"),
        ("rust/src/lib.rs", "pub mod old;\n"),
        ("rust/src/old.rs", "pub struct Old; impl Old { pub fn response(r: u32) -> u16 { u16::try_from(r).expect(\"COUNT\") } }\n"),
    ] {
        write(root.path(), path, text);
    }
    let source = walk::measure(root.path(), None, false).unwrap();
    let site = source.expect_sites.iter().next().unwrap();
    let proof_key = format!("uqm::old::@file|{}|{}", site.owner, site.expression);
    let mut baseline = Baseline {
        entries: source.baseline(),
        lint: [
            (
                "linux".into(),
                [
                    ("clippy::expect_used|uqm::old::@file".into(), 1),
                    ("clippy::unwrap_used|untouched::@file".into(), 2),
                ]
                .into(),
            ),
            (
                "macos".into(),
                [
                    ("clippy::expect_used|uqm::old::@file".into(), 1),
                    ("clippy::unwrap_used|untouched::@file".into(), 3),
                ]
                .into(),
            ),
        ]
        .into(),
        expect_sites: [
            ("linux".into(), [(proof_key.clone(), 1)].into()),
            ("macos".into(), [(proof_key, 1)].into()),
        ]
        .into(),
        ..Baseline::default()
    };
    let archived = Baseline {
        entries: source.baseline(),
        ..Baseline::default()
    };
    std::fs::remove_file(root.path().join("rust/src/old.rs")).unwrap();
    write(root.path(), "rust/src/lib.rs", "pub mod host;\n");
    write(root.path(), "rust/src/host.rs", "pub struct Host; impl Host { pub fn response(r: u32) -> u16 { u16::try_from(r).expect(\"COUNT\") } }\n");
    let current = walk::measure(root.path(), None, false).unwrap();
    baseline
        .add_moves(
            &[
                ("uqm::old", "uqm::host"),
                ("uqm::old::Old", "uqm::host::Host"),
                ("uqm::old::<Old>", "uqm::host::<Host>"),
            ],
            &current,
            &source,
            &Default::default(),
        )
        .unwrap();
    baseline.tighten(&current, Some(&archived)).unwrap();
    for (host, untouched) in [("linux", 2), ("macos", 3)] {
        assert_eq!(
            baseline.lint[host]["clippy::expect_used|uqm::host::@file"],
            1
        );
        assert!(!baseline.lint[host].contains_key("clippy::expect_used|uqm::old::@file"));
        assert_eq!(
            baseline.lint[host]["clippy::unwrap_used|untouched::@file"],
            untouched
        );
        assert_eq!(baseline.expect_sites[host].len(), 1);
        assert_eq!(baseline.expect_sites[host].values().sum::<u64>(), 1);
        super::expect_sites::validate(&baseline.expect_sites[host], &current).unwrap();
        let mut grown = baseline.lint[host].clone();
        grown.insert("clippy::expect_used|uqm::host::@file".into(), 2);
        assert!(tool::compare(&grown, &baseline.lint[host], &baseline.moved).is_err());
    }
}
