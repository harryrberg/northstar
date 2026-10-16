// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, Editor, Formatting};

fn editor(raw: &[u8], cursor: usize) -> Editor {
    Editor::restore_with_formatting(
        Document::from_bytes(raw.to_vec()),
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
fn saved_checkpoint_does_not_clear_the_line_buffer() {
    let mut carrier = b"AB\r\nCD".to_vec();
    carrier.resize(128, 0x1a);
    let mut editor = editor(&carrier, 0);
    editor.apply(&Command::DeleteLine).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 0);
    assert_eq!(&editor.document().bytes()[..2], b"CD");
    editor.mark_saved();
    assert!(!editor.snapshot().modified);
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(editor.document().bytes(), carrier);
    assert_eq!(editor.snapshot().cursor_byte, 0);
    assert!(editor.snapshot().modified);
}

#[test]
fn unqualified_line_erasure_and_insertion_fail_atomically() {
    for raw in [
        b"AB\x8d\nCD\x1a\x1a".as_slice(),
        b"AB\r\x8aCD\x1a\x1a",
        b"AB\r\n.CD\x1a\x1a",
        b"A\xc2C\x1a\x1a",
        b"AB\xa0C\x1a\x1a",
        b"AB\x1aTAIL",
    ] {
        let mut editor = editor(raw, 0);
        let before = editor.snapshot();
        assert!(editor.apply(&Command::DeleteLine).is_err(), "{raw:?}");
        assert_eq!(editor.document().bytes(), raw);
        assert_eq!(editor.snapshot(), before);
    }

    let mut editor = editor(b"AB\r\nCD\r\nEF\x1a\x1a", 0);
    editor.apply(&Command::DeleteLine).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    let before = editor.snapshot();
    let bytes = editor.document().bytes().to_vec();
    assert!(editor.apply(&Command::UneraseLine).is_err());
    assert_eq!(editor.document().bytes(), bytes);
    assert_eq!(editor.snapshot(), before);
    editor.apply(&Command::DocumentStart).unwrap();
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..10], b"AB\r\nCD\r\nEF");
}

#[test]
fn word_erasure_replaces_the_shared_buffer_and_break_erasure_clears_it() {
    let mut editor = editor(b"AB\r\nCD\r\nEF\x1a\x1a", 0);
    editor.apply(&Command::DeleteLine).unwrap();
    editor.apply(&Command::DeleteWord).unwrap();
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..6], b"CD\r\nEF");

    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::DeleteWord).unwrap();
    let after_join = editor.document().bytes().to_vec();
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(editor.document().bytes(), after_join);
}

#[test]
fn word_erasure_at_eof_keeps_the_buffer_and_unqualified_content_is_atomic() {
    let mut draft = editor(b"ONE TWO\x1a\x1a", 0);
    draft.apply(&Command::DeleteWord).unwrap();
    for _ in 0..4 {
        draft.apply(&Command::NextCharacter).unwrap();
    }
    draft.apply(&Command::DeleteWord).unwrap();
    draft.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&draft.document().bytes()[..7], b" TWOONE");

    for raw in [
        b"ONE;TWO\x1a\x1a".as_slice(),
        b"ONE\xa0TWO\x1a\x1a",
        b"A\xc2C\x1a\x1a",
    ] {
        let mut editor = editor(raw, 0);
        let before = editor.snapshot();
        assert!(editor.apply(&Command::DeleteWord).is_err());
        assert_eq!(editor.document().bytes(), raw);
        assert_eq!(editor.snapshot(), before);
    }
}
