// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Utf8Editor, Utf8Error};

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
