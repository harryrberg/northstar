// SPDX-License-Identifier: GPL-3.0-only
//! Byte-preserving legacy documents and a deliberately bounded editing profile.
//!
//! Only declared ASCII editing and reformat scenarios are currently qualified. Parsing a token
//! does not establish editing, layout, or rendering semantics for that token.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, ops::Range};

mod formatting;
mod reformat;
mod typing;
mod utf8;
pub use formatting::Formatting;
pub use utf8::{Utf8Editor, Utf8Error, Utf8Snapshot};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TokenKind {
    Text { value: u8, high_bit: bool },
    LineBreak { soft: bool, page_marker: bool },
    FramedCharacter { code: u8, high_bit_prefix: bool },
    UnresolvedFrame,
    UnpairedCarriageReturn,
    PaddingCandidate,
    SoftSpace,
    Tab,
    Control { value: u8, high_bit: bool },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Token {
    pub range: Range<usize>,
    #[serde(flatten)]
    pub kind: TokenKind,
}

/// The original carrier is authoritative, including bytes after EOF candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    bytes: Vec<u8>,
}

impl Document {
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Token ranges partition the entire carrier. No normalization is applied.
    pub fn tokens(&self) -> impl Iterator<Item = Token> + '_ {
        let mut offset = 0;
        std::iter::from_fn(move || {
            let start = offset;
            let &byte = self.bytes.get(start)?;
            offset += 1;
            let kind = match byte {
                0x1b | 0x9b => {
                    offset = start.saturating_add(3).min(self.bytes.len());
                    if self.bytes.get(start + 2) == Some(&0x1c) {
                        TokenKind::FramedCharacter {
                            code: self.bytes[start + 1],
                            high_bit_prefix: byte == 0x9b,
                        }
                    } else {
                        TokenKind::UnresolvedFrame
                    }
                }
                0x0d | 0x8d => match self.bytes.get(offset) {
                    Some(lf @ (0x0a | 0x8a)) => {
                        offset += 1;
                        TokenKind::LineBreak {
                            soft: byte == 0x8d,
                            page_marker: *lf == 0x8a,
                        }
                    }
                    _ => TokenKind::UnpairedCarriageReturn,
                },
                0x1a => {
                    while self.bytes.get(offset) == Some(&0x1a) {
                        offset += 1;
                    }
                    TokenKind::PaddingCandidate
                }
                0xa0 => TokenKind::SoftSpace,
                0x09 => TokenKind::Tab,
                b if (0x20..=0x7e).contains(&(b & 0x7f)) => TokenKind::Text {
                    value: b & 0x7f,
                    high_bit: b & 0x80 != 0,
                },
                b if b & 0x7f < 0x20 => TokenKind::Control {
                    value: b & 0x7f,
                    high_bit: b & 0x80 != 0,
                },
                _ => TokenKind::Unknown,
            };
            Some(Token {
                range: start..offset,
                kind,
            })
        })
    }

    /// An editing-profile boundary, not a universal interpretation of DOS EOF.
    pub fn active_end(&self) -> usize {
        self.tokens()
            .find(|t| t.kind == TokenKind::PaddingCandidate)
            .map_or(self.bytes.len(), |t| t.range.start)
    }

    /// Mutations must not discard data after an EOF candidate.
    fn checked_active_end(&self) -> Result<usize, EditError> {
        for token in self.tokens() {
            if token.kind == TokenKind::PaddingCandidate {
                if token.range.end != self.bytes.len() {
                    return Err(EditError::UnsupportedDocument {
                        offset: token.range.start,
                    });
                }
                return Ok(token.range.start);
            }
        }
        Ok(self.bytes.len())
    }

    /// A physical line whose text cells are all one-byte ASCII or soft spaces.
    /// Other lines are retained without imposing this line's editing contract.
    fn ascii_line(&self, cursor: usize, active_end: usize) -> Result<AsciiLine, EditError> {
        let mut line = AsciiLine {
            start: 0,
            end: active_end,
            next: None,
        };
        for token in self.tokens() {
            if let TokenKind::LineBreak { page_marker, .. } = token.kind {
                if token.range.end <= cursor {
                    line.start = token.range.end;
                } else {
                    if page_marker {
                        return Err(EditError::UnsupportedDocument {
                            offset: token.range.start,
                        });
                    }
                    line.end = token.range.start;
                    line.next = Some(token.range.end);
                    break;
                }
            }
        }
        for token in self
            .tokens()
            .skip_while(|t| t.range.start < line.start)
            .take_while(|t| t.range.start < line.end)
        {
            if !matches!(token.kind, TokenKind::Text { .. } | TokenKind::SoftSpace) {
                return Err(EditError::UnsupportedDocument {
                    offset: token.range.start,
                });
            }
        }
        if self.bytes.get(line.start).is_some_and(|b| b & 0x7f == b'.') {
            return Err(EditError::UnqualifiedOperation("dot commands"));
        }
        Ok(line)
    }
}

