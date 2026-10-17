// SPDX-License-Identifier: GPL-3.0-only
//! ASCII typing, bounded automatic wrapping, Enter, and Ctrl-N.
use crate::{Document, Edit, EditError, Formatting, formatting};
use std::ops::Range;

pub(super) fn check_formatting(formatting: Formatting) -> Result<(), EditError> {
    if (formatting.left_margin, formatting.right_margin) != (1, 65)
        || formatting.line_spacing != 1
        || !formatting.document_mode
    {
        return Err(EditError::UnqualifiedOperation(
            "ASCII editing requires document mode, single spacing, and margins 1/65",
        ));
    }
    Ok(())
}

pub(super) fn type_ascii(
    document: &Document,
    mut cursor: usize,
    insert: bool,
    text: &str,
    formatting: Formatting,
) -> Result<Edit, EditError> {
    check_formatting(formatting)?;
    if !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(EditError::UnsupportedInput);
    }
    let end = document.checked_active_end()?;
    let selected = document.ascii_line(cursor, end)?;
    let mut line = selected.start..selected.end;
    if text.is_empty() {
        return Ok(Edit::MoveTo(cursor));
    }
    // Stage the entire batch once. Wrapping is evaluated before each character,
    // but a later unsupported branch must not commit an earlier typed prefix.
    let mut staged = Document::from_bytes(document.bytes()[..end].to_vec());
    for incoming in text.bytes() {
        if cursor - line.start >= 65 {
            // Beyond-margin spaces bypass both wrapping and 87EF marking.
            if incoming != b' ' {
                if formatting.right_justify {
                    return Err(EditError::UnqualifiedOperation(
                        "automatic wrapping with right justification",
                    ));
                }
                if incoming == b'-' {
                    return Err(EditError::UnqualifiedOperation("hyphen break rules"));
                }
                wrap_prefix(&mut staged, &mut cursor, &mut line)?;
            }
        } else if incoming == b' ' && cursor > line.start && staged.bytes[cursor - 1] & 0x7f != b' '
        {
            staged.bytes[cursor - 1] |= 0x80;
        }
        if cursor == line.start && incoming == b'.' {
            return Err(EditError::UnqualifiedOperation("dot commands"));
        }
        // Printable overwrite falls back to insertion at CR/LF or EOF (8D38).
        if insert || cursor == line.end {
            staged.bytes.insert(cursor, incoming);
            line.end += 1;
        } else {
            staged.bytes[cursor] = incoming;
        }
        cursor += 1;
    }
    Ok(Edit::Replace {
        active: staged.bytes,
        cursor,
    })
}

fn wrap_prefix(
    staged: &mut Document,
    cursor: &mut usize,
    line: &mut Range<usize>,
) -> Result<(), EditError> {
    if staged.bytes.get(*cursor).is_some_and(|b| b & 0x7f == b'-') {
        return Err(EditError::UnqualifiedOperation("hyphen break rules"));
    }
    let clean = formatting::normalize(staged, line.start..*cursor)?;
    let removed = *cursor - line.start - clean.len();
    staged.bytes.splice(line.start..*cursor, clean);
    *cursor -= removed;
    line.end -= removed;
    if *cursor - line.start < 65 {
        return Ok(());
    }

    // 9048-905C scans through the saved cursor inclusively. The existing byte
    // there may mark its predecessor or trigger a break; its suffix is not read.
    let mut scan = line.start;
    let mut candidate = None;
    let mut leading = true;
    while scan <= *cursor {
        let value = (scan < line.end).then(|| staged.bytes[scan] & 0x7f);
        if value == Some(b' ') {
            if !leading {
                if staged.bytes[scan - 1] & 0x7f != b' ' {
                    staged.bytes[scan - 1] |= 0x80;
                }
                candidate = Some(scan + 1);
            }
            scan += 1;
            continue;
        }
        leading = false;
        // EOF/CRLF at the anchor also tests width >=65 (9067-9078).
        if scan - line.start >= 65
            && let Some(at) = candidate
        {
            if staged.bytes.get(at).is_some_and(|b| b & 0x7f == b'.') {
                return Err(EditError::UnqualifiedOperation(
                    "creating a dot-command line",
                ));
            }
            staged.bytes.splice(at..at, b"\x8d\n".iter().copied());
            *cursor += 2;
            line.end += 2;
            line.start = at + 2;
            scan = line.start;
            candidate = None;
            leading = true;
            continue;
        }
        if value.is_none() {
            break;
        }
        scan += 1;
    }
    Ok(())
}

pub(super) fn enter(
    document: &Document,
    cursor: usize,
    insert: bool,
    formatting: Formatting,
) -> Result<Edit, EditError> {
    check_formatting(formatting)?;
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    if !insert && let Some(next) = line.next {
        return Ok(Edit::MoveTo(next));
    }
    let at = if insert { cursor } else { line.end };
    hard_break_at(document, end, at, at + 2)
}

pub(super) fn insert_hard_break(
    document: &Document,
    cursor: usize,
    formatting: Formatting,
) -> Result<Edit, EditError> {
    check_formatting(formatting)?;
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    if line.end - line.start > 65
        || !document.bytes()[line.start..line.end]
            .iter()
            .all(|byte| (0x20..=0x7e).contains(byte))
        || line
            .next
            .is_some_and(|_| document.bytes()[line.end] != b'\r')
    {
        return Err(EditError::UnqualifiedOperation(
            "Ctrl-N outside an ordinary ASCII line",
        ));
    }
    // The retained Ctrl-N gap-buffer trace inserts on the right of the gap;
    // unlike Enter, the cursor remains before the new CR/LF.
    hard_break_at(document, end, cursor, cursor)
}

fn hard_break_at(
    document: &Document,
    end: usize,
    at: usize,
    next_cursor: usize,
) -> Result<Edit, EditError> {
    if document.bytes().get(at).is_some_and(|b| b & 0x7f == b'.') {
        return Err(EditError::UnqualifiedOperation(
            "creating a dot-command line",
        ));
    }
    let mut active = document.bytes()[..end].to_vec();
    active.splice(at..at, b"\r\n".iter().copied());
    Ok(Edit::Replace {
        active,
        cursor: next_cursor,
    })
}
