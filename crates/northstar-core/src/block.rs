// SPDX-License-Identifier: GPL-3.0-only
//! Bounded WordStar block marks and copy.
use crate::{Document, Edit, EditError, Formatting, TokenKind};

#[derive(Clone, Copy, Default)]
pub(super) struct Marks {
    pub begin: Option<usize>,
    pub end: Option<usize>,
}

fn check_formatting(formatting: Formatting) -> Result<(), EditError> {
    if (formatting.left_margin, formatting.right_margin) != (1, 65)
        || formatting.line_spacing != 1
        || !formatting.document_mode
        || formatting.right_justify
    {
        return Err(EditError::UnqualifiedOperation(
            "block copy requires document mode, single spacing, margins 1/65, and justification off",
        ));
    }
    Ok(())
}

/// Verify complete physical lines touching a range in one token pass. A
/// selected break must be an ordinary hard CRLF; unrelated lines are retained.
fn check_lines(document: &Document, begin: usize, end: usize) -> Result<(), EditError> {
    let active_end = document.checked_active_end()?;
    let mut line_start = 0;
    let mut unsupported = None;
    let check_line = |start: usize, finish: usize, unsupported: Option<usize>| {
        if finish - start > 65 {
            return Err(EditError::UnqualifiedOperation(
                "block copy on a line beyond the qualified right margin",
            ));
        }
        if document
            .bytes()
            .get(start)
            .is_some_and(|byte| byte & 0x7f == b'.')
        {
            return Err(EditError::UnqualifiedOperation("dot commands"));
        }
        if let Some(offset) = unsupported {
            return Err(EditError::UnsupportedDocument { offset });
        }
        Ok(())
    };
    for token in document
        .tokens()
        .take_while(|token| token.range.start < active_end)
    {
        match token.kind {
            TokenKind::LineBreak { soft, page_marker } => {
                if line_start < end && token.range.end > begin {
                    check_line(line_start, token.range.start, unsupported)?;
                }
                if token.range.start < end && token.range.end > begin && (soft || page_marker) {
                    return Err(EditError::UnqualifiedOperation(
                        "block copy across a soft or page-marked break",
                    ));
                }
                line_start = token.range.end;
                unsupported = None;
            }
            TokenKind::Text {
                high_bit: false, ..
            } => {}
            _ => {
                unsupported.get_or_insert(token.range.start);
            }
        }
    }
    if line_start < end && active_end > begin {
        check_line(line_start, active_end, unsupported)?;
    }
    Ok(())
}

pub(super) fn mark(
    document: &Document,
    cursor: usize,
    formatting: Formatting,
) -> Result<(), EditError> {
    check_formatting(formatting)?;
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    // Include this line even for a mark at its end or the document end.
    check_lines(document, line.start, line.end.max(line.start + 1).min(end))
}

pub(super) fn copy(
    document: &Document,
    cursor: usize,
    formatting: Formatting,
    marks: Marks,
) -> Result<(Edit, Marks), EditError> {
    check_formatting(formatting)?;
    let active_end = document.checked_active_end()?;
    let (Some(begin), Some(end)) = (marks.begin, marks.end) else {
        return Err(EditError::UnqualifiedOperation("block is not defined"));
    };
    if end <= begin {
        return Err(EditError::UnqualifiedOperation(
            "block end is at or before block start",
        ));
    }
    if end > active_end {
        return Err(EditError::InvalidCursor { offset: end });
    }
    check_lines(document, begin, end)?;
    let copied = &document.bytes()[begin..end];
    let new_length =
        active_end
            .checked_add(copied.len())
            .ok_or(EditError::UnqualifiedOperation(
                "block copy exceeds document capacity",
            ))?;
    if new_length > 1024 * 1024 {
        return Err(EditError::UnqualifiedOperation(
            "block copy exceeds the 1 MiB prototype limit",
        ));
    }
    let mut active = Vec::with_capacity(new_length);
    active.extend_from_slice(&document.bytes()[..cursor]);
    active.extend_from_slice(copied);
    active.extend_from_slice(&document.bytes()[cursor..active_end]);
    let staged = Document::from_bytes(active.clone());
    let destination_line = document.ascii_line(cursor, active_end)?;
    // Include the destination suffix line when the copied span ends in CRLF.
    check_lines(&staged, destination_line.start, cursor + copied.len() + 1)?;
    let shift = |mark: usize, inclusive: bool| {
        if cursor < mark || (inclusive && cursor == mark) {
            mark + copied.len()
        } else {
            mark
        }
    };
    Ok((
        Edit::Replace { active, cursor },
        Marks {
            begin: Some(shift(begin, true)),
            end: Some(shift(end, false)),
        },
    ))
}
