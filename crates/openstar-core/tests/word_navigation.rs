// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, EditError, Editor};

#[test]
fn unqualified_word_movement_preserves_cursor_and_carrier() {
    for (carrier, cursor, command, expected) in [
        (
            b"ONE.TWO\x1a\x1a".to_vec(),
            0,
            Command::NextWord,
            EditError::UnsupportedDocument { offset: 3 },
        ),
        (
            b"ONE\x8d\nTWO\x1a\x1a".to_vec(),
            3,
            Command::NextWord,
            EditError::UnqualifiedOperation("word movement across a soft or page-marked break"),
        ),
        (
            b"A\r\x8aB\x1a\x1a".to_vec(),
            1,
            Command::NextWord,
            EditError::UnsupportedDocument { offset: 1 },
        ),
        (
            b"ONE\tTWO\x1a\x1a".to_vec(),
            0,
            Command::PreviousWord,
            EditError::UnsupportedDocument { offset: 3 },
        ),
        (
            b"ONE- TWO\x1a\x1a".to_vec(),
            0,
            Command::NextWord,
            EditError::UnsupportedDocument { offset: 3 },
        ),
        (
            b"ONE\r\n\x1bB\x1c\x1a\x1a".to_vec(),
            3,
            Command::NextWord,
            EditError::UnsupportedDocument { offset: 5 },
        ),
    ] {
        let mut editor =
            Editor::restore(Document::from_bytes(carrier.clone()), cursor, true).unwrap();
        assert_eq!(editor.apply(&command), Err(expected));
        assert_eq!(editor.snapshot().cursor_byte, cursor);
        assert_eq!(editor.document().bytes(), carrier);
        assert!(!editor.snapshot().modified);
    }
}

#[test]
fn word_movement_accepts_marker_created_by_typing_a_space() {
    let mut editor = Editor::restore(Document::from_bytes(b"ONE".to_vec()), 3, true).unwrap();
    editor
        .apply(&Command::InsertAscii {
            text: " TWO".into(),
        })
        .unwrap();
    let after_typing = editor.document().bytes().to_vec();
    assert!(after_typing.starts_with(b"ON\xC5 TWO"));
    assert_eq!(editor.snapshot().cursor_byte, 7);

    editor.apply(&Command::PreviousWord).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 4);
    editor.apply(&Command::PreviousWord).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 0);
    editor.apply(&Command::NextWord).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 4);
    assert_eq!(editor.document().bytes(), after_typing);
}
