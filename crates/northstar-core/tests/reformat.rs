// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor, Formatting};

fn formatting() -> Formatting {
    Formatting {
        right_justify: false,
        ..Formatting::default()
    }
}

#[test]
fn reformat_rejections_preserve_all_state_and_original_bytes() {
    let cases: Vec<(Vec<u8>, usize, Formatting)> = vec![
        (b"ALPHA BETA".to_vec(), 0, Formatting::default()),
        (
            b"ALPHA BETA".to_vec(),
            0,
            Formatting {
                left_margin: 2,
                ..formatting()
            },
        ),
        (
            b"ALPHA BETA".to_vec(),
            0,
            Formatting {
                right_margin: 66,
                ..formatting()
            },
        ),
        (
            b"ALPHA BETA".to_vec(),
            0,
            Formatting {
                line_spacing: 2,
                ..formatting()
            },
        ),
        (
            b"ALPHA BETA".to_vec(),
            0,
            Formatting {
                document_mode: false,
                ..formatting()
            },
        ),
        (b"ALPHA BETA".to_vec(), 1, formatting()),
        (b"ALPHA\x8d\nBETA".to_vec(), 7, formatting()),
        (b"ALPHA-BETA".to_vec(), 0, formatting()),
        (b"ALPHA\tBETA".to_vec(), 0, formatting()),
        (b"ALPHA\x8d\x8aBETA".to_vec(), 0, formatting()),
        (b"ALPHA\r\x8aBETA".to_vec(), 0, formatting()),
        (b"ALPHA\x02BETA\x02".to_vec(), 0, formatting()),
        (b"ALPHA\x9b\xc4\x1c".to_vec(), 0, formatting()),
        (b"ALPHA\x9b\xc4".to_vec(), 0, formatting()),
        (b"ALPHA\r".to_vec(), 0, formatting()),
        (b"ALPHA\x7f".to_vec(), 0, formatting()),
        (b"ALPHA\x1aTAIL".to_vec(), 0, formatting()),
        (b".RM 65".to_vec(), 0, formatting()),
        (vec![b'A'; 66], 0, formatting()),
        ([b"  ".as_slice(), &[b'A'; 64]].concat(), 0, formatting()),
    ];
    for (raw, cursor, mode) in cases {
        let mut editor =
            Editor::restore_with_formatting(Document::from_bytes(raw.clone()), cursor, false, mode)
                .unwrap();
        let before = editor.snapshot();
        assert!(editor.apply(&Command::Reformat).is_err(), "{raw:?}");
        assert_eq!(editor.document().bytes(), raw);
        assert_eq!(editor.snapshot(), before);
    }
}

#[test]
fn formatted_documents_allow_structural_navigation_and_mode_changes() {
    let mut editor = Editor::restore_with_formatting(
        Document::from_bytes(b"ALPHA BETA".to_vec()),
        0,
        true,
        formatting(),
    )
    .unwrap();
    editor.apply(&Command::Reformat).unwrap();
    let saved = editor.document().bytes().to_vec();
    editor.apply(&Command::DocumentStart).unwrap();
    for _ in 0..5 {
        editor.apply(&Command::NextCharacter).unwrap();
    }
    assert_eq!(editor.snapshot().cursor_byte, 5);
    editor.apply(&Command::PreviousCharacter).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 4);
    editor.apply(&Command::ToggleInsert).unwrap();
    assert!(!editor.snapshot().insert);
    assert_eq!(editor.document().bytes(), saved);
}

#[test]
fn ascii_mutations_reject_unimplemented_formatting_before_changing_state() {
    for mode in [
        Formatting {
            left_margin: 2,
            ..formatting()
        },
        Formatting {
            right_margin: 20,
            ..formatting()
        },
        Formatting {
            line_spacing: 2,
            ..formatting()
        },
        Formatting {
            document_mode: false,
            ..formatting()
        },
    ] {
        let mut editor =
            Editor::restore_with_formatting(Document::from_bytes(b"ALPHA".to_vec()), 5, true, mode)
                .unwrap();
        let before = editor.snapshot();
        for command in [
            Command::Enter,
            Command::InsertAscii {
                text: " BETA".into(),
            },
        ] {
            assert!(editor.apply(&command).is_err());
            assert_eq!(editor.snapshot(), before);
            assert_eq!(editor.document().bytes(), b"ALPHA");
        }
    }
}

#[test]
fn reformat_preserves_opaque_content_outside_selected_hard_paragraph() {
    let prefix = b"\x9b\xc4\x1c\x02FIRST\x02\r\n";
    let suffix = b"\x13SECOND\x93\t\x7f\r\n";
    let raw = [prefix.as_slice(), b"ALPHA BETA\r\n", suffix].concat();
    let mut editor = Editor::restore_with_formatting(
        Document::from_bytes(raw),
        prefix.len(),
        true,
        formatting(),
    )
    .unwrap();
    editor.apply(&Command::Reformat).unwrap();
    let bytes = editor.document().bytes();
    assert_eq!(&bytes[..prefix.len()], prefix);
    assert_eq!(
        &bytes[prefix.len()..prefix.len() + 12],
        b"ALPH\xc1 BETA\r\n"
    );
    assert_eq!(
        &bytes[prefix.len() + 12..editor.document().active_end()],
        suffix
    );
    assert_eq!(editor.snapshot().cursor_byte, prefix.len() + 12);
    let before = editor.snapshot();
    let saved = bytes.to_vec();
    assert!(editor.apply(&Command::Reformat).is_err());
    assert_eq!(editor.document().bytes(), saved);
    assert_eq!(editor.snapshot(), before);
}
