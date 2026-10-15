// SPDX-License-Identifier: GPL-3.0-only
use crate::session::DocumentSession;
use northstar_core::{Editor, TokenKind, Utf8Editor};
use unicode_normalization::UnicodeNormalization;
use unicode_width::UnicodeWidthChar;

// Leave one visible cell for the insertion cursor and the last cell for the
// existing right-edge indicator. Legacy projection keeps its own DOS layout.
pub const UTF8_TEXT_COLUMNS: usize = 78;

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
    utf8_stops: Vec<Vec<usize>>,
}

struct Utf8Row {
    cells: Vec<Cell>,
    stops: Vec<usize>,
    break_after: Vec<bool>,
}

impl Utf8Row {
    fn new(start: usize) -> Self {
        Self {
            cells: Vec::new(),
            stops: vec![start],
            break_after: Vec::new(),
        }
    }

    fn wrap(&mut self) -> Self {
        let split = self
            .break_after
            .iter()
            .take(UTF8_TEXT_COLUMNS)
            .enumerate()
            .rev()
            .find(|(index, breakable)| {
                **breakable && self.break_after[..*index].iter().any(|value| !value)
            })
            .map_or(UTF8_TEXT_COLUMNS, |(index, _)| index + 1);
        let cells = self.cells.split_off(split);
        let break_after = self.break_after.split_off(split);
        let stops = self.stops.split_off(split);
        self.stops.push(stops[0]);
        self.soft_break();
        Self {
            cells,
            stops,
            break_after,
        }
    }

    fn soft_break(&mut self) {
        self.cells.push(Cell {
            glyph: '↵',
            marker: true,
        });
    }
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
        let mut rows = vec![Utf8Row::new(0)];
        for (start, grapheme) in editor.graphemes() {
            if matches!(grapheme, "\n" | "\r\n" | "\r") {
                rows.last_mut().unwrap().cells.push(Cell {
                    glyph: '¶',
                    marker: true,
                });
                rows.push(Utf8Row::new(start + grapheme.len()));
                continue;
            }
            if matches!(grapheme, " " | "\t")
                && rows.last().unwrap().cells.len() == UTF8_TEXT_COLUMNS
            {
                rows.last_mut().unwrap().soft_break();
                rows.push(Utf8Row::new(start + grapheme.len()));
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
            let row = rows.last_mut().unwrap();
            row.cells.push(Cell {
                glyph,
                marker: glyph == '□',
            });
            row.stops.push(start + grapheme.len());
            row.break_after.push(matches!(grapheme, " " | "\t"));
            if row.cells.len() > UTF8_TEXT_COLUMNS {
                let continuation = row.wrap();
                rows.push(continuation);
            }
        }
        let mut result = Self {
            page_unknown_before_cursor: true,
            ..Self::default()
        };
        for (index, row) in rows.into_iter().enumerate() {
            if let Some(column) = row.stops.iter().position(|offset| *offset == cursor) {
                // A soft-wrap boundary belongs to the continuation row.
                result.cursor = (index, column);
            }
            result.rows.push(row.cells);
            result.utf8_stops.push(row.stops);
        }
        result
    }

    pub fn utf8_vertical_target(&self, direction: isize, preferred_column: usize) -> Option<usize> {
        let row = self.cursor.0.checked_add_signed(direction)?;
        let stops = self.utf8_stops.get(row)?;
        Some(stops[preferred_column.min(stops.len() - 1)])
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
