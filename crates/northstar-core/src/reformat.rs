// SPDX-License-Identifier: GPL-3.0-only
//! Bounded reconstruction of Ctrl-B. See docs/CTRL_B.md for evidence and limits.
use crate::{Document, Edit, EditError, TokenKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Formatting {
    pub left_margin: u16,
    pub right_margin: u16,
    pub right_justify: bool,
    pub line_spacing: u16,
    pub document_mode: bool,
}

impl Default for Formatting {
    fn default() -> Self {
        Self {
            left_margin: 1,
            right_margin: 65,
            right_justify: true,
            line_spacing: 1,
            document_mode: true,
        }
    }
}

pub(super) fn paragraph(
    document: &Document,
    cursor: usize,
    formatting: Formatting,
) -> Result<Edit, EditError> {
    if formatting
        != (Formatting {
            right_justify: false,
            ..Formatting::default()
        })
    {
        return Err(EditError::UnqualifiedOperation(
            "Ctrl-B requires document mode, single spacing, margins 1/65, and right justification off",
        ));
    }
    let active_end = document.checked_active_end()?;
    let mut is_start = cursor == 0;
    let mut end = active_end;
    let mut next_paragraph = active_end;
    for token in document.tokens() {
        if token.kind
            == (TokenKind::LineBreak {
                soft: false,
                page_marker: false,
            })
        {
            is_start |= token.range.end == cursor;
            if token.range.start >= cursor && end == active_end {
                end = token.range.start;
                next_paragraph = token.range.end;
            }
        }
    }
    if !is_start {
        return Err(EditError::UnqualifiedOperation(
            "Ctrl-B away from a hard paragraph start",
        ));
    }

    // Validate and normalize through the document's token owner. Framing payloads
    // must never be masked as if they were ordinary high-bit ASCII text.
    let mut clean = Vec::with_capacity(end - cursor);
    let mut removed = false;
    for token in document
        .tokens()
        .skip_while(|t| t.range.start < cursor)
        .take_while(|t| t.range.start < end)
    {
        match token.kind {
            TokenKind::Text { value, .. } => {
                if value == b'-' {
                    return Err(EditError::UnqualifiedOperation("hyphen break rules"));
                }
                // EF90 inserts a separator when removed markers joined two words.
                if removed && value != b' ' && clean.last().is_some_and(|b| *b != b' ') {
                    clean.push(b' ');
                }
                clean.push(value);
                removed = false;
            }
            TokenKind::SoftSpace
            | TokenKind::LineBreak {
                soft: true,
                page_marker: false,
            } => {
                removed = true;
            }
            _ => {
                return Err(EditError::UnsupportedDocument {
                    offset: token.range.start,
                });
            }
        }
    }
    if removed && clean.last().is_some_and(|b| *b != b' ') {
        // EF90 also inserts a needed separator before a hard boundary or EOF.
        clean.push(b' ');
    }
    if clean.first() == Some(&b'.') {
        return Err(EditError::UnqualifiedOperation("dot commands"));
    }

    // Keep every ordinary space, including indentation and trailing spaces.
    // WordStar tests the last nonspace column, not the following separator.
    let mut formatted = Vec::with_capacity(clean.len());
    let mut line_start = 0;
    let mut i = 0;
    let mut has_word = false;
    while i < clean.len() {
        if clean[i] == b' ' {
            formatted.push(b' ');
            i += 1;
            continue;
        }
        let word_start = i;
        while i < clean.len() && clean[i] != b' ' {
            i += 1;
        }
        if i - word_start > 65 {
            return Err(EditError::UnqualifiedOperation(
                "words longer than the right margin",
            ));
        }
        if i - line_start > 65 {
            if !has_word {
                return Err(EditError::UnqualifiedOperation(
                    "indentation extending a word past the right margin",
                ));
            }
            formatted.extend_from_slice(b"\x8d\n");
            line_start = word_start;
        }
        formatted.extend_from_slice(&clean[word_start..i]);
        // 90BA/90CF marks the character before a space, even on the final line.
        if i < clean.len() {
            *formatted.last_mut().unwrap() |= 0x80;
        }
        has_word = true;
    }
    let new_cursor = cursor + formatted.len() + (next_paragraph - end);
    let mut active = document.bytes()[..cursor].to_vec();
    active.extend_from_slice(&formatted);
    active.extend_from_slice(&document.bytes()[end..active_end]);
    Ok(Edit::Replace {
        active,
        cursor: new_cursor,
    })
}
