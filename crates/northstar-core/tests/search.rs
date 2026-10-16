// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor, Formatting};

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

fn replace(text: &str, replacement: &str) -> Command {
    Command::ReplaceLiteral {
        text: text.into(),
        replacement: replacement.into(),
        ignore_case: false,
        whole_words: false,
        without_asking: false,
    }
}

fn replace_editor(bytes: &[u8]) -> Editor {
    let mut carrier = bytes.to_vec();
    carrier.resize(128, 0x1a);
    Editor::restore_with_formatting(
        Document::from_bytes(carrier),
        0,
        true,
        Formatting {
            right_justify: false,
            ..Formatting::default()
        },
    )
    .unwrap()
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

#[test]
fn replacement_question_blocks_other_edits_and_decline_preserves_bytes() {
    let mut editor = replace_editor(b"ALPHA BETA ALPHA");
    let original = editor.document().bytes().to_vec();
    editor.apply(&replace("ALPHA", "OMEGA")).unwrap();
    assert!(editor.replacement_pending());
    let before = editor.snapshot();
    assert!(
        editor
            .apply(&Command::InsertAscii { text: "Z".into() })
            .is_err()
    );
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), original);
    editor
        .apply(&Command::AnswerReplace { accept: false })
        .unwrap();
    assert!(!editor.replacement_pending());
    assert_eq!(editor.document().bytes(), original);
    editor.apply(&Command::RepeatFind).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 11);
    assert!(editor.replacement_pending());
}

#[test]
fn replacement_rejects_unqualified_words_and_overlong_output_atomically() {
    let mut editor = replace_editor(b"ALPHA");
    let before = editor.snapshot();
    let bytes = editor.document().bytes().to_vec();
    assert!(editor.apply(&replace("ALPHA", "TWO WORDS")).is_err());
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), bytes);

    let mut editor = replace_editor(&[b'A'; 65]);
    let before = editor.snapshot();
    let bytes = editor.document().bytes().to_vec();
    assert!(editor.apply(&replace("A", "LONG")).is_err());
    assert_eq!(editor.snapshot(), before);
    assert_eq!(editor.document().bytes(), bytes);
}
