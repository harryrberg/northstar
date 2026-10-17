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
fn marked_block_writer_matches_the_observed_dos_carrier_without_editing_source() {
    let mut editor = editor(b"ABCDE", 1);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    let before = editor.snapshot();
    let source = editor.document().bytes().to_vec();
    let output = editor.marked_block_carrier().unwrap();
    assert_eq!(&output[..2], b"BC");
    assert_eq!(output.len(), 128);
    assert!(output[2..].iter().all(|byte| *byte == 0x1a));
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), source);
}

#[test]
fn marked_block_writer_rejects_unmarked_and_cross_line_ranges() {
    let unmarked = editor(b"ABCDE", 1);
    assert!(unmarked.marked_block_carrier().is_err());

    let mut editor = editor(b"AB\r\nCD", 1);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::DocumentEnd).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    let before = editor.snapshot();
    assert!(editor.marked_block_carrier().is_err());
    assert_eq!(editor.snapshot(), before);
}

#[test]
fn quick_block_jumps_require_a_mark_and_leave_carrier_unchanged() {
    let mut editor = editor(b"ONE TWO\r\nTHREE FOUR\r\n", 0);
    let carrier = editor.document().bytes().to_vec();
    let before = editor.snapshot();
    assert!(editor.apply(&Command::JumpBlockBegin).is_err());
    assert!(editor.apply(&Command::JumpBlockEnd).is_err());
    assert_eq!(editor.snapshot(), before);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    for _ in 0..4 {
        editor.apply(&Command::NextCharacter).unwrap();
    }
    editor.apply(&Command::MarkBlockEnd).unwrap();
    editor.apply(&Command::JumpBlockBegin).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 0);
    editor.apply(&Command::JumpBlockEnd).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 4);
    editor.apply(&Command::DocumentEnd).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 21);
    assert_eq!(editor.document().bytes(), carrier);
}

#[test]
fn block_display_toggle_keeps_marks_cursor_and_saved_bytes() {
    let mut editor = editor(b"ABCDE", 1);
    let carrier = editor.document().bytes().to_vec();
    assert!(!editor.snapshot().block_visible);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    assert!(editor.snapshot().block_visible);
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    editor.apply(&Command::ToggleBlockDisplay).unwrap();
    let hidden = editor.snapshot();
    assert!(!hidden.block_visible);
    assert_eq!(
        (hidden.cursor_byte, hidden.block_begin, hidden.block_end),
        (3, Some(1), Some(3))
    );
    assert!(!hidden.modified);
    assert_eq!(editor.document().bytes(), carrier);
    editor.apply(&Command::JumpBlockBegin).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 1);
    editor.apply(&Command::JumpBlockEnd).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 3);
    editor.apply(&Command::ToggleBlockDisplay).unwrap();
    assert!(editor.snapshot().block_visible);
    assert_eq!(editor.document().bytes(), carrier);
}

#[test]
fn missing_or_empty_block_rejects_copy_without_mutation() {
    let mut editor = editor(b"ABCDE", 2);
    let carrier = editor.document().bytes().to_vec();
    let before = editor.snapshot();
    assert!(editor.apply(&Command::CopyBlock).is_err());
    assert_eq!(editor.document().bytes(), carrier);
    assert_eq!(editor.snapshot(), before);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    assert!(!editor.snapshot().modified);
    let before = editor.snapshot();
    assert!(editor.apply(&Command::CopyBlock).is_err());
    assert_eq!(editor.document().bytes(), carrier);
    assert_eq!(editor.snapshot(), before);
}

#[test]
fn copy_rejects_unqualified_source_and_capacity_atomically() {
    for (active, end) in [(&b"AB\x8d\nCD"[..], 4), (&b"AB\r\nX\tY\r\nCD"[..], 9)] {
        let mut editor = editor(active, 1);
        editor.apply(&Command::MarkBlockBegin).unwrap();
        while editor.snapshot().cursor_byte < end {
            editor.apply(&Command::NextCharacter).unwrap();
        }
        editor.apply(&Command::MarkBlockEnd).unwrap();
        let before = editor.snapshot();
        let bytes = editor.document().bytes().to_vec();
        assert!(editor.apply(&Command::CopyBlock).is_err());
        assert_eq!(editor.document().bytes(), bytes);
        assert_eq!(editor.snapshot(), before);
    }

    let mut editor = editor(&[b'A'; 64], 0);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    for _ in 0..62 {
        editor.apply(&Command::NextCharacter).unwrap();
    }
    let before = editor.snapshot();
    assert!(editor.apply(&Command::CopyBlock).is_err());
    assert_eq!(editor.snapshot(), before);
}

