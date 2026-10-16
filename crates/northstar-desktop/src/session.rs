// SPDX-License-Identifier: GPL-3.0-only
use northstar_core::{Command, Document, Editor, Formatting, Utf8Editor};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

// The prototype scans the carrier for navigation and projection. Bound that
// work until larger-document latency has been measured; this is not a format limit.
pub const DOCUMENT_LIMIT: usize = 1024 * 1024;

pub struct Session {
    pub editor: Editor,
    pub path: Option<PathBuf>,
    baseline: Option<Vec<u8>>,
}

pub struct Utf8Session {
    pub editor: Utf8Editor,
    pub path: Option<PathBuf>,
    baseline: Option<Vec<u8>>,
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
                    Command::InsertAscii { .. }
                    | Command::ToggleInsert
                    | Command::Reformat
                    | Command::NextLine
                    | Command::PreviousLine
                    | Command::DeleteCharacter
                    | Command::DeleteWord
                    | Command::DeleteLine
                    | Command::UneraseLine => {
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

    pub fn undo(&mut self) -> Result<bool, String> {
        match self {
            Self::Legacy(_) => Err("Undo is not yet qualified for WordStar files".into()),
            Self::Utf8(session) => Ok(session.editor.undo()),
        }
    }

    pub fn redo(&mut self) -> Result<bool, String> {
        match self {
            Self::Legacy(_) => Err("Redo is not available for WordStar files".into()),
            Self::Utf8(session) => Ok(session.editor.redo()),
        }
    }

    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        match self {
            Self::Legacy(session) => session.save_as(path),
            Self::Utf8(session) => session.save_as(path),
        }
    }

    pub fn save(&mut self) -> io::Result<()> {
        match self {
            Self::Legacy(session) => session.save(),
            Self::Utf8(session) => session.save(),
        }
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::from_bytes(Vec::new(), None, false)
    }
}

impl Session {
    pub fn new_at(path: PathBuf) -> Self {
        Self::from_bytes(Vec::new(), Some(path), false)
    }
    fn from_bytes(bytes: Vec<u8>, path: Option<PathBuf>, existing: bool) -> Self {
        let baseline = existing.then(|| bytes.clone());
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
            baseline,
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        Ok(Self::from_bytes(
            read_document(path)?,
            Some(path.to_owned()),
            true,
        ))
    }

    /// No-clobber Save As. Failure keeps the active path and dirty checkpoint.
    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        write_new_file(path, self.editor.document().bytes())?;
        self.editor.mark_saved();
        self.path = Some(path.to_owned());
        self.baseline = Some(self.editor.document().bytes().to_vec());
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.as_deref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Choose a file name before saving")
        })?;
        let bytes = self.editor.document().bytes();
        save_current(path, self.baseline.as_deref(), bytes, BackupKind::Legacy)?;
        self.baseline = Some(bytes.to_vec());
        self.editor.mark_saved();
        Ok(())
    }
}

impl Utf8Session {
    pub fn new_at(path: Option<PathBuf>) -> Self {
        Self {
            editor: Utf8Editor::new(String::new()),
            path,
            baseline: None,
        }
    }

    pub fn open(path: &Path) -> io::Result<Self> {
        let bytes = read_document(path)?;
        let text = String::from_utf8(bytes.clone())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "File is not valid UTF-8"))?;
        Ok(Self {
            editor: Utf8Editor::new(text),
            path: Some(path.to_owned()),
            baseline: Some(bytes),
        })
    }

    pub fn save_as(&mut self, path: &Path) -> io::Result<()> {
        write_new_file(path, self.editor.text().as_bytes())?;
        self.editor.mark_saved();
        self.path = Some(path.to_owned());
        self.baseline = Some(self.editor.text().as_bytes().to_vec());
        Ok(())
    }

    pub fn save(&mut self) -> io::Result<()> {
        let path = self.path.as_deref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Choose a file name before saving")
        })?;
        let bytes = self.editor.text().as_bytes();
        save_current(path, self.baseline.as_deref(), bytes, BackupKind::Utf8)?;
        self.baseline = Some(bytes.to_vec());
        self.editor.mark_saved();
        Ok(())
    }

    pub(crate) fn baseline(&self) -> Option<&[u8]> {
        self.baseline.as_deref()
    }
}

pub(crate) fn read_document(path: &Path) -> io::Result<Vec<u8>> {
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
    let staged = StagedFile::write(path, bytes, None)?;
    fs::hard_link(&staged.path, path)
}

