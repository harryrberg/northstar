// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor};

fn editor(bytes: &[u8]) -> Editor {
    let mut carrier = bytes.to_vec();
    carrier.resize(128, 0x1a);
    Editor::restore(Document::from_bytes(carrier), 0, true).unwrap()
}

fn find(text: &str) -> Command {
    Command::FindLiteral {
        text: text.into(),
        ignore_case: false,
        whole_words: false,
        backwards: false,
    }
}

#[test]
fn failed_search_preserves_carrier_and_insert_mode_and_repeat_starts_after_cursor() {
    let mut editor = editor(b"BANANA CAT\r\nCAT");
    let carrier = editor.document().bytes().to_vec();
    editor.apply(&find("ANA")).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 1);
    editor.apply(&Command::RepeatFind).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 3);
    editor.apply(&Command::RepeatFind).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 15);
    assert_eq!(editor.last_find_result(), Some(false));
    assert_eq!(editor.document().bytes(), carrier);
    assert!(!editor.snapshot().modified);
    assert!(editor.snapshot().insert);
}

#[test]
fn unqualified_document_and_invalid_query_reject_without_changing_search_state() {
    let mut clean = editor(b"ALPHA\r\nBETA");
    assert!(clean.apply(&Command::RepeatFind).is_err());
    assert!(clean.apply(&find("")).is_err());
    assert_eq!(clean.last_find_result(), None);
    assert_eq!(clean.snapshot().cursor_byte, 0);
    let mut marked = editor(b"ALPHA\xA0BETA");
    assert!(marked.apply(&find("BETA")).is_err());
    assert_eq!(marked.snapshot().cursor_byte, 0);
    assert_eq!(marked.last_find_result(), None);
}
