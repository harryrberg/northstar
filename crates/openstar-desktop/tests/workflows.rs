// SPDX-License-Identifier: GPL-3.0-only
use iced::keyboard::{
    Event, Key, Location, Modifiers,
    key::{Named, NativeCode, Physical},
};
use openstar_core::{Command, Document, Editor};
use openstar_desktop::{
    input::{Action, Keyboard},
    projection::{Projection, UTF8_TEXT_COLUMNS},
    screen::Screen,
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
            "openstar-desktop-{}-{}",
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
        let command = match action {
            Action::Edit(command) => command,
            Action::Backspace => Command::Backspace,
            Action::Delete => Command::DeleteCharacter,
            _ => panic!("unexpected action: {action:?}"),
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
        "navword-next-over-hyphen",
        "navword-next-twice",
        "navword-marked-letter-inside",
        "navword-marked-letter-from-space",
        "navword-marked-period-next",
        "navline-first-start",
        "navline-first-end",
        "navline-second-start",
        "navline-second-end",
        "navline-empty-end",
        "navquick-block-begin",
        "navquick-block-end",
        "navquick-document-end",
        "backdel-backspace-middle",
        "backdel-backspace-join",
        "backdel-delete-middle",
        "backdel-delete-join",
        "block-copy-end",
        "block-copy-repeat-end",
        "block-copy-hard-break",
        "block-copy-before-source",
        "block-copy-inside-source",
        "block-copy-insert-off",
        "block-display-mark-on",
        "block-display-hide",
        "block-display-show",
        "right-delete-suffix",
        "right-restore-suffix",
        "right-hard-break",
        "right-break-noop-buffer",
        "right-highbit-suffix",
        "right-highbit-restore",
        "left-delete-prefix",
        "left-start-noop-buffer",
        "left-hard-break",
        "left-hard-break-restore",
        "left-break-full-line",
        "left-break-restore",
        "left-highbit-prefix",
        "left-highbit-restore",
        "block-copy-repeat-before-source",
        "block-copy-repeat-inside-source",
        "block-move-end",
        "block-move-inline",
        "block-move-then-copy",
        "block-delete-inline",
        "block-move-before-source",
        "block-move-before-near-source",
        "block-move-before-interior",
        "block-move-before-adjacent",
        "block-move-after-interior",
        "block-move-before-first-line",
        "block-move-before-following-line",
        "block-move-before-then-copy",
        "block-move-source-to-eof",
        "block-delete-suffix-two",
        "block-delete-through-eof",
        "block-delete-with-following-line",
        "block-delete-across-break",
        "block-delete-across-break-following",
        "block-delete-through-break",
        "block-delete-break-nonterminal-suffix",
        "hard-break-insert-on",
        "hard-break-insert-off",
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
        let initial_cursor = case["initial"]["cursor_byte"].as_u64().unwrap() as usize;
        while session.editor.snapshot().cursor_byte < initial_cursor {
            let previous = session.editor.snapshot().cursor_byte;
            dispatch(&mut keyboard, &mut session, ctrl("d", false));
            assert!(session.editor.snapshot().cursor_byte > previous);
        }
        assert_eq!(session.editor.snapshot().cursor_byte, initial_cursor);
        if case["initial"]["insert"] == false {
            dispatch(&mut keyboard, &mut session, ctrl("v", false));
        }
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
                Command::LineStart | Command::LineEnd => {
                    let key = if matches!(command, Command::LineStart) {
                        "s"
                    } else {
                        "d"
                    };
                    dispatch(&mut keyboard, &mut session, ctrl("q", false));
                    dispatch(
                        &mut keyboard,
                        &mut session,
                        press(
                            Key::Character(key.into()),
                            Modifiers::empty(),
                            Some(key),
                            false,
                        ),
                    );
                }
                Command::DocumentEnd | Command::JumpBlockBegin | Command::JumpBlockEnd => {
                    let key = match command {
                        Command::DocumentEnd => "c",
                        Command::JumpBlockBegin => "b",
                        Command::JumpBlockEnd => "k",
                        _ => unreachable!(),
                    };
                    dispatch(&mut keyboard, &mut session, ctrl("q", false));
                    dispatch(
                        &mut keyboard,
                        &mut session,
                        press(
                            Key::Character(key.into()),
                            Modifiers::empty(),
                            Some(key),
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
                Command::NextWord => dispatch(&mut keyboard, &mut session, ctrl("f", false)),
                Command::PreviousWord => dispatch(&mut keyboard, &mut session, ctrl("a", false)),
                Command::NextLine => dispatch(&mut keyboard, &mut session, ctrl("x", false)),
                Command::PreviousLine => dispatch(&mut keyboard, &mut session, ctrl("e", false)),
                Command::DeleteCharacter => dispatch(
                    &mut keyboard,
                    &mut session,
                    if case["profile"] == "ws4-backspace-delete-v1" {
                        press(Key::Named(Named::Delete), Modifiers::empty(), None, false)
                    } else {
                        ctrl("g", false)
                    },
                ),
                Command::Backspace => dispatch(
                    &mut keyboard,
                    &mut session,
                    press(
                        Key::Named(Named::Backspace),
                        Modifiers::empty(),
                        None,
                        false,
                    ),
                ),
                Command::MarkBlockBegin
                | Command::MarkBlockEnd
                | Command::ToggleBlockDisplay
                | Command::CopyBlock
                | Command::MoveBlock
                | Command::DeleteBlock => {
                    let key = match command {
                        Command::MarkBlockBegin => "b",
                        Command::MarkBlockEnd => "k",
                        Command::ToggleBlockDisplay => "h",
                        Command::CopyBlock => "c",
                        Command::MoveBlock => "v",
                        Command::DeleteBlock => "y",
                        _ => unreachable!(),
                    };
                    dispatch(&mut keyboard, &mut session, ctrl("k", false));
                    dispatch(
                        &mut keyboard,
                        &mut session,
                        press(
                            Key::Character(key.into()),
                            Modifiers::empty(),
                            Some(key),
                            false,
                        ),
                    );
                }
                Command::DeleteWord => dispatch(&mut keyboard, &mut session, ctrl("t", false)),
                Command::DeleteLine => dispatch(&mut keyboard, &mut session, ctrl("y", false)),
                Command::DeleteToLineEnd => {
                    dispatch(&mut keyboard, &mut session, ctrl("q", false));
                    dispatch(
                        &mut keyboard,
                        &mut session,
                        press(
                            Key::Character("y".into()),
                            Modifiers::empty(),
                            Some("y"),
                            false,
                        ),
                    );
                }
                Command::DeleteToLineStart => {
                    dispatch(&mut keyboard, &mut session, ctrl("q", false));
                    dispatch(
                        &mut keyboard,
                        &mut session,
                        press(Key::Named(Named::Delete), Modifiers::empty(), None, false),
                    );
                }
                Command::UneraseLine => dispatch(&mut keyboard, &mut session, ctrl("u", false)),
                Command::Enter => dispatch(
                    &mut keyboard,
                    &mut session,
                    press(Key::Named(Named::Enter), Modifiers::empty(), None, false),
                ),
                Command::InsertHardBreak => dispatch(&mut keyboard, &mut session, ctrl("n", false)),
                Command::FindLiteral { .. }
                | Command::FindCharacter { .. }
                | Command::ReplaceLiteral { .. }
                | Command::AnswerReplace { .. }
                | Command::RepeatFind => unreachable!("other profile"),
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
        if case["profile"] == "ws4-block-copy-v1"
            || (case["profile"] == "ws4-block-move-delete-v1"
                && case["expected"]["marks_cleared"] != true)
            || case["profile"] == "ws4-block-display-v1"
        {
            assert_eq!(
                session.editor.snapshot().block_begin,
                Some(expected["block_begin"].as_u64().unwrap() as usize)
            );
            assert_eq!(
                session.editor.snapshot().block_end,
                Some(expected["block_end"].as_u64().unwrap() as usize)
            );
        }
        if case["profile"] == "ws4-block-display-v1" {
            assert_eq!(
                session.editor.snapshot().block_visible,
                expected["block_visible"].as_bool().unwrap()
            );
        }
        if case["expected"]["marks_cleared"] == true {
            assert_eq!(session.editor.snapshot().block_begin, None);
            assert_eq!(session.editor.snapshot().block_end, None);
        }
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
fn legacy_backspace_and_delete_session_save_and_reopen() {
    let scratch = Scratch::new();
    let source = scratch.0.join("input.doc");
    let mut carrier = b"ABCDE".to_vec();
    carrier.resize(128, 0x1a);
    fs::write(&source, &carrier).unwrap();
    let mut session = DocumentSession::Legacy(Session::open(&source).unwrap());
    for _ in 0..3 {
        session.apply_command(&Command::NextCharacter).unwrap();
    }
    session.backspace().unwrap();
    session.delete().unwrap();
    let destination = scratch.0.join("output.doc");
    session.save_as(&destination).unwrap();
    assert_eq!(&fs::read(&destination).unwrap()[..3], b"ABE");
    assert_eq!(&fs::read(&source).unwrap()[..5], b"ABCDE");
    let reopened = Session::open(&destination).unwrap();
    assert_eq!(
        reopened.editor.document().bytes(),
        fs::read(&destination).unwrap()
    );
}

#[test]
fn selected_block_uses_reverse_video_and_end_marker() {
    let mut session = Session::default();
    session
        .editor
        .apply(&Command::InsertAscii {
            text: "ABCD".into(),
        })
        .unwrap();
    session.editor.apply(&Command::DocumentStart).unwrap();
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.editor.apply(&Command::MarkBlockBegin).unwrap();
    let begin_screen = Screen::editor(
        &Projection::new(&session.editor),
        &session.editor.snapshot(),
        "TEST.DOC",
        None,
    );
    assert_eq!(begin_screen.cells[11][79].glyph, 'B');
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.editor.apply(&Command::MarkBlockEnd).unwrap();
    let projection = Projection::new(&session.editor);
    let screen = Screen::editor(&projection, &session.editor.snapshot(), "TEST.DOC", None);
    assert_eq!(screen.cells[11][0].attribute, 7);
    assert_eq!(screen.cells[11][1].attribute, 0x70);
    assert_eq!(screen.cells[11][2].attribute, 0x70);
    assert_eq!(screen.cells[11][3].attribute, 7);
    assert_eq!(screen.cells[11][79].glyph, 'K');
    session.editor.apply(&Command::ToggleBlockDisplay).unwrap();
    let hidden_projection = Projection::new(&session.editor);
    let hidden = Screen::editor(
        &hidden_projection,
        &session.editor.snapshot(),
        "TEST.DOC",
        Some('K'),
    );
    assert_eq!(hidden.cells[11][1].attribute, 7);
    assert_eq!(hidden.cells[11][2].attribute, 7);
    assert_ne!(hidden.cells[11][79].glyph, 'K');
    assert!(!hidden_projection.rows[0][1].selected);
    let menu: String = hidden.cells[5][26..43]
        .iter()
        .map(|cell| cell.glyph)
        .collect();
    assert_eq!(menu, "H turn display on");
    session.editor.apply(&Command::ToggleBlockDisplay).unwrap();
    let shown = Screen::editor(
        &Projection::new(&session.editor),
        &session.editor.snapshot(),
        "TEST.DOC",
        Some('K'),
    );
    assert_eq!(shown.cells[11][1].attribute, 0x70);
    assert_eq!(shown.cells[11][79].glyph, 'K');
    let menu: String = shown.cells[5][26..44]
        .iter()
        .map(|cell| cell.glyph)
        .collect();
    assert_eq!(menu, "H turn display off");
}

#[test]
fn legacy_word_keys_follow_dos_positions_and_preserve_the_file() {
    let scratch = Scratch::new();
    let source = scratch.0.join("N2.DOC");
    let mut carrier = b"ONE TWO THREE".to_vec();
    carrier.resize(128, 0x1a);
    fs::write(&source, &carrier).unwrap();
    let mut session = Session::open(&source).unwrap();
    let mut keyboard = Keyboard::default();
    for (key, expected) in [("f", 4), ("f", 8), ("a", 4), ("a", 0)] {
        dispatch(&mut keyboard, &mut session, ctrl(key, false));
        assert_eq!(session.editor.snapshot().cursor_byte, expected);
        assert_eq!(session.editor.document().bytes(), carrier);
        assert!(!session.editor.snapshot().modified);
    }
    let copy = scratch.0.join("N2-COPY.DOC");
    session.save_as(&copy).unwrap();
    assert_eq!(fs::read(source).unwrap(), carrier);
    assert_eq!(fs::read(&copy).unwrap(), carrier);
    assert_eq!(
        Session::open(&copy).unwrap().editor.document().bytes(),
        carrier
    );
}

#[test]
fn legacy_vertical_keys_follow_observed_clamping_and_preserve_the_file() {
    let scratch = Scratch::new();
    let source = scratch.0.join("NAV4.DOC");
    let mut carrier = b"ABCDE\r\nX\r\n\r\nLONG LINE HERE".to_vec();
    carrier.resize(128, 0x1a);
    fs::write(&source, &carrier).unwrap();
    let mut session = Session::open(&source).unwrap();
    let mut keyboard = Keyboard::default();
    for _ in 0..4 {
        dispatch(&mut keyboard, &mut session, ctrl("d", false));
    }
    assert_eq!(session.editor.snapshot().cursor_byte, 4);
    for (event, expected) in [
        (ctrl("x", false), 8),
        (
            press(
                Key::Named(Named::ArrowDown),
                Modifiers::empty(),
                None,
                false,
            ),
            10,
        ),
        (ctrl("x", false), 12),
        (
            press(
                Key::Named(Named::ArrowDown),
                Modifiers::empty(),
                None,
                false,
            ),
            12,
        ),
        (ctrl("e", false), 10),
        (
            press(Key::Named(Named::ArrowUp), Modifiers::empty(), None, false),
            7,
        ),
        (ctrl("e", false), 0),
    ] {
        dispatch(&mut keyboard, &mut session, event);
        assert_eq!(session.editor.snapshot().cursor_byte, expected);
        assert_eq!(session.editor.document().bytes(), carrier);
    }
    assert!(!session.editor.snapshot().modified);
    let copy = scratch.0.join("NAV4-COPY.DOC");
    session.save_as(&copy).unwrap();
    assert_eq!(fs::read(&source).unwrap(), carrier);
    assert_eq!(fs::read(&copy).unwrap(), carrier);
    assert_eq!(
        Session::open(&copy).unwrap().editor.document().bytes(),
        carrier
    );
}

#[test]
fn legacy_ctrl_g_deletes_and_saved_file_reopens_with_the_dos_bytes() {
    let scratch = Scratch::new();
    let source = scratch.0.join("DEL1.DOC");
    let mut carrier = b"ABCDE\r\nXYZ\r\n".to_vec();
    carrier.resize(128, 0x1a);
    fs::write(&source, &carrier).unwrap();
    let mut session = Session::open(&source).unwrap();
    let mut keyboard = Keyboard::default();
    dispatch(&mut keyboard, &mut session, ctrl("d", false));
    dispatch(&mut keyboard, &mut session, ctrl("g", false));
    assert_eq!(session.editor.snapshot().cursor_byte, 1);
    assert_eq!(&session.editor.document().bytes()[..4], b"ACDE");
    let output = scratch.0.join("DEL1-OUT.DOC");
    session.save_as(&output).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&output).unwrap())),
        "27f16abd4838201af57adb1c0878f4a6f75ad8f2262d8785c91864222dbe7f60"
    );
    assert_eq!(
        Session::open(&output).unwrap().editor.document().bytes(),
        fs::read(&output).unwrap()
    );
    assert_eq!(fs::read(&source).unwrap(), carrier);
}

#[test]
fn legacy_ctrl_y_and_ctrl_u_restore_a_line_through_the_keyboard_adapter() {
    let scratch = Scratch::new();
    let source = scratch.0.join("LINE.DOC");
    let mut carrier = b"AB\r\nCD".to_vec();
    carrier.resize(128, 0x1a);
    fs::write(&source, &carrier).unwrap();
    let mut session = Session::open(&source).unwrap();
    let mut keyboard = Keyboard::default();
    dispatch(&mut keyboard, &mut session, ctrl("y", false));
    assert_eq!(&session.editor.document().bytes()[..2], b"CD");
    assert_eq!(session.editor.snapshot().cursor_byte, 0);
    dispatch(&mut keyboard, &mut session, ctrl("u", false));
    assert_eq!(session.editor.document().bytes(), carrier);
    assert_eq!(session.editor.snapshot().cursor_byte, 0);
    let output = scratch.0.join("LINE-OUT.DOC");
    session.save_as(&output).unwrap();
    assert_eq!(fs::read(&output).unwrap(), carrier);
    assert_eq!(
        Session::open(&output).unwrap().editor.document().bytes(),
        carrier
    );
    assert_eq!(fs::read(&source).unwrap(), carrier);
}

#[test]
fn legacy_ctrl_t_and_ctrl_u_restore_a_word_through_the_keyboard_adapter() {
    let scratch = Scratch::new();
    let source = scratch.0.join("WORD.DOC");
    let mut carrier = b"ONE TWO".to_vec();
    carrier.resize(128, 0x1a);
    fs::write(&source, &carrier).unwrap();
    let mut session = Session::open(&source).unwrap();
    let mut keyboard = Keyboard::default();
    dispatch(&mut keyboard, &mut session, ctrl("t", false));
    assert_eq!(&session.editor.document().bytes()[..4], b" TWO");
    dispatch(&mut keyboard, &mut session, ctrl("u", false));
    assert_eq!(session.editor.document().bytes(), carrier);
    let output = scratch.0.join("WORD-OUT.DOC");
    session.save_as(&output).unwrap();
    assert_eq!(fs::read(&output).unwrap(), carrier);
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
fn legacy_repeated_save_replaces_document_and_rotates_bak() {
    let scratch = Scratch::new();
    let path = scratch.0.join("SAVE1.DOC");
    let mut original = b"ALPHA\r\n".to_vec();
    original.resize(128, 0x1a);
    fs::write(&path, &original).unwrap();
    let mut session = Session::open(&path).unwrap();
    session
        .editor
        .apply(&Command::InsertAscii { text: "Z".into() })
        .unwrap();
    let first = session.editor.document().bytes().to_vec();
    session.save().unwrap();
    assert_eq!(fs::read(&path).unwrap(), first);
    assert_eq!(fs::read(scratch.0.join("SAVE1.BAK")).unwrap(), original);
    assert!(!session.editor.snapshot().modified);

    session
        .editor
        .apply(&Command::InsertAscii { text: "Y".into() })
        .unwrap();
    let second = session.editor.document().bytes().to_vec();
    session.save().unwrap();
    assert_eq!(fs::read(&path).unwrap(), second);
    assert_eq!(fs::read(scratch.0.join("SAVE1.BAK")).unwrap(), first);
    session.save().unwrap();
    assert_eq!(fs::read(scratch.0.join("SAVE1.BAK")).unwrap(), second);
}

#[test]
fn external_change_or_missing_document_blocks_save_without_advancing_checkpoint() {
    let scratch = Scratch::new();
    let path = scratch.0.join("draft.txt");
    fs::write(&path, "Først").unwrap();
    let mut session = Utf8Session::open(&path).unwrap();
    session.editor.insert_text("X").unwrap();
    fs::write(&path, "Fra annet program").unwrap();
    assert!(
        session
            .save()
            .unwrap_err()
            .to_string()
            .contains("changed outside")
    );
    assert_eq!(fs::read(&path).unwrap(), "Fra annet program".as_bytes());
    assert!(!scratch.0.join("draft.txt.bak").exists());
    assert!(session.editor.snapshot().modified);
    fs::remove_file(&path).unwrap();
    assert_eq!(
        session.save().unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(!path.exists());
    assert!(session.editor.snapshot().modified);

    let copy = scratch.0.join("saved-copy.txt");
    session.save_as(&copy).unwrap();
    assert_eq!(fs::read(&copy).unwrap(), "XFørst".as_bytes());
    assert!(!session.editor.snapshot().modified);
}

#[test]
fn utf8_save_preserves_existing_bytes_and_creates_separate_backup() {
    let scratch = Scratch::new();
    let path = scratch.0.join("chapter.txt");
    let original = "\u{feff}Cafe\u{301}\r\næøå\n";
    fs::write(&path, original).unwrap();
    let mut session = Utf8Session::open(&path).unwrap();
    session.editor.insert_text("Blå ").unwrap();
    let edited = session.editor.text().as_bytes().to_vec();
    session.save().unwrap();
    assert_eq!(fs::read(&path).unwrap(), edited);
    assert_eq!(
        fs::read(scratch.0.join("chapter.txt.bak")).unwrap(),
        original.as_bytes()
    );
    assert_eq!(
        Utf8Session::open(&path).unwrap().editor.text().as_bytes(),
        edited
    );
}

#[test]
fn unusable_backup_target_blocks_replacement_and_keeps_document_dirty() {
    let scratch = Scratch::new();
    let path = scratch.0.join("chapter.txt");
    fs::write(&path, "first").unwrap();
    fs::create_dir(scratch.0.join("chapter.txt.bak")).unwrap();
    let mut session = Utf8Session::open(&path).unwrap();
    session.editor.insert_text("X").unwrap();
    assert!(session.save().is_err());
    assert_eq!(fs::read(&path).unwrap(), b"first");
    assert!(session.editor.snapshot().modified);
}

#[cfg(windows)]
#[test]
fn windows_file_without_delete_sharing_blocks_replacement_but_keeps_backup() {
    use std::os::windows::fs::OpenOptionsExt;

    let scratch = Scratch::new();
    let path = scratch.0.join("locked.txt");
    fs::write(&path, "first").unwrap();
    let mut session = Utf8Session::open(&path).unwrap();
    session.editor.insert_text("X").unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0x00000001) // FILE_SHARE_READ, without FILE_SHARE_DELETE
        .open(&path)
        .unwrap();
    assert!(session.save().is_err());
    assert_eq!(fs::read(&path).unwrap(), b"first");
    assert_eq!(
        fs::read(scratch.0.join("locked.txt.bak")).unwrap(),
        b"first"
    );
    assert!(session.editor.snapshot().modified);
    drop(lock);
    session.save().unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"Xfirst");
    assert!(!session.editor.snapshot().modified);
}

#[test]
fn save_keys_keep_save_and_save_as_distinct() {
    let mut keyboard = Keyboard::default();
    assert_eq!(keyboard.handle(&ctrl("k", false)), None);
    assert_eq!(keyboard.handle(&ctrl("s", false)), Some(Action::Save));
    assert_eq!(
        keyboard.handle(&press(
            Key::Character("s".into()),
            Modifiers::CTRL | Modifiers::SHIFT,
            None,
            false
        )),
        Some(Action::SaveAs)
    );
    assert_eq!(keyboard.handle(&ctrl("k", false)), None);
    assert_eq!(
        keyboard.handle(&ctrl("d", false)),
        Some(Action::SaveAndDone)
    );
    assert_eq!(keyboard.handle(&ctrl("k", false)), None);
    assert_eq!(
        keyboard.handle(&ctrl("x", false)),
        Some(Action::SaveAndExit)
    );
    assert_eq!(keyboard.handle(&ctrl("k", false)), None);
    assert_eq!(keyboard.handle(&ctrl("w", false)), Some(Action::WriteBlock));
    assert_eq!(keyboard.handle(&ctrl("k", false)), None);
    assert_eq!(keyboard.handle(&ctrl("r", false)), Some(Action::InsertFile));
    assert_eq!(keyboard.handle(&ctrl("k", false)), None);
    assert_eq!(
        keyboard.handle(&ctrl("h", false)),
        Some(Action::Edit(Command::ToggleBlockDisplay))
    );
}

#[test]
fn ctrl_k_r_inserts_a_wordstar_file_and_saves_the_dos_result() {
    let scratch = Scratch::new();
    let source = scratch.0.join("INSERT.DOC");
    let target = scratch.0.join("MAIN.DOC");
    let mut input = b"XY".to_vec();
    input.resize(128, 0x1a);
    fs::write(&source, &input).unwrap();
    let mut session = Session::new_at(target.clone());
    session
        .editor
        .apply(&Command::InsertAscii {
            text: "ABCDE".into(),
        })
        .unwrap();
    session.editor.apply(&Command::DocumentStart).unwrap();
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.insert_file(&source).unwrap();
    assert_eq!(session.editor.snapshot().cursor_byte, 2);
    assert_eq!(&session.editor.document().bytes()[..7], b"ABXYCDE");
    assert_eq!(fs::read(&source).unwrap(), input);
    session.save().unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&target).unwrap())),
        "3233716c289dcb21280b7716c735d33f721441b56ff879c36e6b77409fc87ca4"
    );
    assert_eq!(
        Session::open(&target).unwrap().editor.document().bytes(),
        fs::read(&target).unwrap()
    );
    assert_eq!(fs::read(&source).unwrap(), input);
}

