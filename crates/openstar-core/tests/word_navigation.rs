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
