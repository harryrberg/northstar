// SPDX-License-Identifier: GPL-3.0-only
use crate::session::DocumentSession;
use northstar_core::{Editor, TokenKind, Utf8Editor};
use unicode_normalization::UnicodeNormalization;
use unicode_width::UnicodeWidthChar;

#[derive(Debug, Clone)]
pub struct Cell {
    pub glyph: char,
    pub marker: bool,
}

#[derive(Debug, Default)]
pub struct Projection {
    pub rows: Vec<Vec<Cell>>,
    pub cursor: (usize, usize),
    pub hidden_tail_bytes: usize,
    pub page_unknown_before_cursor: bool,
}

impl Projection {
    pub fn for_session(session: &DocumentSession) -> Self {
        match session {
            DocumentSession::Legacy(session) => Self::new(&session.editor),
            DocumentSession::Utf8(session) => Self::utf8(&session.editor),
        }
    }

    pub fn utf8(editor: &Utf8Editor) -> Self {
        let cursor = editor.snapshot().cursor_byte;
        let mut result = Self {
            rows: vec![vec![]],
            page_unknown_before_cursor: true,
            ..Self::default()
        };
        for (start, grapheme) in editor.graphemes() {
            if start == cursor {
                result.cursor = (result.rows.len() - 1, result.rows.last().unwrap().len());
            }
            if matches!(grapheme, "\n" | "\r\n") {
                result.rows.last_mut().unwrap().push(Cell {
                    glyph: '¶',
                    marker: true,
                });
                result.rows.push(vec![]);
                continue;
            }
            // The fixed DOS grid can display one-cell glyphs. Keep every other
            // grapheme intact in the editor, but show a visible placeholder.
            let composed = grapheme.nfc().collect::<String>();
            let glyph = if let Some(ch) = composed.chars().next()
                && composed.chars().count() == 1
                && !ch.is_control()
                && UnicodeWidthChar::width(ch) == Some(1)
            {
                ch
            } else {
                '□'
            };
            result.rows.last_mut().unwrap().push(Cell {
                glyph,
                marker: glyph == '□',
            });
        }
        if cursor == editor.text().len() {
            result.cursor = (result.rows.len() - 1, result.rows.last().unwrap().len());
        }
        result
    }

    /// A structural view, not reconstructed DOS pagination or glyph layout.
    /// Each non-break token gets one cell, including explicit placeholders.
    pub fn new(editor: &Editor) -> Self {
        let snapshot = editor.snapshot();
        let mut result = Self {
            rows: vec![vec![]],
            ..Self::default()
        };
        for token in editor.document().tokens() {
            if token.range.start == snapshot.cursor_byte {
                result.cursor = (result.rows.len() - 1, result.rows.last().unwrap().len());
            }
            if token.range.start >= snapshot.active_bytes {
                break;
            }
            if token.range.start < snapshot.cursor_byte
                && !matches!(
                    token.kind,
                    TokenKind::Text { .. }
                        | TokenKind::SoftSpace
                        | TokenKind::LineBreak {
                            page_marker: false,
                            ..
                        }
                )
            {
                result.page_unknown_before_cursor = true;
            }
            let row = result.rows.last_mut().unwrap();
            match token.kind {
                TokenKind::Text { value, .. } => row.push(Cell {
                    glyph: value as char,
                    marker: false,
                }),
                TokenKind::SoftSpace => row.push(Cell {
                    glyph: ' ',
                    marker: false,
                }),
                TokenKind::LineBreak { soft, page_marker } => {
                    row.push(Cell {
                        glyph: if page_marker {
                            '!'
                        } else if soft {
                            '↵'
                        } else {
                            '¶'
                        },
                        marker: true,
                    });
                    result.rows.push(vec![]);
                }
                _ => row.push(Cell {
                    glyph: '□',
                    marker: true,
                }),
            }
        }
        if snapshot.cursor_byte == snapshot.active_bytes {
            result.cursor = (result.rows.len() - 1, result.rows.last().unwrap().len());
        }
        // Padding remains in the carrier. A payload following the first EOF is
        // separately disclosed instead of silently presenting it as ordinary text.
        if let Some(padding) = editor
            .document()
            .tokens()
            .find(|t| t.kind == TokenKind::PaddingCandidate)
        {
            result.hidden_tail_bytes = editor.document().bytes().len() - padding.range.end;
        }
        result
    }
}
