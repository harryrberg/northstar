// SPDX-License-Identifier: GPL-3.0-only
use openstar_core::Command;
use openstar_desktop::{
    recovery::RecoveryStore,
    session::{Session, Utf8Session},
};
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
            "openstar-recovery-test-{}-{}",
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
    fs::write(root.join(".openstar-interrupted.tmp"), b"incomplete stage").unwrap();
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

#[test]
fn interrupted_wordstar_edit_restores_exact_carrier_and_saves_with_backup() {
    let scratch = Scratch::new();
    let path = scratch.0.join("chapter.doc");
    let mut original = b"ALPHA\r\n".to_vec();
    original.extend_from_slice(&[0x9b, 0x7f, 0x1c]);
    original.extend_from_slice(b"\r\nBETA\x1a");
    fs::write(&path, &original).unwrap();
    let mut interrupted = Session::open(&path).unwrap();
    interrupted
        .editor
        .apply(&Command::InsertAscii { text: "X".into() })
        .unwrap();
    let expected = interrupted.editor.document().bytes().to_vec();
    let cursor = interrupted.editor.snapshot().cursor_byte;
    let store = RecoveryStore::new(scratch.0.join("recovery"));
    let id = RecoveryStore::new_id();
    store.write_legacy(&id, &interrupted).unwrap();
    drop(interrupted);
    assert_eq!(fs::read(&path).unwrap(), original);

    let candidate = store.find_for_mode(&path, true).unwrap().candidate.unwrap();
    assert!(candidate.is_legacy());
    let mut outcome = store.restore_legacy(&candidate).unwrap();
    assert!(!outcome.detached);
    assert_eq!(outcome.session.editor.document().bytes(), expected);
    assert_eq!(outcome.session.editor.saved_bytes(), original);
    assert_eq!(outcome.session.editor.snapshot().cursor_byte, cursor);
    assert!(outcome.session.editor.snapshot().modified);
    outcome.session.save().unwrap();
    store.remove(&id).unwrap();
    assert_eq!(fs::read(&path).unwrap(), expected);
    assert_eq!(fs::read(scratch.0.join("chapter.BAK")).unwrap(), original);
    assert!(
        store
            .find_for_mode(&path, true)
            .unwrap()
            .candidate
            .is_none()
    );
}

#[test]
fn changed_or_missing_wordstar_source_requires_save_as() {
    let scratch = Scratch::new();
    let path = scratch.0.join("draft.doc");
    let original = b"ALPHA\x1a";
    fs::write(&path, original).unwrap();
    let mut session = Session::open(&path).unwrap();
    session
        .editor
        .apply(&Command::InsertAscii { text: "Z".into() })
        .unwrap();
    let expected = session.editor.document().bytes().to_vec();
    let store = RecoveryStore::new(scratch.0.join("recovery"));
    store
        .write_legacy(&RecoveryStore::new_id(), &session)
        .unwrap();
    fs::write(&path, b"CHANGED\x1a").unwrap();

    let candidate = store.find_for_mode(&path, true).unwrap().candidate.unwrap();
    let mut outcome = store.restore_legacy(&candidate).unwrap();
    assert!(outcome.detached);
    assert!(outcome.session.path.is_none());
    assert_eq!(outcome.session.editor.document().bytes(), expected);
    assert!(outcome.session.save().is_err());
    let copy = scratch.0.join("recovered.doc");
    outcome.session.save_as(&copy).unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"CHANGED\x1a");
    assert_eq!(fs::read(&copy).unwrap(), expected);

    fs::remove_file(&path).unwrap();
    let outcome = store.restore_legacy(&candidate).unwrap();
    assert!(outcome.detached);
    assert!(outcome.session.path.is_none());
}

#[test]
fn malformed_wordstar_recovery_is_retained_and_utf8_records_still_open() {
    let scratch = Scratch::new();
    let root = scratch.0.join("recovery");
    let store = RecoveryStore::new(root.clone());
    let path = scratch.0.join("draft.doc");
    fs::write(&path, b"A\x1a").unwrap();
    let mut session = Session::open(&path).unwrap();
    session
        .editor
        .apply(&Command::InsertAscii { text: "B".into() })
        .unwrap();
    let id = RecoveryStore::new_id();
    store.write_legacy(&id, &session).unwrap();
    let record_path = root.join(format!("{id}.json"));
    let mut record: serde_json::Value =
        serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
    record["bytes"][0] = serde_json::json!(255);
    fs::write(&record_path, serde_json::to_vec(&record).unwrap()).unwrap();
    let scan = store.find_for_mode(&path, true).unwrap();
    assert_eq!(scan.invalid_files, 1);
    assert!(scan.candidate.is_none());
    assert!(record_path.exists());

    let mut utf8 = Utf8Session::new_at(None);
    utf8.editor.insert_text("Utkast").unwrap();
    store.write(&RecoveryStore::new_id(), &utf8).unwrap();
    let candidate = store.find(None).unwrap().candidate.unwrap();
    assert!(!candidate.is_legacy());
    assert_eq!(
        store.restore(&candidate).unwrap().session.editor.text(),
        "Utkast"
    );
}

#[test]
fn recovery_lookup_keeps_document_modes_separate_at_the_same_path() {
    let scratch = Scratch::new();
    let path = scratch.0.join("draft.txt");
    fs::write(&path, b"ALPHA").unwrap();
    let store = RecoveryStore::new(scratch.0.join("recovery"));
    let mut legacy = Session::open(&path).unwrap();
    legacy
        .editor
        .apply(&Command::InsertAscii { text: "X".into() })
        .unwrap();
    store
        .write_legacy(&RecoveryStore::new_id(), &legacy)
        .unwrap();
    let mut utf8 = Utf8Session::open(&path).unwrap();
    utf8.editor.insert_text("Y").unwrap();
    store.write(&RecoveryStore::new_id(), &utf8).unwrap();

    let legacy_candidate = store.find_for_mode(&path, true).unwrap().candidate.unwrap();
    let utf8_candidate = store
        .find_for_mode(&path, false)
        .unwrap()
        .candidate
        .unwrap();
    assert!(legacy_candidate.is_legacy());
    assert!(!utf8_candidate.is_legacy());
    let mut expected = b"XALPHA".to_vec();
    expected.resize(128, 0x1a);
    assert_eq!(
        store
            .restore_legacy(&legacy_candidate)
            .unwrap()
            .session
            .editor
            .document()
            .bytes(),
        expected
    );
    assert_eq!(
        store
            .restore(&utf8_candidate)
            .unwrap()
            .session
            .editor
            .text(),
        "YALPHA"
    );
    assert_eq!(fs::read(path).unwrap(), b"ALPHA");
}

#[test]
fn unsaved_new_wordstar_file_can_recover_at_its_unused_path() {
    let scratch = Scratch::new();
    let path = scratch.0.join("new.doc");
    let mut session = Session::new_at(path.clone());
    session
        .editor
        .apply(&Command::InsertAscii { text: "NEW".into() })
        .unwrap();
    let expected = session.editor.document().bytes().to_vec();
    let store = RecoveryStore::new(scratch.0.join("recovery"));
    store
        .write_legacy(&RecoveryStore::new_id(), &session)
        .unwrap();
    assert!(!path.exists());

    let candidate = store.find_for_mode(&path, true).unwrap().candidate.unwrap();
    let mut outcome = store.restore_legacy(&candidate).unwrap();
    assert!(!outcome.detached);
    assert_eq!(outcome.session.path.as_deref(), Some(path.as_path()));
    outcome.session.save().unwrap();
    assert_eq!(fs::read(path).unwrap(), expected);
}
