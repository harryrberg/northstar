// SPDX-License-Identifier: GPL-3.0-only
use northstar_desktop::{recovery::RecoveryStore, session::Utf8Session};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static SERIAL: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "northstar-recovery-test-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn interrupted_utf8_edit_restores_exact_text_cursor_and_save_path() {
    let scratch = Scratch::new();
    let path = scratch.0.join("chapter.txt");
    let original = "Første linje\r\nCafe\u{301}";
    fs::write(&path, original).unwrap();
    let mut interrupted = Utf8Session::open(&path).unwrap();
    interrupted.editor.insert_text("Blå ").unwrap();
    let cursor = interrupted.editor.snapshot().cursor_byte;
    let recovered_bytes = interrupted.editor.text().as_bytes().to_vec();
    let store_path = scratch.0.join("recovery");
    let store = RecoveryStore::new(store_path.clone());
    let id = RecoveryStore::new_id();
    store.write(&id, &interrupted).unwrap();
    drop(interrupted);
    assert_eq!(fs::read(&path).unwrap(), original.as_bytes());

    let restarted = RecoveryStore::new(store_path);
    let scan = restarted.find(Some(&path)).unwrap();
    assert_eq!(scan.invalid_files, 0);
    let candidate = scan.candidate.unwrap();
    assert_eq!(candidate.id, id);
    let mut outcome = restarted.restore(&candidate).unwrap();
    assert!(!outcome.detached);
    assert_eq!(outcome.session.path.as_deref(), Some(path.as_path()));
    assert!(outcome.session.editor.snapshot().modified);
    assert_eq!(outcome.session.editor.snapshot().cursor_byte, cursor);
    assert_eq!(outcome.session.editor.text().as_bytes(), recovered_bytes);
    outcome.session.save().unwrap();
    restarted.remove(&id).unwrap();
    assert!(restarted.find(Some(&path)).unwrap().candidate.is_none());
    assert_eq!(fs::read(&path).unwrap(), recovered_bytes);
    assert_eq!(
        fs::read(scratch.0.join("chapter.txt.bak")).unwrap(),
        original.as_bytes()
    );
}

#[test]
fn changed_or_missing_original_detaches_recovered_copy() {
    let scratch = Scratch::new();
    let path = scratch.0.join("draft.txt");
    fs::write(&path, "Original æøå").unwrap();
    let mut interrupted = Utf8Session::open(&path).unwrap();
    interrupted.editor.insert_text("Ny ").unwrap();
    let store = RecoveryStore::new(scratch.0.join("recovery"));
    let id = RecoveryStore::new_id();
    store.write(&id, &interrupted).unwrap();
    fs::write(&path, "Changed elsewhere").unwrap();

    let candidate = store.find(Some(&path)).unwrap().candidate.unwrap();
    let mut outcome = store.restore(&candidate).unwrap();
    assert!(outcome.detached);
    assert!(outcome.session.path.is_none());
    assert!(outcome.session.editor.snapshot().modified);
    assert_eq!(outcome.session.editor.text(), "Ny Original æøå");
    let copy = scratch.0.join("recovered.txt");
    outcome.session.save_as(&copy).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"Changed elsewhere");
    assert_eq!(fs::read(&copy).unwrap(), "Ny Original æøå".as_bytes());

    fs::remove_file(&path).unwrap();
    let outcome = store.restore(&candidate).unwrap();
    assert!(outcome.detached);
    assert!(outcome.session.path.is_none());
    assert!(!path.exists());
}

#[test]
fn empty_recovered_draft_stays_dirty_and_truncated_records_are_retained() {
    let scratch = Scratch::new();
    let path = scratch.0.join("draft.txt");
    fs::write(&path, "X").unwrap();
    let mut interrupted = Utf8Session::open(&path).unwrap();
    interrupted.editor.delete();
    assert!(interrupted.editor.snapshot().modified);
    let root = scratch.0.join("recovery");
    let store = RecoveryStore::new(root.clone());
    let id = RecoveryStore::new_id();
    store.write(&id, &interrupted).unwrap();
    fs::write(&path, "Changed elsewhere").unwrap();
    fs::write(root.join("recovery-deadbeef.json"), b"{\"version\":1").unwrap();
    fs::write(root.join(".northstar-interrupted.tmp"), b"incomplete stage").unwrap();
    let scan = store.find(None).unwrap();
    assert_eq!(scan.invalid_files, 1);
    let candidate = scan.candidate.unwrap();
    assert_eq!(candidate.id, id);
    let outcome = store.restore(&candidate).unwrap();
    assert!(outcome.detached);
    assert!(outcome.session.editor.text().is_empty());
    assert!(outcome.session.editor.snapshot().modified);
    assert_eq!(fs::read(&path).unwrap(), b"Changed elsewhere");
    assert!(root.join("recovery-deadbeef.json").exists());
}

#[test]
fn relative_source_path_is_resolved_for_restart_without_creating_the_source() {
    let scratch = Scratch::new();
    let id = RecoveryStore::new_id();
    let relative = PathBuf::from(format!("draft-{id}.txt"));
    let mut session = Utf8Session::new_at(Some(relative.clone()));
    session.editor.insert_text("Ulagret tekst").unwrap();
    let store = RecoveryStore::new(scratch.0.join("recovery"));
    store.write(&id, &session).unwrap();
    let candidate = store.find(Some(&relative)).unwrap().candidate.unwrap();
    assert_eq!(candidate.path.as_deref(), Some(relative.as_path()));
    let outcome = store.restore(&candidate).unwrap();
    let expected = std::env::current_dir().unwrap().join(&relative);
    assert_eq!(outcome.session.path.as_deref(), Some(expected.as_path()));
    assert_eq!(outcome.session.editor.text(), "Ulagret tekst");
    assert!(!expected.exists());
}
