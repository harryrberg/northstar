// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, EditError, Editor};

#[test]
fn dos_observed_vertical_sequence_clamps_to_each_actual_line() {
    let mut carrier = b"ABCDE\r\nX\r\n\r\nLONG LINE HERE".to_vec();
    carrier.resize(128, 0x1a);
    let mut editor = Editor::restore(Document::from_bytes(carrier.clone()), 4, true).unwrap();
    for (command, expected) in [
        (Command::NextLine, 8),
        (Command::NextLine, 10),
        (Command::NextLine, 12),
        (Command::NextLine, 12),
        (Command::PreviousLine, 10),
        (Command::PreviousLine, 7),
        (Command::PreviousLine, 0),
        (Command::PreviousLine, 0),
    ] {
        editor.apply(&command).unwrap();
        assert_eq!(editor.snapshot().cursor_byte, expected);
        assert_eq!(editor.document().bytes(), carrier);
        assert!(!editor.snapshot().modified);
    }
}

#[test]
fn observed_soft_break_is_a_navigable_line_boundary() {
    let mut carrier = b"SOFT FIRST\x8d\nSOFT SECOND LONG\r\n".to_vec();
    carrier.resize(128, 0x1a);
    let mut editor = Editor::restore(Document::from_bytes(carrier.clone()), 0, true).unwrap();
    editor.apply(&Command::NextLine).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 12);
    editor.apply(&Command::PreviousLine).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 0);
    assert_eq!(editor.document().bytes(), carrier);
}

#[test]
fn unqualified_page_markers_and_opaque_cells_fail_without_moving() {
    for (carrier, cursor, expected) in [
        (
            b"A\r\x8aB\x1a\x1a".to_vec(),
            0,
            EditError::UnqualifiedOperation("page-marked line break"),
        ),
        (
            b"A\r\n\x1bB\x1c\x1a\x1a".to_vec(),
            0,
            EditError::UnsupportedDocument { offset: 3 },
        ),
        (
            b"A\r\n.B\x1a\x1a".to_vec(),
            0,
            EditError::UnqualifiedOperation("dot commands"),
        ),
    ] {
        let mut editor =
            Editor::restore(Document::from_bytes(carrier.clone()), cursor, true).unwrap();
        assert_eq!(editor.apply(&Command::NextLine), Err(expected));
        assert_eq!(editor.snapshot().cursor_byte, cursor);
        assert_eq!(editor.document().bytes(), carrier);
    }
}
