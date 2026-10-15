// SPDX-License-Identifier: GPL-3.0-only
//! Separate, bounded snapshots of unsaved UTF-8 drafts.

use crate::session::{DOCUMENT_LIMIT, StagedFile, Utf8Session, read_document, replace_existing};
use northstar_core::Utf8Editor;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const RECORD_VERSION: u8 = 1;
const RECORD_LIMIT: u64 = 16 * 1024 * 1024;
const FILE_LIMIT: usize = 128;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize, Deserialize)]
struct Record {
    version: u8,
    path: Option<PathBuf>,
    opening_dir: Option<PathBuf>,
    resolved_path: Option<PathBuf>,
    baseline_sha256: Option<String>,
    saved_text: String,
    text: String,
    text_sha256: String,
    cursor_byte: usize,
}

pub struct RecoveryCandidate {
    pub id: String,
    pub path: Option<PathBuf>,
    record: Record,
}

pub struct RecoveryScan {
    pub candidate: Option<RecoveryCandidate>,
    pub invalid_files: usize,
}

pub struct RecoveryOutcome {
    pub session: Utf8Session,
    pub detached: bool,
}

pub struct RecoveryStore {
    root: PathBuf,
}

impl RecoveryStore {
    pub fn default_location() -> io::Result<Self> {
        #[cfg(windows)]
        let root = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is missing"))?
            .join("NorthStar-Recovery");
        #[cfg(not(windows))]
        let root = match env::var_os("XDG_STATE_HOME") {
            Some(path) => PathBuf::from(path),
            None => env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is missing"))?
                .join(".local/state"),
        }
        .join("northstar/recovery");
        Ok(Self { root })
    }

    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn new_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!(
            "recovery-{nanos:x}-{:x}-{:x}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        )
    }

    pub fn write(&self, id: &str, session: &Utf8Session) -> io::Result<()> {
        let destination = self.record_path(id)?;
        let opening_dir = session
            .path
            .as_ref()
            .filter(|path| !path.is_absolute())
            .map(|_| env::current_dir())
            .transpose()?;
        let resolved_path = session.path.as_ref().map(|path| {
            opening_dir
                .as_ref()
                .map_or_else(|| path.clone(), |dir| dir.join(path))
        });
        let record = Record {
            version: RECORD_VERSION,
            path: session.path.clone(),
            opening_dir,
            resolved_path,
            baseline_sha256: session.baseline().map(digest),
            saved_text: session.editor.saved_text().to_owned(),
            text: session.editor.text().to_owned(),
            text_sha256: digest(session.editor.text().as_bytes()),
            cursor_byte: session.editor.snapshot().cursor_byte,
        };
        let bytes = serde_json::to_vec(&record).map_err(io::Error::other)?;
        if bytes.len() as u64 > RECORD_LIMIT {
            return Err(io::Error::other("Recovery snapshot exceeds 16 MiB"));
        }
        fs::create_dir_all(&self.root)?;
        self.check_root()?;
        let staged = StagedFile::write(&destination, &bytes, None)?;
        match fs::symlink_metadata(&destination) {
            Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => {
                replace_existing(&destination, &staged.path)
            }
            Ok(_) => Err(io::Error::other("Recovery target is not a regular file")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::hard_link(&staged.path, &destination)
            }
            Err(error) => Err(error),
        }
    }

