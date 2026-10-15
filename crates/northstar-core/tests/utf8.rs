// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{UTF8_HISTORY_EDITS, Utf8Editor, Utf8Error};

#[test]
fn cursor_and_deletion_follow_graphemes_without_rewriting_utf8() {
    let mut editor = Utf8Editor::new(String::new());
    editor.insert_text("æøå e\u{301} 👩‍💻").unwrap();
    assert_eq!(editor.text().as_bytes(), "æøå e\u{301} 👩‍💻".as_bytes());
    assert!(editor.snapshot().modified);
    let end = editor.snapshot().cursor_byte;
    assert_eq!(end, editor.text().len());

    editor.previous();
    let emoji_start = editor.snapshot().cursor_byte;
    assert_eq!(&editor.text()[emoji_start..], "👩‍💻");
    assert_eq!(
        editor.set_cursor(emoji_start + 4),
        Err(Utf8Error::InvalidCursor)
    );
    editor.delete();
    assert_eq!(editor.text(), "æøå e\u{301} ");
    editor.backspace();
    assert_eq!(editor.text(), "æøå e\u{301}");
    editor.backspace();
    assert_eq!(editor.text(), "æøå ");
    editor.mark_saved();
    assert!(!editor.snapshot().modified);
    editor.insert_text("e\u{301}").unwrap();
    assert_eq!(editor.text(), "æøå e\u{301}");
    assert_eq!(editor.snapshot().cursor_byte, editor.text().len());
}

#[test]
fn input_is_atomic_and_line_endings_are_preserved() {
    let mut editor = Utf8Editor::new("A\r\nB".into());
    editor.set_cursor(1).unwrap();
    editor.next();
    assert_eq!(editor.snapshot().cursor_byte, 3);
    assert_eq!(editor.insert_text("x\0y"), Err(Utf8Error::ControlInput));
    assert_eq!(editor.text(), "A\r\nB");
    assert!(!editor.snapshot().modified);
    editor.enter();
    assert_eq!(editor.text(), "A\r\n\nB");
    editor.previous();
    editor.backspace();
    assert_eq!(editor.text(), "A\nB");
}

#[test]
fn undo_redo_restore_exact_text_cursor_and_saved_checkpoint() {
    let original = "Blåbær\r\nCafe\u{301} 👩‍💻";
    let mut editor = Utf8Editor::new(original.into());
    editor.set_cursor("Blåbær\r\n".len()).unwrap();
    let saved_cursor = editor.snapshot().cursor_byte;
    editor.insert_text("æøå").unwrap();
    let inserted_cursor = editor.snapshot().cursor_byte;
    editor.mark_saved();
    editor.backspace();
    assert_eq!(editor.text(), "Blåbær\r\næøCafe\u{301} 👩‍💻");
    assert!(editor.snapshot().modified);
    assert!(editor.undo());
    assert_eq!(editor.snapshot().cursor_byte, inserted_cursor);
    assert!(!editor.snapshot().modified);
    assert_eq!(editor.text(), "Blåbær\r\næøåCafe\u{301} 👩‍💻");
    assert!(editor.undo());
    assert_eq!(editor.snapshot().cursor_byte, saved_cursor);
    assert_eq!(editor.text(), original);
    assert!(editor.snapshot().modified);
    assert!(editor.redo());
    assert!(!editor.snapshot().modified);
    assert!(editor.redo());
    assert!(editor.snapshot().modified);
    assert!(!editor.redo());
}

#[test]
fn failed_or_empty_edits_preserve_redo_and_new_edit_discards_it() {
    let mut editor = Utf8Editor::new("e\u{301} 👩‍💻".into());
    editor.set_cursor(editor.text().len()).unwrap();
    editor.backspace();
    assert_eq!(editor.text(), "e\u{301} ");
    assert!(editor.undo());
    let at_end = editor.snapshot().cursor_byte;
    assert_eq!(editor.insert_text("\0"), Err(Utf8Error::ControlInput));
    editor.insert_text("").unwrap();
    editor.document_start();
    editor.backspace();
    editor.set_cursor(at_end).unwrap();
    assert!(editor.redo());
    assert_eq!(editor.text(), "e\u{301} ");
    assert!(editor.undo());
    editor.insert_text("å").unwrap();
    assert_eq!(editor.text(), "e\u{301} 👩‍💻å");
    assert!(!editor.redo());
    assert!(editor.undo());
    assert_eq!(editor.text(), "e\u{301} 👩‍💻");
}

#[test]
fn history_evicts_old_changes_when_changed_text_exceeds_budget() {
    let mut editor = Utf8Editor::new(String::new());
    let chunk = "x".repeat(768 * 1024);
    for _ in 0..3 {
        editor.insert_text(&chunk).unwrap();
    }
    assert!(editor.undo());
    assert!(editor.undo());
    assert!(!editor.undo());
    assert_eq!(editor.text(), chunk);
    assert!(editor.redo());
    assert!(editor.redo());
    assert_eq!(editor.text().len(), chunk.len() * 3);
}

#[test]
fn history_evicts_old_changes_when_edit_count_exceeds_limit() {
    let mut editor = Utf8Editor::new(String::new());
    for _ in 0..=UTF8_HISTORY_EDITS {
        editor.insert_text("x").unwrap();
    }
    for _ in 0..UTF8_HISTORY_EDITS {
        assert!(editor.undo());
    }
    assert!(!editor.undo());
    assert_eq!(editor.text(), "x");
}