#[test]
fn unrelated_mutation_clears_marks_without_changing_copy_results() {
    let mut editor = editor(b"ABCDE", 1);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    editor
        .apply(&Command::InsertAscii { text: "Z".into() })
        .unwrap();
    assert_eq!(editor.snapshot().block_begin, None);
    assert_eq!(editor.snapshot().block_end, None);
    assert!(editor.apply(&Command::CopyBlock).is_err());
}

#[test]
fn copied_hard_break_validates_the_destination_suffix() {
    let mut editor = editor(b"AB\r\nCD\r\nX\xc4", 1);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    for _ in 0..3 {
        editor.apply(&Command::NextCharacter).unwrap();
    }
    let before = editor.snapshot();
    let bytes = editor.document().bytes().to_vec();
    assert!(editor.apply(&Command::CopyBlock).is_err());
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), bytes);
}

#[test]
fn move_and_delete_reject_unmeasured_ranges_atomically() {
    let mut editor = editor(b"ABCDE", 1);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    editor.apply(&Command::MarkBlockEnd).unwrap();
    let bytes = editor.document().bytes().to_vec();
    editor.apply(&Command::DocumentStart).unwrap();
    let before = editor.snapshot();
    assert!(editor.apply(&Command::DeleteBlock).is_err());
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), bytes);

    for _ in 0..2 {
        editor.apply(&Command::NextCharacter).unwrap();
    }
    let before = editor.snapshot();
    assert!(editor.apply(&Command::MoveBlock).is_err());
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), bytes);

    let mut other = self::editor(b"AB\r\nCD", 1);
    other.apply(&Command::MarkBlockBegin).unwrap();
    for _ in 0..3 {
        other.apply(&Command::NextCharacter).unwrap();
    }
    other.apply(&Command::MarkBlockEnd).unwrap();
    let before = other.snapshot();
    let bytes = other.document().bytes().to_vec();
    assert!(other.apply(&Command::DeleteBlock).is_err());
    assert_eq!(other.snapshot(), before);
    assert_eq!(other.document().bytes(), bytes);

    let mut other = self::editor(b"ABCDE", 1);
    other.apply(&Command::MarkBlockBegin).unwrap();
    other.apply(&Command::NextCharacter).unwrap();
    other.apply(&Command::NextCharacter).unwrap();
    other.apply(&Command::MarkBlockEnd).unwrap();
    other.apply(&Command::NextCharacter).unwrap();
    other.apply(&Command::NextCharacter).unwrap();
    other.apply(&Command::ToggleInsert).unwrap();
    let before = other.snapshot();
    let bytes = other.document().bytes().to_vec();
    assert!(other.apply(&Command::MoveBlock).is_err());
    assert!(other.apply(&Command::DeleteBlock).is_err());
    assert_eq!(other.snapshot(), before);
    assert_eq!(other.document().bytes(), bytes);

    let mut other = self::editor(b"ABCDE", 2);
    other.apply(&Command::MarkBlockBegin).unwrap();
    other.apply(&Command::NextCharacter).unwrap();
    other.apply(&Command::NextCharacter).unwrap();
    other.apply(&Command::MarkBlockEnd).unwrap();
    let before = other.snapshot();
    let bytes = other.document().bytes().to_vec();
    assert!(other.apply(&Command::DeleteBlock).is_err());
    assert_eq!(other.snapshot(), before);
    assert_eq!(other.document().bytes(), bytes);
    other.apply(&Command::DocumentStart).unwrap();
    let before = other.snapshot();
    assert!(other.apply(&Command::MoveBlock).is_err());
    assert_eq!(other.snapshot(), before);
    assert_eq!(other.document().bytes(), bytes);
}

#[test]
fn block_delete_rejects_two_hard_breaks_atomically() {
    let mut editor = editor(b"AB\r\nCD\r\nEFG", 1);
    editor.apply(&Command::MarkBlockBegin).unwrap();
    for _ in 0..6 {
        editor.apply(&Command::NextCharacter).unwrap();
    }
    editor.apply(&Command::MarkBlockEnd).unwrap();
    let before = editor.snapshot();
    let bytes = editor.document().bytes().to_vec();
    assert!(editor.apply(&Command::DeleteBlock).is_err());
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), bytes);
}
