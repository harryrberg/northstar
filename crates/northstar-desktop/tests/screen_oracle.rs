// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor, Formatting};
use northstar_desktop::{projection::Projection, screen::Screen};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn screen_bytes(editor: &Editor) -> Vec<u8> {
    let projection = Projection::new(editor);
    Screen::editor(&projection, &editor.snapshot(), "C:PIPE1.DOC", None).monochrome_bytes()
}

fn compare_screen(actual: &[u8], name: &str, expected_sha256: &str) {
    assert_eq!(actual.len(), 80 * 25 * 2);
    if let Some(directory) = std::env::var_os("NORTHSTAR_DOS_SCREENS") {
        let reference = fs::read(PathBuf::from(directory).join(name)).unwrap();
        let mut printed = 0;
        let mut differences = 0;
        for (index, (got, expected)) in actual.iter().zip(&reference).enumerate() {
            if got != expected {
                differences += 1;
                let cell = index / 2;
                if printed < 20 {
                    eprintln!(
                        "{name}: row {}, column {}, {}: got {got:#04x}, DOS {expected:#04x}",
                        cell / 80,
                        cell % 80,
                        if index % 2 == 0 { "glyph" } else { "attribute" }
                    );
                    printed += 1;
                }
            }
        }
        if differences > 0 {
            eprintln!("{name}: {differences} byte differences");
        }
    }
    assert_eq!(format!("{:x}", Sha256::digest(actual)), expected_sha256);
}

#[test]
fn pipe1_editor_screen_matches_retained_dos_captures() {
    let oracle = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/oracles/edit-reformat-edit-split-chain.json");
    let case: Value = serde_json::from_slice(&fs::read(oracle).unwrap()).unwrap();
    let hex = case["initial"]["carrier_hex"].as_str().unwrap();
    let carrier = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect();
    let formatting: Formatting =
        serde_json::from_value(case["initial"]["formatting"].clone()).unwrap();
    let mut editor =
        Editor::restore_with_formatting(Document::from_bytes(carrier), 0, true, formatting)
            .unwrap();
    compare_screen(
        &screen_bytes(&editor),
        "PIPE1-before-screen.bin",
        "7016d55d25ba33892bf5087031df3a1995bbddfebc49efa78bc9b644a0757de1",
    );
    for value in case["commands"].as_array().unwrap() {
        let command: Command = serde_json::from_value(value.clone()).unwrap();
        editor.apply(&command).unwrap();
    }
    compare_screen(
        &screen_bytes(&editor),
        "PIPE1-after-screen.bin",
        "e6c1617dd204b3ec1c92844a6701d45c0c1510b7a82f2cd5cfb7eb3f69b81ac3",
    );
}
