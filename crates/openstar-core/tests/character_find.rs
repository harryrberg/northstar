// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, Editor};

fn editor(active: &[u8], cursor: usize) -> Editor {
    let mut carrier = active.to_vec();
    carrier.resize(128, 0x1a);
    Editor::restore(Document::from_bytes(carrier), cursor, true).unwrap()
}

fn search(editor: &mut Editor, character: char, backwards: bool) {
    editor
        .apply(&Command::FindCharacter {
            character,
            backwards,
        })
        .unwrap();
}

#[test]
fn forward_search_includes_current_cell_and_backward_search_excludes_it() {
    let mut e = editor(b"ABACABA", 1);
    let original = e.document().bytes().to_vec();
    search(&mut e, 'A', false);
    assert_eq!(e.snapshot().cursor_byte, 2);
    search(&mut e, 'A', false);
    assert_eq!(e.snapshot().cursor_byte, 2);
    search(&mut e, 'A', true);
    assert_eq!(e.snapshot().cursor_byte, 0);
    assert_eq!(e.document().bytes(), original);
    assert!(!e.snapshot().modified);

    let mut lines = editor(b"AB\r\nCD", 1);
    search(&mut lines, 'C', false);
    assert_eq!(lines.snapshot().cursor_byte, 4);
    search(&mut lines, 'A', true);
    assert_eq!(lines.snapshot().cursor_byte, 0);
}

#[test]
fn character_search_respects_case_and_failure_boundary() {
    let mut e = editor(b"aBaA", 0);
    search(&mut e, 'A', false);
    assert_eq!(e.snapshot().cursor_byte, 3);
    search(&mut e, 'a', true);
    assert_eq!(e.snapshot().cursor_byte, 2);
    search(&mut e, 'Z', false);
    assert_eq!(e.snapshot().cursor_byte, 4);
    assert_eq!(e.last_find_result(), Some(false));
    search(&mut e, 'Z', true);
    assert_eq!(e.snapshot().cursor_byte, 0);
}

#[test]
fn unqualified_character_and_document_leave_state_unchanged() {
    let mut e = editor(b"ABACABA", 1);
    let before = e.snapshot();
    assert!(
        e.apply(&Command::FindCharacter {
            character: '!',
            backwards: false
        })
        .is_err()
    );
    assert_eq!(e.snapshot(), before);
    let mut marked = editor(b"A\xA0B", 0);
    let before = marked.snapshot();
    assert!(
        marked
            .apply(&Command::FindCharacter {
                character: 'B',
                backwards: false
            })
            .is_err()
    );
    assert_eq!(marked.snapshot(), before);
}
