// SPDX-License-Identifier: GPL-3.0-only
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(0);

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "northstar-test-{}-{nanos}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        // Only files created by this test are placed in this unique directory.
        if let Ok(entries) = fs::read_dir(&self.0) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
        let _ = fs::remove_dir(&self.0);
    }
}

fn run(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_northstar"))
        .args(args)
        .output()
        .unwrap()
}

fn oracle(id: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/oracles")
        .join(format!("{id}.json"))
}

fn case(id: &str) -> Value {
    serde_json::from_slice(&fs::read(oracle(id)).unwrap()).unwrap()
}

#[test]
fn retained_enter_scenarios_pass_through_the_real_cli() {
    for id in ["enter-insert-on-alpha-beta", "enter-insert-off-next-line"] {
        let output = run(&["replay".as_ref(), oracle(id).as_os_str()]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["passed"], true);
        assert_eq!(report["target_executed"], false);
        assert_eq!(report["carrier_bytes"], 128);
        assert_eq!(report["active_hex"], "414c5048410d0a42455441");
        assert_eq!(report["compared_fields"]["cursor_byte"], id.contains("off"));
    }
}

#[test]
fn retained_editing_and_reformat_scenarios_match_saved_bytes_cursor_and_modes() {
    use sha2::{Digest, Sha256};
    let scratch = Scratch::new();
    let mut paths: Vec<_> = fs::read_dir(oracle("unused").parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            let name = path.file_name().unwrap().to_string_lossy();
            name.starts_with("reformat-") || name.starts_with("edit-")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty());
    for (index, input) in paths.iter().enumerate() {
        let spec: Value = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
        let saved = scratch.path(&format!("case-{index}.doc"));
        let output = run(&[
            "replay".as_ref(),
            input.as_os_str(),
            "--output".as_ref(),
            saved.as_os_str(),
        ]);
        assert!(
            output.status.success(),
            "{}: {}",
            input.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        let saved_hash = format!("{:x}", Sha256::digest(fs::read(saved).unwrap()));
        assert_eq!(saved_hash, spec["expected"]["carrier_sha256"]);
        for field in ["cursor_byte", "insert", "formatting"] {
            assert_eq!(report["compared_fields"][field], true);
            assert_eq!(report["state"][field], spec["expected"][field]);
        }
        assert_eq!(report["target_executed"], false);
    }
}

#[test]
fn reformat_requires_explicit_profile_and_formatting_without_publishing_failures() {
    let scratch = Scratch::new();
    for (index, condition) in [
        "missing_formatting",
        "old_profile",
        "justify_on",
        "wrong_margin",
        "comparison",
    ]
    .iter()
    .enumerate()
    {
        let mut spec = case("reformat-plain-wrap");
        match *condition {
            "missing_formatting" => {
                spec["initial"]
                    .as_object_mut()
                    .unwrap()
                    .remove("formatting");
            }
            "old_profile" => spec["profile"] = json!("ws4-ascii-enter-v1"),
            "justify_on" => spec["initial"]["formatting"]["right_justify"] = json!(true),
            "wrong_margin" => spec["initial"]["formatting"]["right_margin"] = json!(66),
            _ => spec["expected"]["formatting"]["right_justify"] = json!(true),
        }
        let input = scratch.path(&format!("invalid-{index}.json"));
        let saved = scratch.path(&format!("invalid-{index}.doc"));
        fs::write(&input, serde_json::to_vec(&spec).unwrap()).unwrap();
        assert!(
            !run(&[
                "replay".as_ref(),
                input.as_os_str(),
                "--output".as_ref(),
                saved.as_os_str()
            ])
            .status
            .success()
        );
        assert!(!saved.exists());
    }
}

#[test]
fn editing_requires_declared_formatting_and_does_not_publish_unsupported_batches() {
    let scratch = Scratch::new();
    for (index, condition) in ["missing_formatting", "beyond_margin"].iter().enumerate() {
        let mut spec = case("edit-type-column-65");
        if *condition == "missing_formatting" {
            spec["initial"]
                .as_object_mut()
                .unwrap()
                .remove("formatting");
        } else {
            spec["commands"] = json!([{"command": "insert_ascii", "text": " X"}]);
        }
        let input = scratch.path(&format!("editing-{index}.json"));
        let saved = scratch.path(&format!("editing-{index}.doc"));
        fs::write(&input, serde_json::to_vec(&spec).unwrap()).unwrap();
        let output = run(&[
            "replay".as_ref(),
            input.as_os_str(),
            "--output".as_ref(),
            saved.as_os_str(),
        ]);
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains(if *condition == "missing_formatting" {
                "formatting"
            } else {
                "typing beyond column 65"
            }),
            "{error}"
        );
        assert!(!saved.exists());
    }
}

