// SPDX-License-Identifier: GPL-3.0-only
//! Native UTF-8 documents. No WordStar byte or formatting rules apply here.

use std::{collections::VecDeque, error::Error, fmt};
use unicode_segmentation::UnicodeSegmentation;

/// Retained changed text, across both undo and redo stacks.
pub const UTF8_HISTORY_BYTES: usize = 2 * 1024 * 1024;
pub const UTF8_HISTORY_EDITS: usize = 4096;

struct TextEdit {
    start: usize,
    removed: String,
    inserted: String,
    cursor_before: usize,
    cursor_after: usize,
}

impl TextEdit {
    fn bytes(&self) -> usize {
        self.removed.len() + self.inserted.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Utf8Snapshot {
    pub cursor_byte: usize,
    pub modified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Utf8Error {
    InvalidCursor,
    ControlInput,
}

impl fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCursor => f.write_str("cursor is not at a text boundary"),
            Self::ControlInput => f.write_str("text contains an unsupported control character"),
        }
    }
}

impl Error for Utf8Error {}

/// Owns modern text and its cursor. The stored UTF-8 is never normalized.
pub struct Utf8Editor {
    text: String,
    saved: String,
    cursor: usize,
    undo: VecDeque<TextEdit>,
    redo: VecDeque<TextEdit>,
    history_bytes: usize,
}

impl Utf8Editor {
    pub fn new(text: String) -> Self {
        Self {
            saved: text.clone(),
            text,
            cursor: 0,
            undo: VecDeque::new(),
            redo: VecDeque::new(),
            history_bytes: 0,
        }
    }

    /// Restore an unsaved draft without creating a synthetic undo operation.
    pub fn recover(text: String, saved: String, cursor: usize) -> Result<Self, Utf8Error> {
        let mut editor = Self::new(text);
        editor.set_cursor(cursor)?;
        editor.saved = saved;
        Ok(editor)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn saved_text(&self) -> &str {
        &self.saved
    }

    pub fn graphemes(&self) -> impl Iterator<Item = (usize, &str)> {
        self.text.grapheme_indices(true)
    }

    pub fn snapshot(&self) -> Utf8Snapshot {
        Utf8Snapshot {
            cursor_byte: self.cursor,
            modified: self.text != self.saved,
        }
    }

    pub fn mark_saved(&mut self) {
        self.saved.clone_from(&self.text);
    }

    pub fn set_cursor(&mut self, offset: usize) -> Result<(), Utf8Error> {
        if offset == self.text.len()
            || self
                .text
                .grapheme_indices(true)
                .any(|(start, _)| start == offset)
        {
            self.cursor = offset;
            Ok(())
        } else {
            Err(Utf8Error::InvalidCursor)
        }
    }

    pub fn document_start(&mut self) {
        self.cursor = 0;
    }

    pub fn previous(&mut self) {
        if let Some((start, _)) = self
            .text
            .grapheme_indices(true)
            .take_while(|(start, _)| *start < self.cursor)
            .last()
        {
            self.cursor = start;
        }
    }

    pub fn next(&mut self) {
        if let Some((start, grapheme)) = self
            .text
            .grapheme_indices(true)
            .find(|(start, _)| *start == self.cursor)
        {
            self.cursor = start + grapheme.len();
        }
    }

    pub fn backspace(&mut self) {
        if let Some((start, _)) = self
            .text
            .grapheme_indices(true)
            .take_while(|(start, _)| *start < self.cursor)
            .last()
        {
            self.replace(start, self.cursor, "");
        }
    }

    pub fn delete(&mut self) {
        if let Some((start, grapheme)) = self
            .text
            .grapheme_indices(true)
            .find(|(start, _)| *start == self.cursor)
        {
            self.replace(start, start + grapheme.len(), "");
        }
    }

    pub fn insert_text(&mut self, value: &str) -> Result<(), Utf8Error> {
        if value
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
        {
            return Err(Utf8Error::ControlInput);
        }
        if value.is_empty() {
            return Ok(());
        }
        self.replace(self.cursor, self.cursor, value);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        let Some(edit) = self.undo.pop_back() else {
            return false;
        };
        self.text
            .replace_range(edit.start..edit.start + edit.inserted.len(), &edit.removed);
        self.cursor = edit.cursor_before;
        self.redo.push_back(edit);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(edit) = self.redo.pop_back() else {
            return false;
        };
        self.text
            .replace_range(edit.start..edit.start + edit.removed.len(), &edit.inserted);
        self.cursor = edit.cursor_after;
        self.undo.push_back(edit);
        true
    }

    fn replace(&mut self, start: usize, end: usize, inserted: &str) {
        let removed = self.text[start..end].to_owned();
        let cursor_before = self.cursor;
        self.text.replace_range(start..end, inserted);
        self.cursor = self
            .text
            .grapheme_indices(true)
            .map(|(start, _)| start)
            .chain(std::iter::once(self.text.len()))
            .find(|boundary| *boundary >= start + inserted.len())
            .expect("end of text is a boundary");
        let edit = TextEdit {
            start,
            removed,
            inserted: inserted.to_owned(),
            cursor_before,
            cursor_after: self.cursor,
        };
        self.history_bytes -= self
            .redo
            .drain(..)
            .map(|entry| entry.bytes())
            .sum::<usize>();
        self.history_bytes += edit.bytes();
        self.undo.push_back(edit);
        while self.undo.len() > UTF8_HISTORY_EDITS || self.history_bytes > UTF8_HISTORY_BYTES {
            let oldest = self.undo.pop_front().expect("new edit exists");
            self.history_bytes -= oldest.bytes();
        }
    }

    pub fn enter(&mut self) {
        self.insert_text("\n").expect("line feed is valid input");
    }
}