#[test]
fn write_marked_block_creates_the_observed_dos_carrier_without_saving_source() {
    let scratch = Scratch::new();
    let source = scratch.0.join("source.doc");
    let target = scratch.0.join("part.doc");
    let mut session = Session::new_at(source.clone());
    session
        .editor
        .apply(&Command::InsertAscii {
            text: "ABCDE".into(),
        })
        .unwrap();
    session.editor.apply(&Command::DocumentStart).unwrap();
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.editor.apply(&Command::MarkBlockBegin).unwrap();
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.editor.apply(&Command::NextCharacter).unwrap();
    session.editor.apply(&Command::MarkBlockEnd).unwrap();
    let before = session.editor.document().bytes().to_vec();
    session.write_block_new(&target).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&target).unwrap())),
        "102b9c95520f06407210fcc65ba922d63824556bc7211fa6cbe9a6a37b39c7f1"
    );
    assert_eq!(session.editor.document().bytes(), before);
    assert_eq!(session.editor.snapshot().cursor_byte, 1);
    assert_eq!(session.editor.snapshot().block_begin, Some(1));
    assert!(session.editor.snapshot().modified);
    assert_eq!(session.path.as_deref(), Some(source.as_path()));
    assert!(!source.exists());
    assert_eq!(
        session.write_block_new(&target).unwrap_err().kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(session.editor.snapshot().cursor_byte, 1);

    let mut utf8 = DocumentSession::Utf8(Utf8Session::new_at(None));
    let unsupported = scratch.0.join("utf8-block.doc");
    assert_eq!(
        utf8.write_block_new(&unsupported).unwrap_err().kind(),
        std::io::ErrorKind::Unsupported
    );
    assert!(!unsupported.exists());
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
fn utf8_undo_redo_keys_restore_cursor_and_exact_saved_bytes() {
    let scratch = Scratch::new();
    let path = scratch.0.join("draft.txt");
    let mut session = DocumentSession::Utf8(Utf8Session::new_at(None));
    let mut keyboard = Keyboard::default();
    let Action::Text(text) = keyboard.handle_mode(&type_text("Blåbær"), true).unwrap() else {
        panic!("UTF-8 text was not routed to the editor")
    };
    session.insert_text(&text).unwrap();
    session.insert_text("\r\nCafe\u{301} 👩‍💻").unwrap();
    let saved_cursor = match &session {
        DocumentSession::Utf8(value) => value.editor.snapshot().cursor_byte,
        DocumentSession::Legacy(_) => unreachable!(),
    };
    session.save_as(&path).unwrap();
    session.backspace().unwrap();
    assert!(session.modified());
    assert_eq!(
        keyboard.handle_mode(&ctrl("z", false), true),
        Some(Action::Undo)
    );
    assert!(session.undo().unwrap());
    assert!(!session.modified());
    let DocumentSession::Utf8(value) = &session else {
        unreachable!()
    };
    assert_eq!(value.editor.snapshot().cursor_byte, saved_cursor);
    assert_eq!(value.editor.text(), "Blåbær\r\nCafe\u{301} 👩‍💻");
    assert_eq!(
        keyboard.handle_mode(&ctrl("y", false), true),
        Some(Action::Redo)
    );
    assert_eq!(
        keyboard.handle_mode(
            &press(
                Key::Character("z".into()),
                Modifiers::CTRL | Modifiers::SHIFT,
                None,
                false
            ),
            true
        ),
        Some(Action::Redo)
    );
    assert!(session.redo().unwrap());
    assert_eq!(keyboard.handle_mode(&ctrl("z", true), true), None);
    assert_ne!(
        keyboard.handle_mode(&ctrl("z", false), false),
        Some(Action::Undo)
    );
    session.save().unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        "Blåbær\r\nCafe\u{301} ".as_bytes()
    );
    assert_eq!(
        fs::read(scratch.0.join("draft.txt.bak")).unwrap(),
        "Blåbær\r\nCafe\u{301} 👩‍💻".as_bytes()
    );
    assert_eq!(
        Utf8Session::open(&path).unwrap().editor.text(),
        "Blåbær\r\nCafe\u{301} "
    );
    assert!(session.undo().unwrap());
    assert!(session.modified());
    session.save().unwrap();
    assert_eq!(
        fs::read(&path).unwrap(),
        "Blåbær\r\nCafe\u{301} 👩‍💻".as_bytes()
    );
}

