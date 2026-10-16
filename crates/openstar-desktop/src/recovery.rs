// SPDX-License-Identifier: GPL-3.0-only
//! Separate, bounded snapshots of unsaved UTF-8 and legacy drafts.

use crate::session::{
    DOCUMENT_LIMIT, Session, StagedFile, Utf8Session, read_document, replace_existing,
};
use openstar_core::{Document, Editor, Formatting, Utf8Editor};
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
const LEGACY_RECORD_VERSION: u8 = 2;
const RECORD_LIMIT: u64 = 16 * 1024 * 1024;
const FILE_LIMIT: usize = 128;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRecord {
    version: u8,
    path: Option<PathBuf>,
    opening_dir: Option<PathBuf>,
    resolved_path: Option<PathBuf>,
    baseline_sha256: Option<String>,
    saved_bytes: Vec<u8>,
    bytes: Vec<u8>,
    bytes_sha256: String,
    cursor_byte: usize,
    insert: bool,
    formatting: Formatting,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum RecoveryRecord {
    Utf8(Record),
    Legacy(LegacyRecord),
}

impl RecoveryRecord {
    fn resolved_path(&self) -> Option<&Path> {
        match self {
            Self::Utf8(record) => record.resolved_path.as_deref(),
            Self::Legacy(record) => record.resolved_path.as_deref(),
        }
    }

    fn current_bytes(&self) -> &[u8] {
        match self {
            Self::Utf8(record) => record.text.as_bytes(),
            Self::Legacy(record) => &record.bytes,
        }
    }
}

pub struct RecoveryCandidate {
    pub id: String,
    pub path: Option<PathBuf>,
    record: RecoveryRecord,
}

impl RecoveryCandidate {
    pub fn is_legacy(&self) -> bool {
        matches!(self.record, RecoveryRecord::Legacy(_))
    }
}

pub struct RecoveryScan {
    pub candidate: Option<RecoveryCandidate>,
    pub invalid_files: usize,
}

pub struct RecoveryOutcome {
    pub session: Utf8Session,
    pub detached: bool,
}

pub struct LegacyRecoveryOutcome {
    pub session: Session,
    pub detached: bool,
}

pub struct RecoveryStore {
    root: PathBuf,
    previous_root: Option<PathBuf>,
}

impl RecoveryStore {
    pub fn default_location() -> io::Result<Self> {
        #[cfg(windows)]
        let base = env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "LOCALAPPDATA is missing"))?;
        #[cfg(not(windows))]
        let base = match env::var_os("XDG_STATE_HOME") {
            Some(path) => PathBuf::from(path),
            None => env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "HOME is missing"))?
                .join(".local/state"),
        };
        #[cfg(windows)]
        let (root, previous_root) = (
            base.join("OpenStar-Recovery"),
            base.join("NorthStar-Recovery"),
        );
        #[cfg(not(windows))]
        let (root, previous_root) = (
            base.join("openstar/recovery"),
            base.join("northstar/recovery"),
        );
        Ok(Self {
            root,
            previous_root: Some(previous_root),
        })
    }

    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            previous_root: None,
        }
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
        let (opening_dir, resolved_path) = recorded_path(session.path.as_deref())?;
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
        self.publish(id, &RecoveryRecord::Utf8(record))
    }

    pub fn write_legacy(&self, id: &str, session: &Session) -> io::Result<()> {
        let (opening_dir, resolved_path) = recorded_path(session.path.as_deref())?;
        let snapshot = session.editor.snapshot();
        let bytes = session.editor.document().bytes();
        let record = LegacyRecord {
            version: LEGACY_RECORD_VERSION,
            path: session.path.clone(),
            opening_dir,
            resolved_path,
            baseline_sha256: session.baseline().map(digest),
            saved_bytes: session.editor.saved_bytes().to_vec(),
            bytes: bytes.to_vec(),
            bytes_sha256: digest(bytes),
            cursor_byte: snapshot.cursor_byte,
            insert: snapshot.insert,
            formatting: snapshot.formatting,
        };
        self.publish(id, &RecoveryRecord::Legacy(record))
    }

    fn publish(&self, id: &str, record: &RecoveryRecord) -> io::Result<()> {
        let destination = self.record_path(id)?;
        let bytes = serde_json::to_vec(&record).map_err(io::Error::other)?;
        if bytes.len() as u64 > RECORD_LIMIT {
            return Err(io::Error::other("Recovery snapshot exceeds 16 MiB"));
        }
        fs::create_dir_all(&self.root)?;
        Self::check_root(&self.root)?;
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
        self.find_matching(path, None)
    }

    pub fn find_for_mode(&self, path: &Path, legacy: bool) -> io::Result<RecoveryScan> {
        self.find_matching(Some(path), Some(legacy))
    }

    fn find_matching(&self, path: Option<&Path>, legacy: Option<bool>) -> io::Result<RecoveryScan> {
        let resolved = path.map(resolve_path).transpose()?;
        let mut newest: Option<(SystemTime, RecoveryCandidate)> = None;
        let mut files = 0;
        let mut invalid_files = 0;
        for root in std::iter::once(&self.root).chain(self.previous_root.iter()) {
            match fs::symlink_metadata(root) {
                Ok(_) => Self::check_root(root)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
            for entry in fs::read_dir(root)? {
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
                        if legacy.is_none_or(|selected| candidate.is_legacy() == selected)
                            && resolved.as_deref().is_none_or(|selected| {
                                candidate.record.resolved_path() == Some(selected)
                            })
                            && !candidate.record.resolved_path().is_some_and(|source| {
                                read_document(source)
                                    .ok()
                                    .is_some_and(|bytes| bytes == candidate.record.current_bytes())
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
        }
        Ok(RecoveryScan {
            candidate: newest.map(|(_, candidate)| candidate),
            invalid_files,
        })
    }

    pub fn restore(&self, candidate: &RecoveryCandidate) -> io::Result<RecoveryOutcome> {
        let RecoveryRecord::Utf8(record) = &candidate.record else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Not a UTF-8 recovery copy",
            ));
        };
        let (path, baseline) = restore_binding(
            record.resolved_path.as_deref(),
            record.baseline_sha256.as_deref(),
        );
        let detached = path.is_none();
        let session = Utf8Session::recover(
            path,
            baseline,
            record.text.clone(),
            record.saved_text.clone(),
            record.cursor_byte,
        )?;
        Ok(RecoveryOutcome { session, detached })
    }

    pub fn restore_legacy(
        &self,
        candidate: &RecoveryCandidate,
    ) -> io::Result<LegacyRecoveryOutcome> {
        let RecoveryRecord::Legacy(record) = &candidate.record else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Not a legacy recovery copy",
            ));
        };
        let (path, baseline) = restore_binding(
            record.resolved_path.as_deref(),
            record.baseline_sha256.as_deref(),
        );
        let detached = path.is_none();
        let session = Session::recover(
            path,
            baseline,
            record.bytes.clone(),
            record.saved_bytes.clone(),
            record.cursor_byte,
            record.insert,
            record.formatting,
        )?;
        Ok(LegacyRecoveryOutcome { session, detached })
    }

    pub fn remove(&self, id: &str) -> io::Result<()> {
        let filename = self.record_path(id)?.file_name().unwrap().to_owned();
        for root in std::iter::once(&self.root).chain(self.previous_root.iter()) {
            match fs::symlink_metadata(root) {
                Ok(_) => Self::check_root(root)?,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
            match fs::remove_file(root.join(&filename)) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn check_root(root: &Path) -> io::Result<()> {
        let metadata = fs::symlink_metadata(root)?;
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
        let record: RecoveryRecord = serde_json::from_slice(&bytes)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let valid = match &record {
            RecoveryRecord::Utf8(record) => self.valid_utf8_record(record),
            RecoveryRecord::Legacy(record) => self.valid_legacy_record(record),
        };
        if !valid {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid recovery contents",
            ));
        }
        let path = match &record {
            RecoveryRecord::Utf8(record) => record.path.clone(),
            RecoveryRecord::Legacy(record) => record.path.clone(),
        };
        Ok(RecoveryCandidate { id, path, record })
    }

    fn valid_utf8_record(&self, record: &Record) -> bool {
        !(record.version != RECORD_VERSION
            || record.text.len() > DOCUMENT_LIMIT
            || record.saved_text.len() > DOCUMENT_LIMIT
            || record.text == record.saved_text
            || record.text_sha256 != digest(record.text.as_bytes())
            || !valid_recorded_path(
                record.path.as_deref(),
                record.opening_dir.as_deref(),
                record.resolved_path.as_deref(),
                record.baseline_sha256.as_deref(),
            )
            || record
                .baseline_sha256
                .as_ref()
                .is_some_and(|expected| *expected != digest(record.saved_text.as_bytes()))
            || Utf8Editor::recover(
                record.text.clone(),
                record.saved_text.clone(),
                record.cursor_byte,
            )
            .is_err())
    }

    fn valid_legacy_record(&self, record: &LegacyRecord) -> bool {
        record.version == LEGACY_RECORD_VERSION
            && record.bytes.len() <= DOCUMENT_LIMIT
            && record.saved_bytes.len() <= DOCUMENT_LIMIT
            && record.bytes != record.saved_bytes
            && record.bytes_sha256 == digest(&record.bytes)
            && valid_recorded_path(
                record.path.as_deref(),
                record.opening_dir.as_deref(),
                record.resolved_path.as_deref(),
                record.baseline_sha256.as_deref(),
            )
            && record
                .baseline_sha256
                .as_ref()
                .is_none_or(|expected| *expected == digest(&record.saved_bytes))
            && Editor::recover(
                Document::from_bytes(record.bytes.clone()),
                record.saved_bytes.clone(),
                record.cursor_byte,
                record.insert,
                record.formatting,
            )
            .is_ok()
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

fn recorded_path(path: Option<&Path>) -> io::Result<(Option<PathBuf>, Option<PathBuf>)> {
    let opening_dir = path
        .filter(|path| !path.is_absolute())
        .map(|_| env::current_dir())
        .transpose()?;
    let resolved_path = path.map(|path| {
        opening_dir
            .as_ref()
            .map_or_else(|| path.to_owned(), |dir| dir.join(path))
    });
    Ok((opening_dir, resolved_path))
}

fn valid_recorded_path(
    path: Option<&Path>,
    opening_dir: Option<&Path>,
    resolved_path: Option<&Path>,
    baseline_sha256: Option<&str>,
) -> bool {
    match (path, opening_dir, resolved_path) {
        (None, None, None) => baseline_sha256.is_none(),
        (Some(path), None, Some(resolved)) if path.is_absolute() => resolved == path,
        (Some(path), Some(dir), Some(resolved)) if !path.is_absolute() && dir.is_absolute() => {
            resolved == dir.join(path)
        }
        _ => false,
    }
}

fn restore_binding(
    resolved_path: Option<&Path>,
    baseline_sha256: Option<&str>,
) -> (Option<PathBuf>, Option<Vec<u8>>) {
    match (resolved_path, baseline_sha256) {
        (Some(path), Some(expected)) => match read_document(path) {
            Ok(bytes) if digest(&bytes) == expected => (Some(path.to_owned()), Some(bytes)),
            _ => (None, None),
        },
        (Some(path), None) if matches!(fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound) => {
            (Some(path.to_owned()), None)
        }
        _ => (None, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renamed_store_reads_and_clears_previous_recovery_records() {
        let base = env::temp_dir().join(format!("openstar-recovery-{}", RecoveryStore::new_id()));
        let previous = base.join("previous");
        let current = base.join("current");
        let mut session = Utf8Session::new_at(None);
        session.editor.insert_text("unfinished").unwrap();
        let id = RecoveryStore::new_id();
        RecoveryStore::new(previous.clone())
            .write(&id, &session)
            .unwrap();
        let store = RecoveryStore {
            root: current.clone(),
            previous_root: Some(previous.clone()),
        };
        let candidate = store.find(None).unwrap().candidate.unwrap();
        assert_eq!(candidate.id, id);
        assert_eq!(
            store.restore(&candidate).unwrap().session.editor.text(),
            "unfinished"
        );
        session.editor.insert_text(" work").unwrap();
        store.write(&id, &session).unwrap();
        assert!(previous.join(format!("{id}.json")).exists());
        assert!(current.join(format!("{id}.json")).exists());
        store.remove(&id).unwrap();
        assert!(store.find(None).unwrap().candidate.is_none());
        fs::remove_dir_all(&base).unwrap();
    }
}