    pub fn find(&self, path: Option<&Path>) -> io::Result<RecoveryScan> {
        let resolved = path.map(resolve_path).transpose()?;
        if self.root.exists() {
            self.check_root()?;
        }
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(RecoveryScan {
                    candidate: None,
                    invalid_files: 0,
                });
            }
            Err(error) => return Err(error),
        };
        let mut newest: Option<(SystemTime, RecoveryCandidate)> = None;
        let mut files = 0;
        let mut invalid_files = 0;
        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            files += 1;
            if files > FILE_LIMIT {
                return Err(io::Error::other(
                    "Too many recovery files; inspect the recovery directory",
                ));
            }
            match self.read_candidate(&entry_path) {
                Ok(candidate)
                    if resolved.as_deref().is_none_or(|selected| {
                        candidate.record.resolved_path.as_deref() == Some(selected)
                    }) && !candidate
                        .record
                        .resolved_path
                        .as_ref()
                        .is_some_and(|source| {
                            read_document(source)
                                .ok()
                                .is_some_and(|bytes| bytes == candidate.record.text.as_bytes())
                        }) =>
                {
                    let modified = entry.metadata()?.modified().unwrap_or(UNIX_EPOCH);
                    if newest.as_ref().is_none_or(|(time, _)| modified > *time) {
                        newest = Some((modified, candidate));
                    }
                }
                Ok(_) => {}
                Err(_) => invalid_files += 1,
            }
        }
        Ok(RecoveryScan {
            candidate: newest.map(|(_, candidate)| candidate),
            invalid_files,
        })
    }

    pub fn restore(&self, candidate: &RecoveryCandidate) -> io::Result<RecoveryOutcome> {
        let record = &candidate.record;
        let mut session = match (&record.resolved_path, &record.baseline_sha256) {
            (Some(path), Some(expected)) => match Utf8Session::open(path) {
                Ok(opened)
                    if opened
                        .baseline()
                        .is_some_and(|bytes| digest(bytes) == *expected) =>
                {
                    opened
                }
                _ => Utf8Session::new_at(None),
            },
            (Some(path), None) if matches!(fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound) => {
                Utf8Session::new_at(Some(path.clone()))
            }
            _ => Utf8Session::new_at(None),
        };
        session.editor = Utf8Editor::recover(
            record.text.clone(),
            record.saved_text.clone(),
            record.cursor_byte,
        )
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let detached = session.path.is_none();
        Ok(RecoveryOutcome { session, detached })
    }

    pub fn remove(&self, id: &str) -> io::Result<()> {
        if self.root.exists() {
            self.check_root()?;
        }
        let path = self.record_path(id)?;
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn check_root(&self) -> io::Result<()> {
        let metadata = fs::symlink_metadata(&self.root)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(io::Error::other(
                "Recovery directory is not a regular directory",
            ));
        }
        Ok(())
    }

    fn record_path(&self, id: &str) -> io::Result<PathBuf> {
        if !id.starts_with("recovery-")
            || id.len() == "recovery-".len()
            || !id["recovery-".len()..]
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || byte == b'-')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid recovery identifier",
            ));
        }
        Ok(self.root.join(format!("{id}.json")))
    }

    fn read_candidate(&self, path: &Path) -> io::Result<RecoveryCandidate> {
        let metadata = fs::symlink_metadata(path)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > RECORD_LIMIT
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid recovery file",
            ));
        }
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid recovery name"))?
            .to_owned();
        self.record_path(&id)?;
        let mut bytes = Vec::new();
        File::open(path)?
            .take(RECORD_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > RECORD_LIMIT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Recovery file is too large",
            ));
        }
        let record: Record = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if record.version != RECORD_VERSION
            || record.text.len() > DOCUMENT_LIMIT
            || record.saved_text.len() > DOCUMENT_LIMIT
            || record.text == record.saved_text
            || record.text_sha256 != digest(record.text.as_bytes())
            || record.path.is_some() != record.resolved_path.is_some()
            || (record.path.is_none() && record.baseline_sha256.is_some())
            || record.path.as_ref().is_some_and(|path| {
                record.resolved_path.as_ref()
                    != Some(
                        &record
                            .opening_dir
                            .as_ref()
                            .map_or_else(|| path.clone(), |dir| dir.join(path)),
                    )
                    || record.opening_dir.is_some() == path.is_absolute()
            })
            || (record.path.is_none() && record.opening_dir.is_some())
            || record
                .resolved_path
                .as_ref()
                .is_some_and(|resolved| !resolved.is_absolute())
            || record
                .baseline_sha256
                .as_ref()
                .is_some_and(|expected| *expected != digest(record.saved_text.as_bytes()))
            || Utf8Editor::recover(
                record.text.clone(),
                record.saved_text.clone(),
                record.cursor_byte,
            )
            .is_err()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid recovery contents",
            ));
        }
        Ok(RecoveryCandidate {
            id,
            path: record.path.clone(),
            record,
        })
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn resolve_path(path: &Path) -> io::Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}
