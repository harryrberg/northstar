// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor, Formatting};
use northstar_desktop::{projection::Projection, screen::Screen};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

fn screen_bytes(editor: &Editor, label: &str) -> Vec<u8> {
    let projection = Projection::new(editor);
    Screen::editor(&projection, &editor.snapshot(), label, None).monochrome_bytes()
}

fn compare_screen(actual: &[u8], name: &str, expected_sha256: &str, evidence_env: &str) {
    assert_eq!(actual.len(), 80 * 25 * 2);
    if let Some(directory) = std::env::var_os(evidence_env) {
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
        &screen_bytes(&editor, "C:PIPE1.DOC"),
        "PIPE1-before-screen.bin",
        "7016d55d25ba33892bf5087031df3a1995bbddfebc49efa78bc9b644a0757de1",
        "NORTHSTAR_DOS_SCREENS",
    );
    for value in case["commands"].as_array().unwrap() {
        let command: Command = serde_json::from_value(value.clone()).unwrap();
        editor.apply(&command).unwrap();
    }
    compare_screen(
        &screen_bytes(&editor, "C:PIPE1.DOC"),
        "PIPE1-after-screen.bin",
        "e6c1617dd204b3ec1c92844a6701d45c0c1510b7a82f2cd5cfb7eb3f69b81ac3",
        "NORTHSTAR_DOS_SCREENS",
    );
}

#[test]
fn long_document_scroll_matches_retained_dos_screens() {
    let mut carrier = Vec::new();
    let mut starts = Vec::new();
    for line in [
        "ALPHA BETA GAMMA",
        "X",
        "LONG SECOND PARAGRAPH",
        "THIRD LINE",
    ]
    .into_iter()
    .chain(std::iter::repeat_n("FOURTH LINE", 14))
    {
        starts.push(carrier.len());
        carrier.extend_from_slice(line.as_bytes());
        carrier.extend_from_slice(b"\r\n");
    }
    carrier.resize(carrier.len().div_ceil(128) * 128, 0x1a);
    assert_eq!(
        format!("{:x}", Sha256::digest(&carrier)),
        "0370f3f91599a1b662bb66efb0a09fd5a95e94b1443fd382baddc831e9d99050"
    );
    let mut editor = Editor::restore_with_formatting(
        Document::from_bytes(carrier),
        starts[0],
        true,
        Formatting {
            right_justify: false,
            ..Formatting::default()
        },
    )
    .unwrap();
    for (line, name, expected) in [
        (
            12,
            "nav-row12-screen.bin",
            "05d9ef90e403b7e2ac769eb6220a8db2bed0efa092054e816ac681c8b49d5477",
        ),
        (
            13,
            "nav-row13-screen.bin",
            "a722f096fb7b038299a2af08beaa92996d862ef2688c5f77d41582fa26ecb95e",
        ),
        (
            14,
            "nav-row14-screen.bin",
            "376bef6bb75110cea334adde75b6cb33c6bea6e9554091e59f8114a4dccd7c4b",
        ),
    ] {
        while editor.snapshot().cursor_byte < starts[line - 1] {
            editor.apply(&Command::NextLine).unwrap();
        }
        assert_eq!(editor.snapshot().cursor_byte, starts[line - 1]);
        compare_screen(
            &screen_bytes(&editor, "C:NAV1.DOC"),
            name,
            expected,
            "NORTHSTAR_NAV_SCREENS",
        );
    }
}

#[test]
fn later_opaque_data_does_not_change_the_current_page_label() {
    let document = Document::from_bytes(b"A\r\n\x01B\x1a\x1a".to_vec());
    let at_start = Editor::restore(document.clone(), 0, true).unwrap();
    let at_b = Editor::restore(document, 4, true).unwrap();
    let start = Screen::editor(
        &Projection::new(&at_start),
        &at_start.snapshot(),
        "C:OPAQUE.DOC",
        None,
    );
    let later = Screen::editor(
        &Projection::new(&at_b),
        &at_b.snapshot(),
        "C:OPAQUE.DOC",
        None,
    );
    assert_eq!(
        (start.cells[0][24].glyph, start.cells[0][25].glyph),
        ('0', '1')
    );
    assert_eq!(
        (later.cells[0][24].glyph, later.cells[0][25].glyph),
        ('?', '?')
    );
}
