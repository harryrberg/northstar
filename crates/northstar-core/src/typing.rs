// SPDX-License-Identifier: GPL-3.0-only
//! Local ASCII typing and Enter; automatic wrapping is outside this profile.
use crate::{Document, Edit, EditError, Formatting};

fn check_formatting(formatting: Formatting) -> Result<(), EditError> {
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
    cursor: usize,
    insert: bool,
    text: &str,
    formatting: Formatting,
) -> Result<Edit, EditError> {
    check_formatting(formatting)?;
    if !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(EditError::UnsupportedInput);
    }
    let end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, end)?;
    if text.is_empty() {
        return Ok(Edit::MoveTo(cursor));
    }
    // 86A4/86AA tests the cursor before typing, not the resulting line length.
    // A whole batch is refused if any character would start beyond column 65.
    if text.len() > 65usize.saturating_sub(cursor - line.start) {
        return Err(EditError::UnqualifiedOperation("typing beyond column 65"));
    }
    if cursor == line.start && text.starts_with('.') {
        return Err(EditError::UnqualifiedOperation("dot commands"));
    }
    let mut incoming = text.as_bytes().to_vec();
    for i in 1..incoming.len() {
        if incoming[i] == b' ' && incoming[i - 1] & 0x7f != b' ' {
            incoming[i - 1] |= 0x80;
        }
    }
    let mut active = document.bytes()[..end].to_vec();
    // 87EF marks the preceding character when a space is typed. Ordinary
    // letters do not repair stale markers, and overwritten markers are lost.
    if incoming[0] == b' ' && cursor > line.start && active[cursor - 1] & 0x7f != b' ' {
        active[cursor - 1] |= 0x80;
    }
    let replaced = if insert {
        0
    } else {
        text.len().min(line.end - cursor)
    };
    // Printable overwrite falls back to insertion at CR/LF or EOF (8D38).
    active.splice(cursor..cursor + replaced, incoming);
    Ok(Edit::Replace {
        active,
        cursor: cursor + text.len(),
    })
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
    if document.bytes().get(at).is_some_and(|b| b & 0x7f == b'.') {
        return Err(EditError::UnqualifiedOperation(
            "creating a dot-command line",
        ));
    }
    let mut active = document.bytes()[..end].to_vec();
    active.splice(at..at, b"\r\n".iter().copied());
    Ok(Edit::Replace {
        active,
        cursor: at + 2,
    })
}
