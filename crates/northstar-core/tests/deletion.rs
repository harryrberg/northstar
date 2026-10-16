// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor, Formatting};

#[test]
fn deleting_unqualified_structures_is_atomic() {
    for (raw, cursor) in [
        (b"AB\x8d\nCD\x1a\x1a".as_slice(), 2),
        (b"AB\r\x8aCD\x1a\x1a", 2),
        (b"AB\r\nC\tD\x1a\x1a", 2),
        (b"AB\r\n.CD\x1a\x1a", 2),
        (b"AB\x1aTAIL", 1),
        (b"AB\xa0C\x1a\x1a", 1),
    ] {
        let mut editor = Editor::restore_with_formatting(
            Document::from_bytes(raw.to_vec()),
            cursor,
            true,
            Formatting {
                right_justify: false,
                ..Formatting::default()
            },
        )
        .unwrap();
        let before = editor.snapshot();
        assert!(editor.apply(&Command::DeleteCharacter).is_err(), "{raw:?}");
        assert_eq!(editor.document().bytes(), raw);
        assert_eq!(editor.snapshot(), before);
    }
}

#[test]
fn deletion_at_eof_preserves_the_original_carrier() {
    let raw = b"AB\x1a\x1a\x1a";
    let mut editor = Editor::restore_with_formatting(
        Document::from_bytes(raw.to_vec()),
        2,
        true,
        Formatting {
            right_justify: false,
            ..Formatting::default()
        },
    )
    .unwrap();
    editor.apply(&Command::DeleteCharacter).unwrap();
    assert_eq!(editor.document().bytes(), raw);
    assert!(!editor.snapshot().modified);
    assert_eq!(editor.snapshot().cursor_byte, 2);
}