#[derive(Clone, Copy)]
enum BackupKind {
    Legacy,
    Utf8,
}

fn backup_path(path: &Path, kind: BackupKind) -> io::Result<PathBuf> {
    let backup = match kind {
        BackupKind::Legacy => path.with_extension("BAK"),
        BackupKind::Utf8 => {
            let mut name = path
                .file_name()
                .ok_or_else(|| io::Error::other("Choose a document file"))?
                .to_os_string();
            name.push(".bak");
            path.with_file_name(name)
        }
    };
    let same_name = if cfg!(windows) {
        backup
            .file_name()
            .zip(path.file_name())
            .is_some_and(|(a, b)| {
                a.to_string_lossy()
                    .eq_ignore_ascii_case(&b.to_string_lossy())
            })
    } else {
        backup == path
    };
    if same_name {
        return Err(io::Error::other(
            "Document name conflicts with its backup name",
        ));
    }
    Ok(backup)
}

fn regular_writable(path: &Path) -> io::Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(io::Error::other(
            "Saving requires a regular file, not a link or directory",
        ));
    }
    if metadata.permissions().readonly() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "File is read-only",
        ));
    }
    Ok(metadata)
}

fn save_current(
    path: &Path,
    baseline: Option<&[u8]>,
    bytes: &[u8],
    kind: BackupKind,
) -> io::Result<()> {
    let Some(expected) = baseline else {
        return write_new_file(path, bytes);
    };
    let metadata = regular_writable(path)?;
    let current = read_document(path)?;
    if current != expected {
        return Err(io::Error::other(
            "File changed outside NorthStar; use Ctrl-Shift-S to save a copy",
        ));
    }
    let backup = backup_path(path, kind)?;
    let backup_exists = match fs::symlink_metadata(&backup) {
        Ok(_) => {
            regular_writable(&backup)?;
            true
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => false,
        Err(e) => return Err(e),
    };
    let staged_document = StagedFile::write(path, bytes, Some(&metadata))?;
    let staged_backup = StagedFile::write(&backup, &current, Some(&metadata))?;
    // Detect changes made while staging. A second writer racing the final
    // replacement remains an unresolved filesystem-level limitation.
    regular_writable(path)?;
    if read_document(path)? != expected {
        return Err(io::Error::other(
            "File changed outside NorthStar; use Ctrl-Shift-S to save a copy",
        ));
    }
    if backup_exists {
        replace_existing(&backup, &staged_backup.path)?;
    } else {
        fs::hard_link(&staged_backup.path, &backup)?;
    }
    // If this fails, the active document is still backed up before its
    // replacement was attempted. Do not advance the editor's saved checkpoint.
    replace_existing(path, &staged_document.path)
}

static NEXT_STAGE: AtomicU64 = AtomicU64::new(0);

pub(crate) struct StagedFile {
    pub(crate) path: PathBuf,
}

impl StagedFile {
    pub(crate) fn write(
        destination: &Path,
        bytes: &[u8],
        metadata: Option<&fs::Metadata>,
    ) -> io::Result<Self> {
        let parent = destination
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        for _ in 0..100 {
            let number = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(".northstar-{}-{number}.tmp", std::process::id()));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let stage = Self { path };
                    file.write_all(bytes)?;
                    if let Some(metadata) = metadata {
                        file.set_permissions(metadata.permissions())?;
                    }
                    file.sync_all()?;
                    return Ok(stage);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Could not create a staging file",
        ))
    }
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(windows)]
pub(crate) fn replace_existing(destination: &Path, staged: &Path) -> io::Result<()> {
    use std::{os::windows::ffi::OsStrExt, ptr};
    use windows_sys::Win32::Storage::FileSystem::ReplaceFileW;
    let wide = |path: &Path| {
        path.as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect::<Vec<_>>()
    };
    let destination = wide(destination);
    let staged = wide(staged);
    // Both files are on the same volume. A separate, synced .BAK already
    // contains the previous document before this call.
    if unsafe {
        ReplaceFileW(
            destination.as_ptr(),
            staged.as_ptr(),
            ptr::null(),
            0,
            ptr::null(),
            ptr::null(),
        )
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(windows))]
pub(crate) fn replace_existing(destination: &Path, staged: &Path) -> io::Result<()> {
    fs::rename(staged, destination)
}
