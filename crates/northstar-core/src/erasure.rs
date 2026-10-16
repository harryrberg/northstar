// SPDX-License-Identifier: GPL-3.0-only
//! Bounded WordStar Ctrl-G, Ctrl-Y, and Ctrl-U semantics.
use crate::{AsciiLine, Document, Edit, EditError, Formatting, TokenKind};

fn check_formatting(formatting: Formatting) -> Result<(), EditError> {
    if (formatting.left_margin, formatting.right_margin) != (1, 65)
        || formatting.line_spacing != 1
        || !formatting.document_mode
        || formatting.right_justify
    {
        return Err(EditError::UnqualifiedOperation(
            "erasure requires document mode, single spacing, margins 1/65, and justification off",
        ));
    }
    Ok(())
}

fn check_line(
    document: &Document,
    line: &AsciiLine,
    allow_high_bit: bool,
) -> Result<(), EditError> {
    if line.end - line.start > 65 {
        return Err(EditError::UnqualifiedOperation(
            "erasure on a line beyond the qualified right margin",
        ));
    }
    if document.tokens().any(|token| {
        token.range.start >= line.start
            && token.range.start < line.end
            && (matches!(token.kind, TokenKind::SoftSpace)
                || (!allow_high_bit
                    && matches!(token.kind, TokenKind::Text { high_bit: true, .. })))
    }) {
        return Err(EditError::UnqualifiedOperation(
            "erasure on a line with unqualified markers",
        ));
    }
    Ok(())
}

fn check_previous_break(document: &Document, start: usize) -> Result<(), EditError> {
    if start == 0 {
        return Ok(());
    }
    let previous = document
        .tokens()
        .find(|token| token.range.end == start)
        .ok_or(EditError::InvalidCursor { offset: start })?;
    if !matches!(
        previous.kind,
        TokenKind::LineBreak {
            soft: false,
            page_marker: false
        }
    ) {
        return Err(EditError::UnqualifiedOperation(
            "line erasure after a soft or page-marked break",
        ));
    }
    Ok(())
}

pub(super) fn delete_character(
    document: &Document,
    cursor: usize,
    formatting: Formatting,
) -> Result<Edit, EditError> {
    check_formatting(formatting)?;
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    check_line(document, &line, true)?;
    if cursor == end {
        return Ok(Edit::MoveTo(cursor));
    }
    let remove = if cursor < line.end {
        cursor..cursor + 1
    } else {
        let next = line.next.ok_or(EditError::UnqualifiedOperation(
            "delete at an unresolved line boundary",
        ))?;
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
                "deleting a soft or page-marked line break",
            ));
        }
        let following = document.ascii_line(next, end)?;
        check_line(document, &following, true)?;
        if line.end - line.start + following.end - following.start > 65 {
            return Err(EditError::UnqualifiedOperation(
                "joining lines beyond the qualified right margin",
            ));
        }
        cursor..next
    };
    let mut active = document.bytes()[..end].to_vec();
    active.drain(remove);
    Ok(Edit::Replace { active, cursor })
}

/// Ctrl-Y supplies a reusable line buffer; an empty final line leaves it alone.
pub(super) fn delete_line(
    document: &Document,
    cursor: usize,
    formatting: Formatting,
) -> Result<Option<(Edit, Vec<u8>)>, EditError> {
    check_formatting(formatting)?;
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    check_line(document, &line, false)?;
    check_previous_break(document, line.start)?;
    let stop = if let Some(next) = line.next {
        let following = document.ascii_line(next, end)?;
        check_line(document, &following, false)?;
        let break_token = document
            .tokens()
            .find(|token| token.range.start == line.end)
            .ok_or(EditError::InvalidCursor { offset: line.end })?;
        if !matches!(
            break_token.kind,
            TokenKind::LineBreak {
                soft: false,
                page_marker: false
            }
        ) {
            return Err(EditError::UnqualifiedOperation(
                "deleting a line with a soft or page-marked break",
            ));
        }
        next
    } else {
        line.end
    };
    if stop == line.start {
        return Ok(None);
    }
    let mut active = document.bytes()[..end].to_vec();
    let erased = active.drain(line.start..stop).collect();
    Ok(Some((
        Edit::Replace {
            active,
            cursor: line.start,
        },
        erased,
    )))
}

/// Ctrl-U inserts the retained Ctrl-Y buffer without consuming it.
pub(super) fn unerase_line(
    document: &Document,
    cursor: usize,
    formatting: Formatting,
    erased: Option<&[u8]>,
) -> Result<Edit, EditError> {
    let Some(erased) = erased else {
        return Ok(Edit::MoveTo(cursor));
    };
    check_formatting(formatting)?;
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    check_line(document, &line, false)?;
    check_previous_break(document, line.start)?;
    if cursor == line.end && line.next.is_some() {
        return Err(EditError::UnqualifiedOperation("unerase at a line break"));
    }
    let text_len = if erased.ends_with(b"\r\n") {
        erased.len() - 2
    } else {
        erased.len()
    };
    if cursor - line.start + text_len > 65
        || (!erased.ends_with(b"\r\n") && line.end - line.start + text_len > 65)
    {
        return Err(EditError::UnqualifiedOperation(
            "unerase beyond the qualified right margin",
        ));
    }
    if erased.ends_with(b"\r\n")
        && document
            .bytes()
            .get(cursor)
            .is_some_and(|byte| byte & 0x7f == b'.')
    {
        return Err(EditError::UnqualifiedOperation(
            "unerase creating a dot-command line",
        ));
    }
    let mut active = document.bytes()[..end].to_vec();
    active.splice(cursor..cursor, erased.iter().copied());
    Ok(Edit::Replace { active, cursor })
}
