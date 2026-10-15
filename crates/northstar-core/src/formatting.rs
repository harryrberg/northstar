// SPDX-License-Identifier: GPL-3.0-only
//! Declared formatting state and shared EEF7/EF90 normalization semantics.
use crate::{Document, EditError, TokenKind};
use serde::{Deserialize, Serialize};
use std::ops::Range;

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

/// Normalize a validated token-aligned range, preserving the byte at its end.
/// Ctrl-B selects a paragraph; typing selects a physical-line prefix. EF90
/// tests the raw next byte when a removed run reaches that exclusive boundary.
pub(super) fn normalize(document: &Document, range: Range<usize>) -> Result<Vec<u8>, EditError> {
    let mut clean = Vec::with_capacity(range.len());
    let mut removed = false;
    for token in document
        .tokens()
        .skip_while(|t| t.range.start < range.start)
        .take_while(|t| t.range.start < range.end)
    {
        match token.kind {
            TokenKind::Text { value, .. } => {
                if value == b'-' {
                    return Err(EditError::UnqualifiedOperation("hyphen break rules"));
                }
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
            } => removed = true,
            _ => {
                return Err(EditError::UnsupportedDocument {
                    offset: token.range.start,
                });
            }
        }
    }
    if removed
        && !matches!(document.bytes().get(range.end), Some(b' ' | b'\t'))
        && clean.last().is_some_and(|b| *b != b' ')
    {
        clean.push(b' ');
    }
    if clean.first() == Some(&b'.') {
        return Err(EditError::UnqualifiedOperation("dot commands"));
    }
    Ok(clean)
}
