// SPDX-License-Identifier: GPL-3.0-only
//! Bounded, DOS-checked literal search on ordinary ASCII WordStar documents.

use crate::{Document, EditError, TokenKind};

pub(super) struct FindSpec {
    bytes: Vec<u8>,
    ignore_case: bool,
    whole_words: bool,
    pub(super) backwards: bool,
}

impl FindSpec {
    pub(super) fn text(&self) -> String {
        String::from_utf8(self.bytes.clone()).expect("validated ASCII query")
    }
    pub(super) fn new(
        text: &str,
        ignore_case: bool,
        whole_words: bool,
        backwards: bool,
    ) -> Result<Self, EditError> {
        // UI length and work bound, not a claimed DOS limit.
        if text.is_empty() || text.len() > 64 || !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
            return Err(EditError::UnsupportedInput);
        }
        Ok(Self {
            bytes: text.as_bytes().to_vec(),
            ignore_case,
            whole_words,
            backwards,
        })
    }
}

pub(super) fn target(
    document: &Document,
    cursor: usize,
    spec: &FindSpec,
    repeat: bool,
) -> Result<Option<usize>, EditError> {
    let end = document.checked_active_end()?;
    let mut line_start = 0;
    for token in document
        .tokens()
        .take_while(|token| token.range.start < end)
    {
        match token.kind {
            TokenKind::Text {
                high_bit: false, ..
            } => {
                if token.range.start == line_start && document.bytes()[line_start] == b'.' {
                    return Err(EditError::UnqualifiedOperation("dot commands"));
                }
            }
            TokenKind::LineBreak {
                soft: false,
                page_marker: false,
            } => line_start = token.range.end,
            _ => {
                return Err(EditError::UnsupportedDocument {
                    offset: token.range.start,
                });
            }
        }
    }
    let bytes = &document.bytes()[..end];
    if spec.bytes.len() > bytes.len() {
        return Ok(None);
    }
    let word = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'_';
    let matches_at = |start: usize| {
        let finish = start + spec.bytes.len();
        let equal = bytes[start..finish].iter().zip(&spec.bytes).all(|(a, b)| {
            if spec.ignore_case {
                a.eq_ignore_ascii_case(b)
            } else {
                a == b
            }
        });
        equal
            && (!spec.whole_words
                || (start == 0 || !word(bytes[start - 1]))
                    && (finish == end || !word(bytes[finish])))
    };
    let mut candidates = 0..=bytes.len() - spec.bytes.len();
    Ok(if spec.backwards {
        candidates
            .rev()
            .find(|&start| start < cursor && matches_at(start))
    } else {
        candidates
            .find(|&start| start >= cursor.saturating_add(usize::from(repeat)) && matches_at(start))
    })
}
