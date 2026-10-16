// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, Editor, Formatting};

#[test]
fn rejected_typing_batches_preserve_bytes_markers_cursor_and_modes() {
    for insert in [false, true] {
        for (raw, cursor, text) in [
            (vec![b'A'; 64], 64, " X"),
            (b"ALPH\xc1 BETA".to_vec(), 5, " X\u{e9}"),
            (b"ALPH\xc1 BETA".to_vec(), 5, " X\n"),
            (b"ALPHA".to_vec(), 0, ".RM 20"),
        ] {
            let mut editor =
                Editor::restore(Document::from_bytes(raw.clone()), cursor, insert).unwrap();
            let before = editor.snapshot();
            assert!(
                editor
                    .apply(&Command::InsertAscii { text: text.into() })
                    .is_err()
            );
            assert_eq!(editor.document().bytes(), raw);
            assert_eq!(editor.snapshot(), before);
        }
    }
}

#[test]
fn a_late_unqualified_wrap_does_not_commit_earlier_typing_or_normalization() {
    for (raw, cursor, text) in [
        (vec![b'A'; 61], 61, " XY-Z"),
        (
            [b"ALPHA ".repeat(10), b"BETA ".to_vec()].concat(),
            65,
            ".RM",
        ),
        ([b"ALPHA ".repeat(10), b".WORD".to_vec()].concat(), 65, "X"),
    ] {
        for insert in [false, true] {
            let mut editor = Editor::restore_with_formatting(
                Document::from_bytes(raw.clone()),
                cursor,
                insert,
                Formatting {
                    right_justify: false,
                    ..Formatting::default()
                },
            )
            .unwrap();
            let before = editor.snapshot();
            assert!(
                editor
                    .apply(&Command::InsertAscii { text: text.into() })
                    .is_err()
            );
            assert_eq!(editor.document().bytes(), raw);
            assert_eq!(editor.snapshot(), before);
        }
    }
}

#[test]
fn unsupported_selected_line_or_eof_payload_rejects_both_edit_operations() {
    for raw in [
        b"ALPHA\tBETA".as_slice(),
        b"ALPHA\x02BETA\x02",
        b"ALPHA\x9b\xc4\x1c",
        b"ALPHA\x9b\xc4",
        b"ALPHA\r",
        b"ALPHA\x7f",
        b"ALPHA\r\x8aBETA",
        b"ALPHA\x8d\x8aBETA",
        b"ALPHA\x1aTAIL",
        b".RM 65",
    ] {
        for insert in [false, true] {
            let mut editor =
                Editor::restore(Document::from_bytes(raw.to_vec()), 0, insert).unwrap();
            let before = editor.snapshot();
            for command in [
                Command::Enter,
                Command::InsertAscii {
                    text: "NEW ".into(),
                },
            ] {
                assert!(editor.apply(&command).is_err(), "{raw:?}");
                assert_eq!(editor.document().bytes(), raw);
                assert_eq!(editor.snapshot(), before);
            }
        }
    }
}

#[test]
fn enter_does_not_silently_create_an_unqualified_dot_command_line() {
    let raw = b"ALPHA.RM 20";
    let mut editor = Editor::restore(Document::from_bytes(raw.to_vec()), 5, true).unwrap();
    let before = editor.snapshot();
    assert!(editor.apply(&Command::Enter).is_err());
    assert_eq!(editor.document().bytes(), raw);
    assert_eq!(editor.snapshot(), before);
}
