// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Editor, TokenKind};

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
}

impl Projection {
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