#[test]
fn utf8_word_wrap_and_vertical_keys_preserve_stored_line_endings() {
    let scratch = Scratch::new();
    let source = scratch.0.join("long.txt");
    let original = format!("{}Blåbær\r\n{}", "word ".repeat(15), "a".repeat(78));
    fs::write(&source, &original).unwrap();
    let mut session = Utf8Session::open(&source).unwrap();
    let projection = Projection::utf8(&session.editor);
    assert_eq!(UTF8_TEXT_COLUMNS, 78);
    let rows = projection
        .rows
        .iter()
        .map(|row| row.iter().map(|cell| cell.glyph).collect::<String>())
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            format!("{}↵", "word ".repeat(15)),
            "Blåbær¶".into(),
            "a".repeat(78)
        ]
    );
    let screen = Screen::utf8_editor(&projection, "long.txt", None, 0);
    assert_eq!(screen.cells[11][79].glyph, ' ');
    assert_eq!(screen.cells[12][79].glyph, '<');

    let mut keyboard = Keyboard::default();
    assert_eq!(
        keyboard.handle_mode(
            &press(
                Key::Named(Named::ArrowDown),
                Modifiers::empty(),
                None,
                false
            ),
            true
        ),
        Some(Action::MoveVertical(1))
    );
    assert_eq!(
        keyboard.handle_mode(&ctrl("e", false), true),
        Some(Action::MoveVertical(-1))
    );
    assert_ne!(
        keyboard.handle_mode(
            &press(
                Key::Named(Named::ArrowDown),
                Modifiers::empty(),
                None,
                false
            ),
            false
        ),
        Some(Action::MoveVertical(1))
    );

    session.editor.set_cursor(75).unwrap();
    let projection = Projection::utf8(&session.editor);
    assert_eq!(projection.cursor, (1, 0));
    session.editor.set_cursor(75 + "Blå".len()).unwrap();
    assert_eq!(Projection::utf8(&session.editor).cursor, (1, 3));
    session.editor.set_cursor(75).unwrap();
    let next = projection.utf8_vertical_target(1, 6).unwrap();
    session.editor.set_cursor(next).unwrap();
    assert_eq!(Projection::utf8(&session.editor).cursor, (2, 6));
    let destination = scratch.0.join("copy.txt");
    session.save_as(&destination).unwrap();
    assert_eq!(fs::read(&source).unwrap(), original.as_bytes());
    assert_eq!(fs::read(&destination).unwrap(), original.as_bytes());
}

