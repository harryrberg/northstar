// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, Editor, Formatting};

fn editor(active: &[u8], cursor: usize) -> Editor {
    let mut carrier = active.to_vec();
    carrier.resize(128, 0x1a);
    Editor::restore_with_formatting(
        Document::from_bytes(carrier),
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
fn removes_suffix_and_ctrl_u_restores_it_at_the_same_cursor() {
    let mut editor = editor(b"ABCDE", 2);
    editor.apply(&Command::DeleteToLineEnd).unwrap();
    assert_eq!(&editor.document().bytes()[..2], b"AB");
    assert_eq!(editor.snapshot().cursor_byte, 2);
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..5], b"ABCDE");
    assert_eq!(editor.snapshot().cursor_byte, 2);
}

#[test]
fn hard_break_stays_and_noop_at_break_retains_erasure_buffer() {
    let mut editor = editor(b"AB\r\nCD", 1);
    editor.apply(&Command::DeleteToLineEnd).unwrap();
    assert_eq!(&editor.document().bytes()[..5], b"A\r\nCD");
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..6], b"AB\r\nCD");
    editor.apply(&Command::NextCharacter).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 2);
    let before = editor.document().bytes().to_vec();
    editor.apply(&Command::DeleteToLineEnd).unwrap();
    assert_eq!(editor.document().bytes(), before);
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..7], b"ABB\r\nCD");
}

#[test]
fn high_bit_word_markers_survive_erasure_and_restoration() {
    let original = b"ON\xc5 TWO\xcf THREE";
    let mut editor = editor(original, 4);
    editor.apply(&Command::DeleteToLineEnd).unwrap();
    assert_eq!(&editor.document().bytes()[..4], b"ON\xc5 ");
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..original.len()], original);
}

#[test]
fn opaque_line_and_block_marks_are_rejected_atomically() {
    let mut opaque = editor(b"A\tBC", 1);
    let before = opaque.snapshot();
    let bytes = opaque.document().bytes().to_vec();
    assert!(opaque.apply(&Command::DeleteToLineEnd).is_err());
    assert_eq!(opaque.snapshot(), before);
    assert_eq!(opaque.document().bytes(), bytes);

    let mut marked = editor(b"ABCDE", 1);
    marked.apply(&Command::MarkBlockBegin).unwrap();
    let before = marked.snapshot();
    let bytes = marked.document().bytes().to_vec();
    assert!(marked.apply(&Command::DeleteToLineEnd).is_err());
    assert_eq!(marked.snapshot(), before);
    assert_eq!(marked.document().bytes(), bytes);
}
