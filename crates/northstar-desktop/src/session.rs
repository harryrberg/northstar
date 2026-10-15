// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Document, Editor, Formatting};
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
        Ok(Self::from_bytes(bytes, Some(path.to_owned())))
    }

    /// No-clobber Save As. Failure keeps the active path and dirty checkpoint.
    /// An I/O error after creation can leave an incomplete file at the new path.
    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(self.editor.document().bytes())?;
        file.sync_all()?;
        self.editor.mark_saved();
        self.path = Some(path.to_owned());
        Ok(())
    }
}
