// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::{Command, Document, Editor, TokenKind};

fn assert_partition(bytes: Vec<u8>) {
    let doc = Document::from_bytes(bytes.clone());
    let mut end = 0;
    let mut reconstructed = Vec::new();
    for token in doc.tokens() {
        assert_eq!(token.range.start, end);
        assert!(!token.range.is_empty());
        end = token.range.end;
        reconstructed.extend_from_slice(&doc.bytes()[token.range]);
    }
    assert_eq!(end, bytes.len());
    assert_eq!(reconstructed, bytes);
    assert_eq!(doc.bytes(), bytes);
}

#[test]
fn every_byte_and_all_two_byte_inputs_partition_losslessly() {
    assert_partition(Vec::new());
    for a in 0..=255u8 {
        assert_partition(vec![a]);
        for b in 0..=255u8 {
            assert_partition(vec![a, b]);
        }
    }
    assert_partition((0..=255).cycle().take(4096).collect());
}

#[test]
fn framing_precedes_eof_and_high_bit_interpretation() {
    for prefix in [0x1b, 0x9b] {
        for code in 0..=255u8 {
            let bytes = vec![prefix, code, 0x1c];
            let doc = Document::from_bytes(bytes.clone());
            assert_eq!(doc.tokens().count(), 1);
            assert!(
                matches!(doc.tokens().next().unwrap().kind, TokenKind::FramedCharacter {code: c, ..} if c == code)
            );
            assert_eq!(doc.active_end(), 3);
            assert_partition(bytes);
        }
    }
}

#[test]
fn malformed_frames_and_post_eof_payload_are_retained() {
    for bytes in [vec![0x1b], vec![0x9b, 0xc4], vec![0x9b, 0xc4, b'X']] {
        let doc = Document::from_bytes(bytes.clone());
        assert_eq!(
            doc.tokens().next().unwrap().kind,
            TokenKind::UnresolvedFrame
        );
        assert_partition(bytes);
    }
    let doc = Document::from_bytes(b"A\x1a\x1aTAIL".to_vec());
    assert_eq!(doc.active_end(), 1);
    assert_eq!(doc.tokens().last().unwrap().range, 6..7);
    let mut editor = Editor::restore(doc.clone(), 0, true).unwrap();
    assert!(editor.apply(&Command::Enter).is_err());
    assert_eq!(editor.document(), &doc);
}

#[test]
fn break_variants_preserve_soft_and_page_identity() {
    for cr in [0x0d, 0x8d] {
        for lf in [0x0a, 0x8a] {
            let doc = Document::from_bytes(vec![cr, lf]);
            assert_eq!(
                doc.tokens().next().unwrap().kind,
                TokenKind::LineBreak {
                    soft: cr == 0x8d,
                    page_marker: lf == 0x8a
                }
            );
        }
    }
    assert_eq!(
        Document::from_bytes(vec![0x0d])
            .tokens()
            .next()
            .unwrap()
            .kind,
        TokenKind::UnpairedCarriageReturn
    );
}

#[test]
fn rejected_commands_do_not_change_bytes_cursor_or_mode() {
    let mut editor = Editor::restore(Document::from_bytes(b"ALPHA".to_vec()), 2, true).unwrap();
    let before = editor.snapshot();
    for text in ["\n", "æ", &"X".repeat(65)] {
        assert!(
            editor
                .apply(&Command::InsertAscii { text: text.into() })
                .is_err()
        );
        assert_eq!(editor.document().bytes(), b"ALPHA");
        assert_eq!(editor.snapshot().cursor_byte, before.cursor_byte);
        assert_eq!(editor.snapshot().insert, before.insert);
    }
}

#[test]
fn cursor_cannot_split_a_structural_token_or_enter_padding() {
    for cursor in [2, 5] {
        assert!(
            Editor::restore(
                Document::from_bytes(b"A\r\nB\x1a\x1a".to_vec()),
                cursor,
                true
            )
            .is_err()
        );
    }
    let mut editor = Editor::restore(Document::from_bytes(b"A\r\nB".to_vec()), 1, true).unwrap();
    editor.apply(&Command::NextCharacter).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 3);
    editor.apply(&Command::PreviousCharacter).unwrap();
    assert_eq!(editor.snapshot().cursor_byte, 1);
}
