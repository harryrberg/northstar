// SPDX-License-Identifier: GPL-3.0-only
//! Byte-preserving legacy documents and a deliberately bounded editing profile.
//!
//! Only declared ASCII editing and reformat scenarios are currently qualified. Parsing a token
//! does not establish editing, layout, or rendering semantics for that token.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt, ops::Range};

mod block;
mod erasure;
mod formatting;
mod reformat;
mod search;
mod typing;
mod utf8;
mod word_navigation;
pub use formatting::Formatting;
pub use utf8::{UTF8_HISTORY_BYTES, UTF8_HISTORY_EDITS, Utf8Editor, Utf8Error, Utf8Snapshot};

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

    fn vertical_target(&self, cursor: usize, down: bool) -> Result<Option<usize>, EditError> {
        let active_end = self.active_end();
        let mut lines = Vec::new();
        let mut start = 0;
        let mut unsupported = None;
        for token in self
            .tokens()
            .take_while(|token| token.range.start < active_end)
        {
            match token.kind {
                TokenKind::Text { .. } | TokenKind::SoftSpace => {}
                TokenKind::LineBreak { page_marker, .. } => {
                    lines.push(NavigationLine {
                        start,
                        end: token.range.start,
                        unsupported,
                        page_break_after: page_marker,
                    });
                    start = token.range.end;
                    unsupported = None;
                }
                _ => {
                    unsupported.get_or_insert(token.range.start);
                }
            }
        }
        lines.push(NavigationLine {
            start,
            end: active_end,
            unsupported,
            page_break_after: false,
        });
        let current = lines
            .iter()
            .position(|line| line.start <= cursor && cursor <= line.end)
            .expect("validated cursor belongs to a physical line");
        let next = if down {
            current.checked_add(1).filter(|index| *index < lines.len())
        } else {
            current.checked_sub(1)
        };
        let Some(next) = next else {
            return Ok(None);
        };
        if lines[if down { current } else { next }].page_break_after {
            return Err(EditError::UnqualifiedOperation("page-marked line break"));
        }
        for line in [&lines[current], &lines[next]] {
            if let Some(offset) = line.unsupported {
                return Err(EditError::UnsupportedDocument { offset });
            }
            if self
                .bytes
                .get(line.start)
                .is_some_and(|byte| byte & 0x7f == b'.')
            {
                return Err(EditError::UnqualifiedOperation("dot commands"));
            }
        }
        let column = cursor - lines[current].start;
        Ok(Some(
            lines[next].start + column.min(lines[next].end - lines[next].start),
        ))
    }

    fn horizontal_boundary(&self, cursor: usize, end: bool) -> Result<usize, EditError> {
        let line = self.ascii_line(cursor, self.checked_active_end()?)?;
        Ok(if end { line.end } else { line.start })
    }
}

struct AsciiLine {
    start: usize,
    end: usize,
    next: Option<usize>,
}

struct NavigationLine {
    start: usize,
    end: usize,
    unsupported: Option<usize>,
    page_break_after: bool,
}

