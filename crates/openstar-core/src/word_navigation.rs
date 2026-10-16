// SPDX-License-Identifier: GPL-3.0-only
//! Bounded Ctrl-A/Ctrl-F movement over measured WordStar 4.00 word cells.
use crate::{AsciiLine, Document, EditError, TokenKind};

fn starts(document: &Document, line: &AsciiLine) -> Result<Vec<usize>, EditError> {
    let mut in_word = false;
    let mut starts = Vec::new();
    for token in document
        .tokens()
        .skip_while(|token| token.range.start < line.start)
        .take_while(|token| token.range.start < line.end)
    {
        let word_cell = match token.kind {
            TokenKind::Text { value, .. } if value.is_ascii_alphanumeric() => true,
            TokenKind::Text {
                value: b'-',
                high_bit: false,
            } if token.range.start > line.start
                && token.range.end < line.end
                && document.bytes()[token.range.start - 1].is_ascii_alphanumeric()
                && document.bytes()[token.range.end].is_ascii_alphanumeric() =>
            {
                true
            }
            TokenKind::Text {
                value: b' ' | b',',
                high_bit: false,
            } => false,
            TokenKind::Text {
                value: b'-' | b',' | b'.',
                high_bit: true,
            } if document.bytes().get(token.range.end) == Some(&b' ') => false,
            TokenKind::Text {
                value: b'.',
                high_bit: false,
            } if document.bytes().get(token.range.end) == Some(&b' ') => false,
            _ => {
                return Err(EditError::UnsupportedDocument {
                    offset: token.range.start,
                });
            }
        };
        if word_cell && !in_word {
            starts.push(token.range.start);
        }
        in_word = word_cell;
    }
    Ok(starts)
}

pub(super) fn target(
    document: &Document,
    cursor: usize,
    forward: bool,
) -> Result<Option<usize>, EditError> {
    let active_end = document.active_end();
    let line = document.ascii_line(cursor, active_end)?;
    let word_starts = starts(document, &line)?;
    if forward && cursor == line.end {
        let Some(next) = line.next else {
            return Ok(None);
        };
        let break_token = document
            .tokens()
            .find(|token| token.range.start == cursor)
            .ok_or(EditError::InvalidCursor { offset: cursor })?;
        if !matches!(
            break_token.kind,
            TokenKind::LineBreak {
                soft: false,
                page_marker: false
            }
        ) {
            return Err(EditError::UnqualifiedOperation(
                "word movement across a soft or page-marked break",
            ));
        }
        starts(document, &document.ascii_line(next, active_end)?)?;
        return Ok(Some(next));
    }
    Ok(if forward {
        word_starts.into_iter().find(|start| *start > cursor)
    } else {
        word_starts
            .into_iter()
            .take_while(|start| *start < cursor)
            .last()
    })
}
