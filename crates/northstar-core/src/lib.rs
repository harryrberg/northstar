// SPDX-License-Identifier: GPL-3.0-only
//! Byte-preserving legacy documents and a deliberately bounded editing profile.
//!
//! Only declared Enter and ASCII reformat scenarios are currently qualified. Parsing a token
//! does not establish editing, layout, or rendering semantics for that token.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, ops::Range};

mod reformat;
pub use reformat::Formatting;

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

    fn check_editable(&self) -> Result<(), EditError> {
        let end = self.active_end();
        for token in self.tokens() {
            let supported = match token.kind {
                TokenKind::Text {
                    high_bit: false, ..
                }
                | TokenKind::LineBreak {
                    soft: false,
                    page_marker: false,
                } => token.range.start < end,
                TokenKind::PaddingCandidate => {
                    token.range.start == end && token.range.end == self.bytes.len()
                }
                _ => false,
            };
            if !supported {
                return Err(EditError::UnsupportedDocument {
                    offset: token.range.start,
                });
            }
        }
        Ok(())
    }
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
        if matches!(command, Command::Enter | Command::InsertAscii { .. }) {
            if (self.formatting.left_margin, self.formatting.right_margin) != (1, 65)
                || self.formatting.line_spacing != 1
                || !self.formatting.document_mode
            {
                return Err(EditError::UnqualifiedOperation(
                    "ASCII editing requires document mode, single spacing, and margins 1/65",
                ));
            }
            self.document.check_editable()?;
        }
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
            Command::Enter if self.insert => self.insert_bytes(b"\r\n"),
            Command::Enter => {
                let next = self.document.tokens().find(|t| {
                    t.range.start >= self.cursor && matches!(t.kind, TokenKind::LineBreak { .. })
                });
                match next {
                    Some(token) => self.cursor = token.range.end,
                    None => {
                        return Err(EditError::UnqualifiedOperation(
                            "overwrite Enter on a final line without CRLF",
                        ));
                    }
                }
            }
            Command::InsertAscii { text } => {
                if !text.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
                    return Err(EditError::UnsupportedInput);
                }
                if text.is_empty() {
                    return Ok(());
                }
                // Automatic wrap and overwrite typing require separate evidence.
                if !self.insert {
                    return Err(EditError::UnqualifiedOperation("overwrite typing"));
                }
                let line_start = self.document.bytes[..self.cursor]
                    .iter()
                    .rposition(|b| *b == b'\n')
                    .map_or(0, |i| i + 1);
                let line_end = self.document.bytes[self.cursor..self.document.active_end()]
                    .iter()
                    .position(|b| *b == b'\r')
                    .map_or(self.document.active_end(), |i| self.cursor + i);
                if line_end - line_start + text.len() > 64 {
                    return Err(EditError::UnqualifiedOperation(
                        "automatic wrapping at the default right margin",
                    ));
                }
                self.insert_bytes(text.as_bytes());
            }
            Command::Reformat => {
                let change = reformat::paragraph(&self.document, self.cursor, self.formatting)?;
                self.replace_active(change.active);
                self.cursor = change.cursor;
            }
        }
        Ok(())
    }

    fn insert_bytes(&mut self, bytes: &[u8]) {
        let mut active = self.document.bytes[..self.document.active_end()].to_vec();
        active.splice(self.cursor..self.cursor, bytes.iter().copied());
        self.cursor += bytes.len();
        self.replace_active(active);
    }

    fn replace_active(&mut self, mut active: Vec<u8>) {
        // Explicit mutations serialize records; navigation/inspection retain the
        // carrier. DOS captures at active lengths 126..129 require two EOF bytes.
        let carrier_size = (active.len() + 2).div_ceil(128) * 128;
        active.resize(carrier_size, 0x1a);
        self.document = Document::from_bytes(active);
    }
}