/// Complete operation result; errors are returned before anything is committed.
enum Edit {
    MoveTo(usize),
    Replace { active: Vec<u8>, cursor: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    InsertAscii {
        text: String,
    },
    Enter,
    InsertHardBreak,
    ToggleInsert,
    DocumentStart,
    DocumentEnd,
    LineStart,
    LineEnd,
    JumpBlockBegin,
    JumpBlockEnd,
    NextCharacter,
    PreviousCharacter,
    NextWord,
    PreviousWord,
    NextLine,
    PreviousLine,
    DeleteCharacter,
    Backspace,
    MarkBlockBegin,
    MarkBlockEnd,
    CopyBlock,
    MoveBlock,
    DeleteBlock,
    DeleteWord,
    DeleteLine,
    UneraseLine,
    Reformat,
    FindLiteral {
        text: String,
        ignore_case: bool,
        whole_words: bool,
        backwards: bool,
    },
    ReplaceLiteral {
        text: String,
        replacement: String,
        ignore_case: bool,
        whole_words: bool,
        without_asking: bool,
    },
    AnswerReplace {
        accept: bool,
    },
    RepeatFind,
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
                "this command is not qualified for the document token at byte {offset}"
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
    pub block_begin: Option<usize>,
    pub block_end: Option<usize>,
    pub replace_pending: bool,
}

/// Owns cursor, modes, and document mutation; adapters must not duplicate these.
pub struct Editor {
    document: Document,
    original: Vec<u8>,
    cursor: usize,
    insert: bool,
    formatting: Formatting,
    erased_text: Option<erasure::ErasedText>,
    find: Option<search::FindSpec>,
    replacement: Option<search::ReplaceSpec>,
    replace_pending: bool,
    last_find_result: Option<bool>,
    block: block::Marks,
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
            erased_text: None,
            find: None,
            replacement: None,
            replace_pending: false,
            last_find_result: None,
            block: block::Marks::default(),
        })
    }

    /// Restore a separate unsaved legacy copy and its last saved checkpoint.
    /// This is modern recovery state, not a WordStar command.
    pub fn recover(
        document: Document,
        saved_bytes: Vec<u8>,
        cursor: usize,
        insert: bool,
        formatting: Formatting,
    ) -> Result<Self, EditError> {
        let mut editor = Self::restore_with_formatting(document, cursor, insert, formatting)?;
        editor.original = saved_bytes;
        Ok(editor)
    }

    pub fn saved_bytes(&self) -> &[u8] {
        &self.original
    }

    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The last qualified search result. None means no search has run yet.
    pub fn last_find_result(&self) -> Option<bool> {
        self.last_find_result
    }

    pub fn last_find_text(&self) -> Option<String> {
        self.find.as_ref().map(|spec| spec.text())
    }

    pub fn replacement_pending(&self) -> bool {
        self.replace_pending
    }

    pub fn last_search_replaces(&self) -> bool {
        self.replacement.is_some()
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
            block_begin: self.block.begin,
            block_end: self.block.end,
            replace_pending: self.replace_pending,
        }
    }

    /// Reject unsupported cases before changing state. Returned errors are atomic.
    pub fn apply(&mut self, command: &Command) -> Result<(), EditError> {
        if self.replace_pending && !matches!(command, Command::AnswerReplace { .. }) {
            return Err(EditError::UnqualifiedOperation(
                "answer the pending replacement before another command",
            ));
        }
        match command {
            Command::ToggleInsert => self.insert = !self.insert,
            Command::DocumentStart => self.cursor = 0,
            Command::DocumentEnd => self.cursor = self.document.checked_active_end()?,
            Command::LineStart | Command::LineEnd => {
                self.cursor = self
                    .document
                    .horizontal_boundary(self.cursor, matches!(command, Command::LineEnd))?;
            }
            Command::JumpBlockBegin | Command::JumpBlockEnd => {
                self.cursor = if matches!(command, Command::JumpBlockBegin) {
                    self.block.begin
                } else {
                    self.block.end
                }
                .ok_or(EditError::UnqualifiedOperation(
                    "block marker has not yet been defined",
                ))?;
            }
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
            Command::NextWord | Command::PreviousWord => {
                if let Some(cursor) = word_navigation::target(
                    &self.document,
                    self.cursor,
                    matches!(command, Command::NextWord),
                )? {
                    self.cursor = cursor;
                }
            }
            Command::NextLine | Command::PreviousLine => {
                if let Some(cursor) = self
                    .document
                    .vertical_target(self.cursor, matches!(command, Command::NextLine))?
                {
                    self.cursor = cursor;
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
            Command::InsertHardBreak => {
                self.commit_edit(typing::insert_hard_break(
                    &self.document,
                    self.cursor,
                    self.formatting,
                )?);
            }
            Command::DeleteCharacter => {
                self.commit_edit(erasure::delete_character(
                    &self.document,
                    self.cursor,
                    self.formatting,
                )?);
            }
            Command::Backspace => {
                self.commit_edit(erasure::backspace(
                    &self.document,
                    self.cursor,
                    self.formatting,
                )?);
            }
            Command::MarkBlockBegin => {
                block::mark(&self.document, self.cursor, self.formatting)?;
                self.block.begin = Some(self.cursor);
            }
            Command::MarkBlockEnd => {
                block::mark(&self.document, self.cursor, self.formatting)?;
                self.block.end = Some(self.cursor);
            }
            Command::CopyBlock => {
                let (edit, marks) =
                    block::copy(&self.document, self.cursor, self.formatting, self.block)?;
                self.commit_edit(edit);
                self.block = marks;
            }
            Command::MoveBlock => {
                if !self.insert {
                    return Err(EditError::UnqualifiedOperation(
                        "block movement with Insert off",
                    ));
                }
                let (edit, marks) =
                    block::move_after(&self.document, self.cursor, self.formatting, self.block)?;
                self.commit_edit(edit);
                self.block = marks;
            }
            Command::DeleteBlock => {
                if !self.insert {
                    return Err(EditError::UnqualifiedOperation(
                        "block deletion with Insert off",
                    ));
                }
                self.commit_edit(block::delete(
                    &self.document,
                    self.cursor,
                    self.formatting,
                    self.block,
                )?);
            }
            Command::DeleteWord => {
                if let Some((edit, erased)) =
                    erasure::delete_word(&self.document, self.cursor, self.formatting)?
                {
                    self.commit_edit(edit);
                    self.erased_text = Some(erasure::ErasedText::Word(erased));
                }
            }
            Command::DeleteLine => {
                if let Some((edit, erased)) =
                    erasure::delete_line(&self.document, self.cursor, self.formatting)?
                {
                    self.commit_edit(edit);
                    self.erased_text = Some(erasure::ErasedText::Line(erased));
                }
            }
            Command::UneraseLine => {
                self.commit_edit(erasure::unerase_line(
                    &self.document,
                    self.cursor,
                    self.formatting,
                    self.erased_text.as_ref(),
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
            Command::FindLiteral {
                text,
                ignore_case,
                whole_words,
                backwards,
            } => {
                let spec = search::FindSpec::new(text, *ignore_case, *whole_words, *backwards)?;
                let target = search::target(&self.document, self.cursor, &spec, false)?;
                self.cursor = target.unwrap_or_else(|| {
                    if *backwards {
                        0
                    } else {
                        self.document.active_end()
                    }
                });
                self.last_find_result = Some(target.is_some());
                self.find = Some(spec);
                self.replacement = None;
                self.replace_pending = false;
            }
            Command::ReplaceLiteral {
                text,
                replacement,
                ignore_case,
                whole_words,
                without_asking,
            } => {
                search::check_replace_context(self.formatting, self.insert)?;
                if self.block.begin.is_some() || self.block.end.is_some() {
                    return Err(EditError::UnqualifiedOperation(
                        "replacement while block marks are set",
                    ));
                }
                let spec = search::FindSpec::new(text, *ignore_case, *whole_words, false)?;
                let replace = search::ReplaceSpec::new(text, replacement, *without_asking)?;
                let target = search::target(&self.document, self.cursor, &spec, false)?;
                let edit = target
                    .map(|cursor| {
                        search::replacement_edit(&self.document, cursor, text.len(), &replace)
                    })
                    .transpose()?;
                self.cursor = target.unwrap_or_else(|| self.document.active_end());
                self.last_find_result = Some(target.is_some());
                self.replace_pending = target.is_some() && !*without_asking;
                self.find = Some(spec);
                self.replacement = Some(replace);
                if *without_asking && let Some(edit) = edit {
                    self.commit_edit(edit);
                }
            }
            Command::AnswerReplace { accept } => {
                if !self.replace_pending {
                    return Err(EditError::UnqualifiedOperation(
                        "no replacement question is pending",
                    ));
                }
                if *accept {
                    let replacement = self.replacement.as_ref().expect("pending replacement");
                    let find_len = self.find.as_ref().expect("pending find").len();
                    let edit = search::replacement_edit(
                        &self.document,
                        self.cursor,
                        find_len,
                        replacement,
                    )?;
                    self.commit_edit(edit);
                }
                self.replace_pending = false;
            }
            Command::RepeatFind => {
                let spec = self
                    .find
                    .as_ref()
                    .ok_or(EditError::UnqualifiedOperation("no previous find"))?;
                if self.replacement.is_some() {
                    search::check_replace_context(self.formatting, self.insert)?;
                }
                let target = search::target(&self.document, self.cursor, spec, true)?;
                let edit = if let (Some(cursor), Some(replacement)) = (target, &self.replacement) {
                    Some(search::replacement_edit(
                        &self.document,
                        cursor,
                        spec.len(),
                        replacement,
                    )?)
                } else {
                    None
                };
                self.cursor = target.unwrap_or_else(|| {
                    if spec.backwards {
                        0
                    } else {
                        self.document.active_end()
                    }
                });
                self.last_find_result = Some(target.is_some());
                self.replace_pending = target.is_some()
                    && self
                        .replacement
                        .as_ref()
                        .is_some_and(|replacement| !replacement.without_asking);
                if self
                    .replacement
                    .as_ref()
                    .is_some_and(|replacement| replacement.without_asking)
                    && let Some(edit) = edit
                {
                    self.commit_edit(edit);
                }
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
                // Other editing commands have no measured mark-adjustment rule.
                self.block = block::Marks::default();
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