struct AsciiLine {
    start: usize,
    end: usize,
    next: Option<usize>,
}

/// Complete operation result; errors are returned before anything is committed.
enum Edit {
    MoveTo(usize),
    Replace { active: Vec<u8>, cursor: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    InsertAscii { text: String },
    Enter,
    ToggleInsert,
    DocumentStart,
    NextCharacter,
    PreviousCharacter,
    Reformat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    UnsupportedDocument { offset: usize },
    InvalidCursor { offset: usize },
    UnsupportedInput,
    UnqualifiedOperation(&'static str),
}

impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDocument { offset } => write!(
                f,
                "editing is not qualified for the document token at byte {offset}"
            ),
            Self::InvalidCursor { offset } => {
                write!(f, "byte {offset} is not an active token boundary")
            }
            Self::UnsupportedInput => {
                f.write_str("only printable ASCII is supported by this editing profile")
            }
            Self::UnqualifiedOperation(reason) => write!(f, "operation is not qualified: {reason}"),
        }
    }
}

impl Error for EditError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub cursor_byte: usize,
    pub insert: bool,
    pub modified: bool,
    pub active_bytes: usize,
    pub formatting: Formatting,
}

/// Owns cursor, modes, and document mutation; adapters must not duplicate these.
pub struct Editor {
    document: Document,
    original: Vec<u8>,
    cursor: usize,
    insert: bool,
    formatting: Formatting,
}

impl Editor {
    /// Restore declared scenario state. This is not a WordStar command.
    pub fn restore(document: Document, cursor: usize, insert: bool) -> Result<Self, EditError> {
        Self::restore_with_formatting(document, cursor, insert, Formatting::default())
    }

    /// Restore explicitly declared formatting; unsupported settings are retained
    /// and rejected by operations that need them, never silently normalized.
    pub fn restore_with_formatting(
        document: Document,
        cursor: usize,
        insert: bool,
        formatting: Formatting,
    ) -> Result<Self, EditError> {
        if cursor > document.active_end()
            || !(cursor == document.active_end()
                || document.tokens().any(|t| t.range.start == cursor))
        {
            return Err(EditError::InvalidCursor { offset: cursor });
        }
        Ok(Self {
            original: document.bytes.clone(),
            document,
            cursor,
            insert,
            formatting,
        })
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Record a successfully persisted carrier. The caller must finish writing
    /// and syncing before advancing this checkpoint. This is an application
    /// operation, not a reconstruction of WordStar's internal dirty flag.
    pub fn mark_saved(&mut self) {
        self.original.clone_from(&self.document.bytes);
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            cursor_byte: self.cursor,
            insert: self.insert,
            modified: self.document.bytes != self.original,
            active_bytes: self.document.active_end(),
            formatting: self.formatting,
        }
    }

    /// Reject unsupported cases before changing state. Returned errors are atomic.
    pub fn apply(&mut self, command: &Command) -> Result<(), EditError> {
        match command {
            Command::ToggleInsert => self.insert = !self.insert,
            Command::DocumentStart => self.cursor = 0,
            Command::NextCharacter => {
                let end = self.document.active_end();
                if let Some(token) = self
                    .document
                    .tokens()
                    .find(|t| t.range.start == self.cursor && t.range.start < end)
                {
                    self.cursor = token.range.end;
                }
            }
            Command::PreviousCharacter => {
                if let Some(token) = self
                    .document
                    .tokens()
                    .take_while(|t| t.range.end <= self.cursor)
                    .last()
                {
                    self.cursor = token.range.start;
                }
            }
            Command::Enter => {
                self.commit_edit(typing::enter(
                    &self.document,
                    self.cursor,
                    self.insert,
                    self.formatting,
                )?);
            }
            Command::InsertAscii { text } => {
                self.commit_edit(typing::type_ascii(
                    &self.document,
                    self.cursor,
                    self.insert,
                    text,
                    self.formatting,
                )?);
            }
            Command::Reformat => {
                self.commit_edit(reformat::paragraph(
                    &self.document,
                    self.cursor,
                    self.formatting,
                )?);
            }
        }
        Ok(())
    }

    fn commit_edit(&mut self, edit: Edit) {
        match edit {
            Edit::MoveTo(cursor) => self.cursor = cursor,
            Edit::Replace { active, cursor } => {
                self.replace_active(active);
                self.cursor = cursor;
            }
        }
    }

    fn replace_active(&mut self, mut active: Vec<u8>) {
        // Explicit mutations serialize records; navigation/inspection retain the
        // carrier. DOS captures at active lengths 126..129 require two EOF bytes.
        let carrier_size = (active.len() + 2).div_ceil(128) * 128;
        active.resize(carrier_size, 0x1a);
        self.document = Document::from_bytes(active);
    }
}
