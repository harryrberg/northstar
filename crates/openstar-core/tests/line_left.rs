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
fn removes_prefix_and_ctrl_u_restores_it_at_line_start() {
    let mut editor = editor(b"ABCDE", 2);
    editor.apply(&Command::DeleteToLineStart).unwrap();
    assert_eq!(&editor.document().bytes()[..3], b"CDE");
    assert_eq!(editor.snapshot().cursor_byte, 0);
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..5], b"ABCDE");
    assert_eq!(editor.snapshot().cursor_byte, 0);
}

#[test]
fn noop_at_line_start_retains_the_erasure_buffer() {
    let mut editor = editor(b"ABCDE", 2);
    editor.apply(&Command::DeleteToLineStart).unwrap();
    let before = editor.document().bytes().to_vec();
    editor.apply(&Command::DeleteToLineStart).unwrap();
    assert_eq!(editor.document().bytes(), before);
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..5], b"ABCDE");
}

#[test]
fn hard_break_remains_when_prefix_or_full_line_is_removed() {
    let mut prefix = editor(b"AB\r\nCD", 1);
    prefix.apply(&Command::DeleteToLineStart).unwrap();
    assert_eq!(&prefix.document().bytes()[..5], b"B\r\nCD");
    prefix.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&prefix.document().bytes()[..6], b"AB\r\nCD");

    let mut full_line = editor(b"AB\r\nCD", 2);
    full_line.apply(&Command::DeleteToLineStart).unwrap();
    assert_eq!(&full_line.document().bytes()[..4], b"\r\nCD");
    full_line.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&full_line.document().bytes()[..6], b"AB\r\nCD");
}

#[test]
fn high_bit_word_markers_survive_erasure_and_restoration() {
    let original = b"ON\xc5 TW\xcf THREE";
    let mut editor = editor(original, 8);
    editor.apply(&Command::DeleteToLineStart).unwrap();
    assert_eq!(&editor.document().bytes()[..5], b"THREE");
    editor.apply(&Command::UneraseLine).unwrap();
    assert_eq!(&editor.document().bytes()[..original.len()], original);
}

#[test]
fn opaque_line_and_block_marks_are_rejected_atomically() {
    let mut opaque = editor(b"A\tBC", 1);
    let before = opaque.snapshot();
    let bytes = opaque.document().bytes().to_vec();
    assert!(opaque.apply(&Command::DeleteToLineStart).is_err());
    assert_eq!(opaque.snapshot(), before);
    assert_eq!(opaque.document().bytes(), bytes);

    let mut marked = editor(b"ABCDE", 1);
    marked.apply(&Command::MarkBlockBegin).unwrap();
    let before = marked.snapshot();
    let bytes = marked.document().bytes().to_vec();
    assert!(marked.apply(&Command::DeleteToLineStart).is_err());
    assert_eq!(marked.snapshot(), before);
    assert_eq!(marked.document().bytes(), bytes);
}
