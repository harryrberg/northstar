// SPDX-License-Identifier: GPL-3.0-only
//! Bounded insertion of a WordStar carrier through Ctrl-K R.
use crate::{Document, EditError, Formatting, block::Marks, typing};

pub(super) fn prepare(
    document: &Document,
    cursor: usize,
    insert_mode: bool,
    formatting: Formatting,
    marks: Marks,
    source: &Document,
) -> Result<Vec<u8>, EditError> {
    typing::check_formatting(formatting)?;
    if formatting.right_justify || !insert_mode {
        return Err(EditError::UnqualifiedOperation(
            "file insertion requires Insert on and justification off",
        ));
    }
    let active_end = document.checked_active_end()?;
    let line = document.ascii_line(cursor, active_end)?;
    match (marks.begin, marks.end) {
        (None, None) => {}
        (Some(begin), Some(end))
            if line.start <= begin && begin < end && cursor == end && end <= line.end => {}
        _ => {
            return Err(EditError::UnqualifiedOperation(
                "file insertion with marks outside the observed end-mark position",
            ));
        }
    }
    if !document.bytes()[line.start..line.end]
        .iter()
        .all(|byte| (0x21..=0x7e).contains(byte))
    {
        return Err(EditError::UnqualifiedOperation(
            "file insertion into a line with spaces or high-bit text",
        ));
    }
    if source.bytes().len() != 128 {
        return Err(EditError::UnqualifiedOperation(
            "inserted file must be a 128-byte WordStar carrier",
        ));
    }
    let source_end = source.checked_active_end()?;
    if source_end == 0
        || source_end + 2 > source.bytes().len()
        || !source.bytes()[..source_end]
            .iter()
            .all(|byte| (0x21..=0x7e).contains(byte))
    {
        return Err(EditError::UnqualifiedOperation(
            "inserted file must contain one ordinary ASCII line and EOF padding",
        ));
    }
    if cursor == line.start && source.bytes()[0] == b'.' {
        return Err(EditError::UnqualifiedOperation("dot commands"));
    }
    if line.end - line.start + source_end > 65 {
        return Err(EditError::UnqualifiedOperation(
            "file insertion would cross the qualified right margin",
        ));
    }
    let new_length = active_end
        .checked_add(source_end)
        .ok_or(EditError::UnqualifiedOperation(
            "file insertion exceeds capacity",
        ))?;
    if (new_length + 2).div_ceil(128) * 128 > 1024 * 1024 {
        return Err(EditError::UnqualifiedOperation(
            "file insertion exceeds the 1 MiB prototype limit",
        ));
    }
    let mut active = Vec::with_capacity(new_length);
    active.extend_from_slice(&document.bytes()[..cursor]);
    active.extend_from_slice(&source.bytes()[..source_end]);
    active.extend_from_slice(&document.bytes()[cursor..active_end]);
    Ok(active)
}