#[test]
fn new_output_is_exact_and_an_existing_file_is_never_overwritten() {
    let scratch = Scratch::new();
    let output_path = scratch.path("saved.doc");
    let input = oracle("enter-insert-on-alpha-beta");
    let before = fs::read(&input).unwrap();
    let args = [
        "replay".as_ref(),
        input.as_os_str(),
        "--output".as_ref(),
        output_path.as_os_str(),
    ];
    assert!(run(&args).status.success());
    let mut expected = b"ALPHA\r\nBETA".to_vec();
    expected.resize(128, 0x1a);
    assert_eq!(fs::read(&output_path).unwrap(), expected);
    let second = run(&args);
    assert!(!second.status.success());
    assert_eq!(fs::read(&output_path).unwrap(), expected);
    assert_eq!(fs::read(&input).unwrap(), before);
}

#[test]
fn mismatches_and_unknown_commands_do_not_publish_output() {
    let scratch = Scratch::new();
    for malformed in [false, true] {
        let mut spec = case("enter-insert-on-alpha-beta");
        if malformed {
            spec["commands"][0] = json!({"command":"unsupported_test_command"});
        } else {
            spec["expected"]["carrier_sha256"] = json!("0".repeat(64));
        }
        let input = scratch.path(if malformed {
            "unknown.json"
        } else {
            "mismatch.json"
        });
        fs::write(&input, serde_json::to_vec(&spec).unwrap()).unwrap();
        let output_path = scratch.path(if malformed {
            "unknown.doc"
        } else {
            "mismatch.doc"
        });
        assert!(
            !run(&[
                "replay".as_ref(),
                input.as_os_str(),
                "--output".as_ref(),
                output_path.as_os_str()
            ])
            .status
            .success()
        );
        assert!(!output_path.exists());
    }
}

#[test]
fn replay_rejects_unknown_fields_and_invalid_carrier_hex() {
    let scratch = Scratch::new();
    for (index, mutate) in ["unknown_field", "bad_hex", "bad_version", "bad_digest"]
        .iter()
        .enumerate()
    {
        let mut spec = case("enter-insert-on-alpha-beta");
        match *mutate {
            "unknown_field" => spec["initial"]["assume_compatible"] = json!(true),
            "bad_hex" => spec["initial"]["carrier_hex"] = json!("1xz"),
            "bad_version" => spec["schema_version"] = json!(2),
            _ => spec["provenance"]["sources"][0]["sha256"] = json!("not-a-digest"),
        }
        let path = scratch.path(&format!("invalid-{index}.json"));
        fs::write(&path, serde_json::to_vec(&spec).unwrap()).unwrap();
        assert!(!run(&["replay".as_ref(), path.as_os_str()]).status.success());
    }
}

#[test]
fn inspect_covers_unknown_bytes_and_limits_large_padding_previews() {
    let scratch = Scratch::new();
    let input = scratch.path("inspect.doc");
    let mut raw = b"A\r\x9b\xc4\x1c\x7f".to_vec();
    raw.resize(4096, 0x1a);
    fs::write(&input, &raw).unwrap();
    let output = run(&["inspect".as_ref(), input.as_os_str()]);
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["carrier_bytes"], 4096);
    assert_eq!(report["unresolved_token_count"], 2);
    assert_eq!(report["source_modified"], false);
    assert_eq!(report["token_preview_complete"], true);
    assert_eq!(
        report["token_preview"].as_array().unwrap().last().unwrap()["raw_prefix_complete"],
        false
    );
    assert!(output.stdout.len() < 5000);
    assert_eq!(fs::read(input).unwrap(), raw);
}

#[test]
fn inspect_reports_truncated_token_preview_without_losing_total_coverage() {
    let scratch = Scratch::new();
    let input = scratch.path("long.doc");
    fs::write(&input, vec![b'A'; 1000]).unwrap();
    let output = run(&["inspect".as_ref(), input.as_os_str()]);
    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["token_count"], 1000);
    assert_eq!(report["token_preview"].as_array().unwrap().len(), 64);
    assert_eq!(report["token_preview_complete"], false);
}
