// SPDX-License-Identifier: GPL-3.0-only
use iced::keyboard::{
    Event, Key, Location, Modifiers,
    key::{Named, NativeCode, Physical},
};
use northstar_core::{Command, Document, Editor};
use northstar_desktop::{
    input::{Action, Keyboard},
    projection::Projection,
    session::{DOCUMENT_LIMIT, DocumentSession, Session, Utf8Session},
};
use sha2::{Digest, Sha256};
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
            "northstar-desktop-{}-{}",
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

fn press(key: Key, modifiers: Modifiers, text: Option<&str>, repeat: bool) -> Event {
    Event::KeyPressed {
        modified_key: key.clone(),
        key,
        physical_key: Physical::Unidentified(NativeCode::Unidentified),
        location: Location::Standard,
        modifiers,
        text: text.map(Into::into),
        repeat,
    }
}
fn ctrl(letter: &str, repeat: bool) -> Event {
    press(Key::Character(letter.into()), Modifiers::CTRL, None, repeat)
}
fn type_text(text: &str) -> Event {
    press(Key::Unidentified, Modifiers::empty(), Some(text), false)
}
fn dispatch(keys: &mut Keyboard, session: &mut Session, event: Event) {
    if let Some(action) = keys.handle(&event) {
        let Action::Edit(command) = action else {
            panic!("unexpected action: {action:?}")
        };
        session.editor.apply(&command).unwrap();
    }
}

#[test]
fn desktop_key_sequences_save_the_retained_dos_results() {
    let scratch = Scratch::new();
    for name in [
        "edit-reformat-edit-split-chain",
        "wrap-type-multiple-wrapped-lines",
    ] {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/oracles")
            .join(format!("{name}.json"));
        let case: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let hex = case["initial"]["carrier_hex"].as_str().unwrap();
        let carrier: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();
        let source = scratch.0.join(format!("{name}-input.doc"));
        fs::write(&source, &carrier).unwrap();
        let mut session = Session::open(&source).unwrap();
        let mut keyboard = Keyboard::default();
        for value in case["commands"].as_array().unwrap() {
            let command: Command = serde_json::from_value(value.clone()).unwrap();
            match command {
                Command::DocumentStart => {
                    dispatch(&mut keyboard, &mut session, ctrl("q", false));
                    dispatch(
                        &mut keyboard,
                        &mut session,
                        press(
                            Key::Character("r".into()),
                            Modifiers::empty(),
                            Some("r"),
                            false,
                        ),
                    );
                }
                Command::InsertAscii { text } => {
                    for ch in text.chars() {
                        dispatch(&mut keyboard, &mut session, type_text(&ch.to_string()));
                    }
                }
                Command::Reformat => dispatch(&mut keyboard, &mut session, ctrl("b", false)),
                Command::ToggleInsert => dispatch(&mut keyboard, &mut session, ctrl("v", false)),
                Command::NextCharacter => dispatch(&mut keyboard, &mut session, ctrl("d", false)),
                Command::PreviousCharacter => {
                    dispatch(&mut keyboard, &mut session, ctrl("s", false))
                }
                Command::Enter => dispatch(
                    &mut keyboard,
                    &mut session,
                    press(Key::Named(Named::Enter), Modifiers::empty(), None, false),
                ),
            }
        }
        let expected = &case["expected"];
        assert_eq!(
            session.editor.snapshot().cursor_byte as u64,
            expected["cursor_byte"].as_u64().unwrap()
        );
        assert_eq!(
            session.editor.snapshot().insert,
            expected["insert"].as_bool().unwrap()
        );
        let destination = scratch.0.join(format!("{name}-output.doc"));
        session.save_as(&destination).unwrap();
        assert!(!session.editor.snapshot().modified);
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(&destination).unwrap())),
            expected["carrier_sha256"].as_str().unwrap()
        );
        assert_eq!(
            Session::open(&destination)
                .unwrap()
                .editor
                .document()
                .bytes(),
            session.editor.document().bytes()
        );
        assert_eq!(fs::read(source).unwrap(), carrier);
    }
}

#[test]
fn save_as_preserves_unknown_bytes_and_refuses_to_overwrite() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.doc");
    let bytes = b"A\x9b\xff\x1c\x0d\x8a\x1a\x1aTAIL";
    fs::write(&source, bytes).unwrap();
    let mut session = Session::open(&source).unwrap();
    let copy = scratch.0.join("copy.doc");
    session.save_as(&copy).unwrap();
    assert_eq!(fs::read(&copy).unwrap(), bytes);
    assert_eq!(
        session.save_as(&source).unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(fs::read(source).unwrap(), bytes);
    assert_eq!(session.path.as_ref(), Some(&copy));
}

#[test]
fn failed_save_keeps_the_path_cursor_and_dirty_checkpoint() {
    let scratch = Scratch::new();
    let mut session = Session::default();
    session
        .editor
        .apply(&Command::InsertAscii { text: "ABC".into() })
        .unwrap();
    let before = session.editor.snapshot();
    assert!(
        session
            .save_as(&scratch.0.join("missing/failed.doc"))
            .is_err()
    );
    assert_eq!(before, session.editor.snapshot());
    assert!(session.path.is_none());
    session.save_as(&scratch.0.join("saved.doc")).unwrap();
    assert_eq!(session.editor.snapshot().cursor_byte, before.cursor_byte);
    assert!(!session.editor.snapshot().modified);
    session
        .editor
        .apply(&Command::InsertAscii { text: "D".into() })
        .unwrap();
    assert!(session.editor.snapshot().modified);
}

