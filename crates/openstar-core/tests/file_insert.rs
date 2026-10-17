// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, Editor, Formatting};

fn carrier(active: &[u8]) -> Document {
    let mut bytes = active.to_vec();
    bytes.resize(128, 0x1a);
    Document::from_bytes(bytes)
}

fn editor(cursor: usize) -> Editor {
    Editor::restore_with_formatting(
        carrier(b"ABCDE"),
        cursor,
        true,
        Formatting {
            right_justify: false,
            ..Formatting::default()
        },
    )
    .unwrap()
}

#[test]
fn inserts_active_bytes_without_eof_padding_and_leaves_cursor_at_the_start() {
    let source = carrier(b"XY");
    let source_before = source.bytes().to_vec();
    let mut editor = editor(2);
    editor.insert_wordstar_file(&source).unwrap();
    assert_eq!(&editor.document().bytes()[..7], b"ABXYCDE");
    assert_eq!(editor.document().bytes().len(), 128);
    assert!(
        editor.document().bytes()[7..]
            .iter()
            .all(|byte| *byte == 0x1a)
    );
    assert_eq!(editor.snapshot().cursor_byte, 2);
    assert!(editor.snapshot().insert);
    assert_eq!(source.bytes(), source_before);
}

#[test]
fn insertion_at_the_end_mark_retains_both_marks() {
    let mut editor = editor(1);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    editor.insert_wordstar_file(&carrier(b"XY")).unwrap();
    assert_eq!(&editor.document().bytes()[..7], b"ABCXYDE");
    let state = editor.snapshot();
    assert_eq!(
        (state.cursor_byte, state.block_begin, state.block_end),
        (3, Some(1), Some(3))
    );
}

#[test]
fn unsupported_input_and_mode_leave_the_document_unchanged() {
    let mut editor = editor(2);
    for source in [
        Document::from_bytes(b"XY".to_vec()),
        carrier(b"X\r\nY"),
        carrier(b"X Y"),
    ] {
        let before = editor.snapshot();
        let bytes = editor.document().bytes().to_vec();
        assert!(editor.insert_wordstar_file(&source).is_err());
        assert_eq!(editor.snapshot(), before);
        assert_eq!(editor.document().bytes(), bytes);
    }
    editor.apply(&Command::ToggleInsert).unwrap();
    assert!(editor.insert_wordstar_file(&carrier(b"XY")).is_err());
    assert_eq!(&editor.document().bytes()[..5], b"ABCDE");
}

#[test]
fn marks_spanning_two_lines_are_not_qualified() {
    let mut editor = Editor::restore_with_formatting(
        carrier(b"AB\r\nCDE"),
        1,
        true,
        Formatting {
            right_justify: false,
            ..Formatting::default()
        },
    )
    .unwrap();
    editor.apply(&Command::MarkBlockBegin).unwrap();
    for _ in 0..4 {
        editor.apply(&Command::NextCharacter).unwrap();
    }
    editor.apply(&Command::MarkBlockEnd).unwrap();
    let before = editor.snapshot();
    let bytes = editor.document().bytes().to_vec();
    assert!(editor.insert_wordstar_file(&carrier(b"XY")).is_err());
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), bytes);
}