#[test]
fn utf8_wrap_keeps_the_cursor_visible_at_a_full_row_and_crlf_boundary() {
    let text = format!("{}\r\nB", "a".repeat(UTF8_TEXT_COLUMNS));
    let mut editor = openstar_core::Utf8Editor::new(text);
    editor.set_cursor(UTF8_TEXT_COLUMNS).unwrap();
    let projection = Projection::utf8(&editor);
    assert_eq!(projection.cursor, (0, UTF8_TEXT_COLUMNS));
    assert_eq!(projection.rows[0].last().unwrap().glyph, '¶');
    let screen = Screen::utf8_editor(&projection, "edge.txt", None, 0);
    assert_eq!(screen.cursor, Some((11, UTF8_TEXT_COLUMNS)));
    editor.set_cursor(UTF8_TEXT_COLUMNS + 2).unwrap();
    assert_eq!(Projection::utf8(&editor).cursor, (1, 0));

    let mut editor =
        openstar_core::Utf8Editor::new(format!("{}👩‍💻", "a".repeat(UTF8_TEXT_COLUMNS + 1)));
    editor.set_cursor(UTF8_TEXT_COLUMNS).unwrap();
    let projection = Projection::utf8(&editor);
    assert_eq!(projection.cursor, (1, 0));
    assert_eq!(projection.rows[0].last().unwrap().glyph, '↵');
    assert_eq!(projection.rows[1].len(), 2);
    editor.set_cursor(UTF8_TEXT_COLUMNS + 1).unwrap();
    assert_eq!(Projection::utf8(&editor).cursor, (1, 1));

    let mut editor = openstar_core::Utf8Editor::new("A\rB".into());
    editor.set_cursor(2).unwrap();
    let projection = Projection::utf8(&editor);
    assert_eq!(projection.cursor, (1, 0));
    assert_eq!(projection.rows[0].last().unwrap().glyph, '¶');

    let mut editor = openstar_core::Utf8Editor::new(format!("{} B", "a".repeat(UTF8_TEXT_COLUMNS)));
    editor.set_cursor(UTF8_TEXT_COLUMNS + 1).unwrap();
    let projection = Projection::utf8(&editor);
    assert_eq!(projection.cursor, (1, 0));
    assert_eq!(projection.rows[1][0].glyph, 'B');
    assert_eq!(editor.text().as_bytes()[UTF8_TEXT_COLUMNS], b' ');
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