#[test]
fn repeat_prefix_reset_and_norwegian_input_do_not_corrupt_text() {
    let mut session = Session::default();
    let mut keyboard = Keyboard::default();
    dispatch(&mut keyboard, &mut session, type_text("ABC"));
    dispatch(&mut keyboard, &mut session, ctrl("v", false));
    dispatch(&mut keyboard, &mut session, ctrl("v", true));
    assert!(!session.editor.snapshot().insert);
    dispatch(&mut keyboard, &mut session, ctrl("s", false));
    dispatch(&mut keyboard, &mut session, ctrl("s", true));
    assert_eq!(session.editor.snapshot().cursor_byte, 1);
    dispatch(&mut keyboard, &mut session, ctrl("q", false));
    dispatch(&mut keyboard, &mut session, ctrl("q", true));
    assert!(keyboard.pending());
    keyboard.reset(); // Window focus loss is routed here by the application.
    dispatch(&mut keyboard, &mut session, type_text("R"));
    assert_eq!(&session.editor.document().bytes()[..3], b"ARC");
    let before = session.editor.snapshot();
    let carrier = session.editor.document().bytes().to_vec();
    let Some(Action::Edit(command)) = keyboard.handle(&type_text("æøåÆØÅ")) else {
        panic!()
    };
    assert!(session.editor.apply(&command).is_err());
    assert_eq!(session.editor.snapshot(), before);
    assert_eq!(session.editor.document().bytes(), carrier);
}

#[test]
fn projection_uses_token_boundaries_and_discloses_hidden_payload() {
    let editor = Editor::restore(
        Document::from_bytes(b"\xc1\xa0\x8d\x0aB\x1b\x82\x1c\x0d\x0a\x1a\x1aTAIL".to_vec()),
        8,
        true,
    )
    .unwrap();
    let projection = Projection::new(&editor);
    assert_eq!(
        projection
            .rows
            .iter()
            .map(|r| r.iter().map(|c| c.glyph).collect::<String>())
            .collect::<Vec<_>>(),
        ["A ↵", "B□¶", ""]
    );
    assert_eq!(projection.cursor, (1, 2));
    assert_eq!(projection.hidden_tail_bytes, 4);
}

#[test]
fn oversized_open_is_rejected_without_loading_a_partial_document() {
    let scratch = Scratch::new();
    let source = scratch.0.join("large.doc");
    fs::File::create(&source)
        .unwrap()
        .set_len(DOCUMENT_LIMIT as u64 + 1)
        .unwrap();
    assert!(Session::open(&source).is_err());
}

#[test]
fn utf8_keyboard_save_reopen_and_projection_preserve_text() {
    let scratch = Scratch::new();
    let mut session = DocumentSession::Utf8(Utf8Session::new_at(None));
    let mut keyboard = Keyboard::default();
    let Some(Action::Text(text)) = keyboard.handle_mode(&type_text("Blåbær og æøå"), true)
    else {
        panic!("modern text was not routed to the UTF-8 editor")
    };
    session.insert_text(&text).unwrap();
    session.apply_command(&Command::Enter).unwrap();
    assert_eq!(
        keyboard.handle_mode(&ctrl("v", false), true),
        Some(Action::Paste)
    );
    session.insert_text("Cafe\u{301} 👩‍💻").unwrap();
    assert!(session.modified());

    let destination = scratch.0.join("draft.txt");
    session.save_as(&destination).unwrap();
    assert!(!session.modified());
    assert_eq!(
        fs::read(&destination).unwrap(),
        "Blåbær og æøå\nCafe\u{301} 👩‍💻".as_bytes()
    );
    assert_eq!(
        session.save_as(&destination).unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );

    let reopened = Utf8Session::open(&destination).unwrap();
    assert_eq!(reopened.editor.text(), "Blåbær og æøå\nCafe\u{301} 👩‍💻");
    let projection = Projection::utf8(&reopened.editor);
    let rows = projection
        .rows
        .iter()
        .map(|row| row.iter().map(|c| c.glyph).collect::<String>())
        .collect::<Vec<_>>();
    assert_eq!(rows, ["Blåbær og æøå¶", "Café □"]);
}

#[test]
fn utf8_mode_is_explicit_and_rejects_invalid_files_without_touching_legacy_bytes() {
    let scratch = Scratch::new();
    let source = scratch.0.join("legacy.doc");
    let bytes = b"A\xff\x1a\x1a";
    fs::write(&source, bytes).unwrap();
    assert_eq!(
        Utf8Session::open(&source).err().unwrap().kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(
        Session::open(&source).unwrap().editor.document().bytes(),
        bytes
    );
    assert_eq!(fs::read(&source).unwrap(), bytes);
}

#[test]
fn utf8_size_limit_rejects_an_entire_insert_without_changing_the_document() {
    let mut session = DocumentSession::Utf8(Utf8Session::new_at(None));
    session.insert_text(&"a".repeat(DOCUMENT_LIMIT)).unwrap();
    assert!(session.insert_text("æ").is_err());
    assert!(
        session
            .apply_command(&Command::InsertAscii { text: "x".into() })
            .is_err()
    );
    let DocumentSession::Utf8(session) = session else {
        panic!()
    };
    assert_eq!(session.editor.text().len(), DOCUMENT_LIMIT);
    assert_eq!(session.editor.snapshot().cursor_byte, DOCUMENT_LIMIT);
}
