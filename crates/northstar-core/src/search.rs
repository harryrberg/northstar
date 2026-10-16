// SPDX-License-Identifier: GPL-3.0-only
//! Bounded, DOS-checked literal search on ordinary ASCII WordStar documents.

use crate::{Document, Edit, EditError, Formatting, TokenKind};

#[derive(Clone)]
pub(super) struct FindSpec {
    bytes: Vec<u8>,
    ignore_case: bool,
    whole_words: bool,
    pub(super) backwards: bool,
}

#[derive(Clone)]
pub(super) struct ReplaceSpec {
    pub(super) bytes: Vec<u8>,
    pub(super) without_asking: bool,
}

impl ReplaceSpec {
    pub(super) fn new(
        find: &str,
        replacement: &str,
        without_asking: bool,
    ) -> Result<Self, EditError> {
        // The retained replacement cases use nonempty alphabetic words. Spaces,
        // punctuation, and empty replacements need their own byte comparisons.
        if !find.bytes().all(|byte| byte.is_ascii_alphabetic())
            || replacement.is_empty()
            || replacement.len() > 64
            || !replacement.bytes().all(|byte| byte.is_ascii_alphabetic())
        {
            return Err(EditError::UnqualifiedOperation(
                "replacement currently requires nonempty ASCII words",
            ));
        }
        Ok(Self {
            bytes: replacement.as_bytes().to_vec(),
            without_asking,
        })
    }
}

pub(super) fn check_replace_context(formatting: Formatting, insert: bool) -> Result<(), EditError> {
    if !insert
        || !formatting.document_mode
        || formatting.line_spacing != 1
        || (formatting.left_margin, formatting.right_margin) != (1, 65)
        || formatting.right_justify
    {
        return Err(EditError::UnqualifiedOperation(
            "replacement requires Insert on, document mode, single spacing, margins 1/65, and justification off",
        ));
    }
    Ok(())
}

pub(super) fn replacement_edit(
    document: &Document,
    cursor: usize,
    find_len: usize,
    replacement: &ReplaceSpec,
) -> Result<Edit, EditError> {
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    if cursor + find_len > line.end
        || line.end - line.start - find_len + replacement.bytes.len() > 65
    {
        return Err(EditError::UnqualifiedOperation(
            "replacement must fit an ordinary physical line within margin 65",
        ));
    }
    let new_length = end - find_len + replacement.bytes.len();
    if new_length > 1024 * 1024 {
        return Err(EditError::UnqualifiedOperation(
            "replacement exceeds the 1 MiB prototype limit",
        ));
    }
    let mut active = Vec::with_capacity(new_length);
    active.extend_from_slice(&document.bytes()[..cursor]);
    active.extend_from_slice(&replacement.bytes);
    active.extend_from_slice(&document.bytes()[cursor + find_len..end]);
    Ok(Edit::Replace { active, cursor })
}

impl FindSpec {
    pub(super) fn text(&self) -> String {
        String::from_utf8(self.bytes.clone()).expect("validated ASCII query")
    }
    pub(super) fn len(&self) -> usize {
        self.bytes.len()
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
