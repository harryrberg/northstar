// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor, Formatting, Utf8Editor};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

// The prototype scans the carrier for navigation and projection. Bound that
// work until larger-document latency has been measured; this is not a format limit.
pub const DOCUMENT_LIMIT: usize = 1024 * 1024;

pub struct Session {
    pub editor: Editor,
    pub path: Option<PathBuf>,
}

pub struct Utf8Session {
    pub editor: Utf8Editor,
    pub path: Option<PathBuf>,
}

pub enum DocumentSession {
    Legacy(Session),
    Utf8(Utf8Session),
}

impl DocumentSession {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Legacy(session) => session.path.as_deref(),
            Self::Utf8(session) => session.path.as_deref(),
        }
    }

    pub fn modified(&self) -> bool {
        match self {
            Self::Legacy(session) => session.editor.snapshot().modified,
            Self::Utf8(session) => session.editor.snapshot().modified,
        }
    }

    pub fn insert_mode(&self) -> bool {
        match self {
            Self::Legacy(session) => session.editor.snapshot().insert,
            Self::Utf8(_) => true,
        }
    }

    pub fn is_utf8(&self) -> bool {
        matches!(self, Self::Utf8(_))
    }

    pub fn apply_command(&mut self, command: &Command) -> Result<(), String> {
        match self {
            Self::Legacy(session) => session.editor.apply(command).map_err(|e| e.to_string()),
            Self::Utf8(session) => {
                let editor = &mut session.editor;
                match command {
                    Command::DocumentStart => editor.document_start(),
                    Command::PreviousCharacter => editor.previous(),
                    Command::NextCharacter => editor.next(),
                    Command::Enter => editor.enter(),
                    Command::InsertAscii { .. } | Command::ToggleInsert | Command::Reformat => {
                        return Err("This WordStar command is not available in UTF-8 mode".into());
                    }
                }
                Ok(())
            }
        }
    }

    pub fn insert_text(&mut self, text: &str) -> Result<(), String> {
        match self {
            Self::Legacy(session) => session
                .editor
                .apply(&Command::InsertAscii { text: text.into() })
                .map_err(|e| e.to_string()),
            Self::Utf8(session) => {
                if session.editor.text().len().saturating_add(text.len()) > DOCUMENT_LIMIT {
                    return Err("Desktop prototype limit: 1 MiB".into());
                }
                session.editor.insert_text(text).map_err(|e| e.to_string())
            }
        }
    }

    pub fn backspace(&mut self) -> Result<(), String> {
        match self {
            Self::Legacy(_) => Err("Deletion is not yet qualified for WordStar files".into()),
            Self::Utf8(session) => {
                session.editor.backspace();
                Ok(())
            }
        }
    }

    pub fn delete(&mut self) -> Result<(), String> {
        match self {
            Self::Legacy(_) => Err("Deletion is not yet qualified for WordStar files".into()),
            Self::Utf8(session) => {
                session.editor.delete();
                Ok(())
            }
        }
    }

    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        match self {
            Self::Legacy(session) => session.save_as(path),
            Self::Utf8(session) => session.save_as(path),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::from_bytes(Vec::new(), None)
    }
}

impl Session {
    pub fn new_at(path: PathBuf) -> Self {
        Self::from_bytes(Vec::new(), Some(path))
    }
    fn from_bytes(bytes: Vec<u8>, path: Option<PathBuf>) -> Self {
        Self {
            editor: Editor::restore_with_formatting(
                Document::from_bytes(bytes),
                0,
                true,
                Formatting {
                    right_justify: false,
                    ..Formatting::default()
                },
            )
            .expect("zero is always an active document boundary"),
            path,
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self::from_bytes(
            read_document(path)?,
            Some(path.to_owned()),
        ))
    }

    /// No-clobber Save As. Failure keeps the active path and dirty checkpoint.
    /// An I/O error after creation can leave an incomplete file at the new path.
    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        write_new_file(path, self.editor.document().bytes())?;
        self.editor.mark_saved();
        self.path = Some(path.to_owned());
        Ok(())
    }
}

impl Utf8Session {
    pub fn new_at(path: Option<PathBuf>) -> Self {
        Self {
            editor: Utf8Editor::new(String::new()),
            path,
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let text = String::from_utf8(read_document(path)?)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "File is not valid UTF-8"))?;
        Ok(Self {
            editor: Utf8Editor::new(text),
            path: Some(path.to_owned()),
        })
    }

    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        write_new_file(path, self.editor.text().as_bytes())?;
        self.editor.mark_saved();
        self.path = Some(path.to_owned());
        Ok(())
    }
}

fn read_document(path: &Path) -> io::Result<Vec<u8>> {
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("Choose a regular document file"));
    }
    let mut bytes = Vec::new();
    file.take(DOCUMENT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > DOCUMENT_LIMIT {
        return Err(io::Error::other("Desktop prototype limit: 1 MiB"));
    }
    Ok(bytes)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}
