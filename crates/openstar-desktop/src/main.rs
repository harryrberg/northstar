// SPDX-License-Identifier: GPL-3.0-only
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use iced::{
    Element, Event, Fill, Font, Subscription, Task, Theme, clipboard, keyboard,
    widget::{canvas, column, responsive, row, space, stack, text_input},
    window,
};
use openstar_desktop::{
    APP_TITLE,
    input::{Action, Keyboard},
    projection::Projection,
    recovery::{RecoveryCandidate, RecoveryStore},
    screen::Screen,
    session::{DocumentSession, Session, Utf8Session},
};
use std::{path::PathBuf, time::Duration};
mod surface;
const UTF8_VISIBLE_ROWS: usize = 12;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title(App::title)
        .subscription(App::subscription)
        .theme(Theme::custom(
            "DOS monochrome",
            iced::theme::Palette {
                background: iced::Color::BLACK,
                text: iced::Color::from_rgb8(170, 170, 170),
                primary: iced::Color::from_rgb8(170, 170, 170),
                success: iced::Color::WHITE,
                danger: iced::Color::WHITE,
                warning: iced::Color::WHITE,
            },
        ))
        .font(include_bytes!("../assets/fonts/Px437_IBM_VGA_8x16.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/IBMPlexMono-Regular.ttf").as_slice())
        .default_font(Font::with_name("IBM Plex Mono"))
        .window(window::Settings {
            size: iced::Size::new(1120.0, 700.0),
            min_size: Some(iced::Size::new(640.0, 400.0)),
            exit_on_close_request: false,
            ..Default::default()
        })
        .run()
}
struct App {
    session: DocumentSession,
    projection: Projection,
    keyboard: Keyboard,
    dialog: Option<Dialog>,
    find_dialog: Option<FindDialog>,
    notice: String,
    window_focused: bool,
    opening: bool,
    fullscreen: bool,
    classic_font: bool,
    session_generation: u64,
    utf8_top_row: usize,
    preferred_column: Option<usize>,
    recovery_store: Option<RecoveryStore>,
    recovery_id: String,
    recovery_written: bool,
    recovery_pending: bool,
}
#[derive(Debug, Clone, Copy)]
enum Next {
    Opening,
    Close(window::Id),
}
#[derive(Debug, Clone, Copy)]
enum PathKind {
    OpenLegacy,
    OpenUtf8,
    SaveAs { done: bool },
}
enum Dialog {
    Path {
        kind: PathKind,
        value: String,
        error: String,
    },
    Discard(Next),
    Recovery {
        candidate: Box<RecoveryCandidate>,
        error: String,
    },
    Help,
}
enum FindDialog {
    Term(String),
    Options {
        term: String,
        flags: String,
    },
    ReplaceTerm(String),
    ReplaceWith {
        term: String,
        value: String,
    },
    ReplaceOptions {
        term: String,
        value: String,
        flags: String,
    },
    ReplaceConfirm,
    Notice {
        term: String,
        repeated: bool,
        replace: bool,
    },
}
#[derive(Debug, Clone)]
pub(crate) enum Message {
    Event(Event, window::Id),
    Open,
    NewUtf8,
    OpenUtf8,
    Save(bool),
    SaveAs(bool),
    Help,
    PathChanged(String),
    Submit,
    Cancel,
    Discard,
    Paste(u64, Option<String>),
    RecoveryTick,
    ShowRecovery,
    Recover,
    SkipRecovery,
    DiscardRecovery,
}
impl App {
    fn handle_find_event(&mut self, event: &keyboard::Event) {
        use keyboard::{Event::KeyPressed, Key, key::Named};
        let Some(dialog) = self.find_dialog.take() else {
            return;
        };
        let KeyPressed {
            key,
            text,
            modifiers,
            ..
        } = event
        else {
            self.find_dialog = Some(dialog);
            return;
        };
        if *key == Key::Named(Named::Escape)
            || (modifiers.control() && key.as_ref() == Key::Character("u"))
        {
            if matches!(dialog, FindDialog::ReplaceConfirm) {
                let result = self
                    .session
                    .apply_command(&openstar_core::Command::AnswerReplace { accept: false });
                self.record_edit(result);
            }
            return;
        }
        match dialog {
            FindDialog::Term(mut term) => match key {
                Key::Named(Named::Enter) if !term.is_empty() => {
                    self.find_dialog = Some(FindDialog::Options {
                        term,
                        flags: String::new(),
                    })
                }
                Key::Named(Named::Backspace | Named::Delete) => {
                    term.pop();
                    self.find_dialog = Some(FindDialog::Term(term));
                }
                _ => {
                    if !modifiers.control()
                        && !modifiers.alt()
                        && !modifiers.logo()
                        && let Some(value) = text
                        && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
                        && term.len() + value.len() <= 64
                    {
                        term.push_str(value);
                    }
                    self.find_dialog = Some(FindDialog::Term(term));
                }
            },
            FindDialog::Options { term, mut flags } => match key {
                Key::Named(Named::Enter) => {
                    let command = openstar_core::Command::FindLiteral {
                        text: term.clone(),
                        whole_words: flags.contains('W'),
                        ignore_case: flags.contains('U'),
                        backwards: flags.contains('B'),
                    };
                    let result = self.session.apply_command(&command);
                    self.record_edit(result);
                    if let DocumentSession::Legacy(session) = &self.session
                        && session.editor.last_find_result() == Some(false)
                    {
                        self.find_dialog = Some(FindDialog::Notice {
                            term,
                            repeated: false,
                            replace: false,
                        });
                    }
                }
                Key::Named(Named::Backspace | Named::Delete) => {
                    flags.pop();
                    self.find_dialog = Some(FindDialog::Options { term, flags });
                }
                _ => {
                    let mut unsupported = false;
                    if !modifiers.control()
                        && !modifiers.alt()
                        && !modifiers.logo()
                        && let Some(value) = text
                    {
                        for ch in value.chars().map(|ch| ch.to_ascii_uppercase()) {
                            if "WUB".contains(ch) && !flags.contains(ch) {
                                flags.push(ch);
                            } else if "G?".contains(ch) {
                                unsupported = true;
                            }
                        }
                    }
                    if unsupported {
                        self.notice = "G and wildcard search options are not yet qualified".into();
                    } else {
                        self.find_dialog = Some(FindDialog::Options { term, flags });
                    }
                }
            },
            FindDialog::ReplaceTerm(mut term) => match key {
                Key::Named(Named::Enter) if !term.is_empty() => {
                    self.find_dialog = Some(FindDialog::ReplaceWith {
                        term,
                        value: String::new(),
                    });
                }
                Key::Named(Named::Backspace | Named::Delete) => {
                    term.pop();
                    self.find_dialog = Some(FindDialog::ReplaceTerm(term));
                }
                _ => {
                    if !modifiers.control()
                        && !modifiers.alt()
                        && !modifiers.logo()
                        && let Some(value) = text
                        && value.bytes().all(|byte| byte.is_ascii_alphabetic())
                        && term.len() + value.len() <= 64
                    {
                        term.push_str(value);
                    }
                    self.find_dialog = Some(FindDialog::ReplaceTerm(term));
                }
            },
            FindDialog::ReplaceWith { term, mut value } => match key {
                Key::Named(Named::Enter) if !value.is_empty() => {
                    self.find_dialog = Some(FindDialog::ReplaceOptions {
                        term,
                        value,
                        flags: String::new(),
                    });
                }
                Key::Named(Named::Backspace | Named::Delete) => {
                    value.pop();
                    self.find_dialog = Some(FindDialog::ReplaceWith { term, value });
                }
                _ => {
                    if !modifiers.control()
                        && !modifiers.alt()
                        && !modifiers.logo()
                        && let Some(input) = text
                        && input.bytes().all(|byte| byte.is_ascii_alphabetic())
                        && value.len() + input.len() <= 64
                    {
                        value.push_str(input);
                    }
                    self.find_dialog = Some(FindDialog::ReplaceWith { term, value });
                }
            },
            FindDialog::ReplaceOptions {
                term,
                value,
                mut flags,
            } => match key {
                Key::Named(Named::Enter) => {
                    let command = openstar_core::Command::ReplaceLiteral {
                        text: term.clone(),
                        replacement: value,
                        whole_words: flags.contains('W'),
                        ignore_case: flags.contains('U'),
                        without_asking: flags.contains('N'),
                    };
                    let result = self.session.apply_command(&command);
                    let ok = result.is_ok();
                    self.record_edit(result);
                    if ok && let DocumentSession::Legacy(session) = &self.session {
                        if session.editor.replacement_pending() {
                            self.find_dialog = Some(FindDialog::ReplaceConfirm);
                        } else if session.editor.last_find_result() == Some(false) {
                            self.find_dialog = Some(FindDialog::Notice {
                                term,
                                repeated: false,
                                replace: true,
                            });
                        }
                    }
                }
                Key::Named(Named::Backspace | Named::Delete) => {
                    flags.pop();
                    self.find_dialog = Some(FindDialog::ReplaceOptions { term, value, flags });
                }
                _ => {
                    if !modifiers.control()
                        && !modifiers.alt()
                        && !modifiers.logo()
                        && let Some(input) = text
                    {
                        for ch in input.chars().map(|ch| ch.to_ascii_uppercase()) {
                            if "WUN".contains(ch) && !flags.contains(ch) {
                                flags.push(ch);
                            } else if "BG?AR".contains(ch) {
                                self.notice = "This replacement option is not yet qualified".into();
                            }
                        }
                    }
                    self.find_dialog = Some(FindDialog::ReplaceOptions { term, value, flags });
                }
            },
            FindDialog::ReplaceConfirm => {
                let accept = match key.as_ref() {
                    Key::Character(value) if value.eq_ignore_ascii_case("y") => true,
                    Key::Character(value) if value.eq_ignore_ascii_case("n") => false,
                    _ => {
                        self.find_dialog = Some(FindDialog::ReplaceConfirm);
                        return;
                    }
                };
                let result = self
                    .session
                    .apply_command(&openstar_core::Command::AnswerReplace { accept });
                let ok = result.is_ok();
                self.record_edit(result);
                if !ok {
                    self.find_dialog = Some(FindDialog::ReplaceConfirm);
                }
            }
            notice @ FindDialog::Notice { .. } => {
                self.find_dialog = Some(notice);
            }
        }
    }
    fn new() -> Self {
        match RecoveryStore::default_location() {
            Ok(store) => Self::new_with_recovery(Some(store), None),
            Err(error) => Self::new_with_recovery(None, Some(error.to_string())),
        }
    }

    fn new_with_recovery(
        recovery_store: Option<RecoveryStore>,
        recovery_error: Option<String>,
    ) -> Self {
        let mut session = DocumentSession::Legacy(Session::default());
        let mut notice = recovery_error
            .map(|error| format!("Recovery unavailable: {error}"))
            .unwrap_or_default();
        let mut opening = true;
        let mut args = std::env::args_os().skip(1);
        if let Some(first) = args.next() {
            let utf8 = first == "--utf8";
            let path = if utf8 { args.next() } else { Some(first) };
            if let Some(path) = path.filter(|_| args.next().is_none()) {
                let path = PathBuf::from(path);
                let opened = if utf8 {
                    Utf8Session::open(&path).map(DocumentSession::Utf8)
                } else {
                    Session::open(&path).map(DocumentSession::Legacy)
                };
                match opened {
                    Ok(value) => {
                        session = value;
                        opening = false;
                    }
                    Err(e) => notice = format!("Open failed: {e}"),
                }
            } else {
                notice = "Usage: openstar-desktop [--utf8] FILE".into();
            }
        }
        let projection = Projection::for_session(&session);
        let mut app = Self {
            session,
            projection,
            keyboard: Keyboard::default(),
            dialog: None,
            find_dialog: None,
            notice,
            window_focused: false,
            opening,
            fullscreen: false,
            classic_font: false,
            session_generation: 0,
            utf8_top_row: 0,
            preferred_column: None,
            recovery_store,
            recovery_id: RecoveryStore::new_id(),
            recovery_written: false,
            recovery_pending: false,
        };
        if app.opening || app.session.path().is_some() {
            app.offer_recovery(app.opening, false);
        }
        app
    }
    fn title(&self) -> String {
        let path = self
            .session
            .path()
            .map(|p| p.to_string_lossy())
            .unwrap_or("Untitled".into());
        format!(
            "{APP_TITLE}: {path}{}",
            if self.session.modified() { " *" } else { "" }
        )
    }
    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::event::listen_with(|event, status, id| {
                if matches!(event, Event::Window(_)) || status == iced::event::Status::Ignored {
                    Some(Message::Event(event, id))
                } else {
                    None
                }
            }),
            iced::time::every(Duration::from_secs(2)).map(|_| Message::RecoveryTick),
        ])
    }
    fn begin(&mut self, next: Next) -> Task<Message> {
        self.keyboard.reset();
        if self.session.modified() {
            self.dialog = Some(Dialog::Discard(next));
            Task::none()
        } else {
            self.proceed(next)
        }
    }
    fn proceed(&mut self, next: Next) -> Task<Message> {
        match next {
            Next::Close(id) => window::close(id),
            Next::Opening => {
                self.session = DocumentSession::Legacy(Session::default());
                self.session_generation += 1;
                self.projection = Projection::for_session(&self.session);
                self.utf8_top_row = 0;
                self.preferred_column = None;
                self.dialog = None;
                self.opening = true;
                self.notice.clear();
                self.recovery_id = RecoveryStore::new_id();
                self.recovery_written = false;
                self.recovery_pending = false;
                Task::none()
            }
        }
    }
    fn path_dialog(&mut self, kind: PathKind) -> Task<Message> {
        self.find_dialog = None;
        self.keyboard.reset();
        let value = match kind {
            PathKind::OpenLegacy | PathKind::OpenUtf8 => String::new(),
            PathKind::SaveAs { .. } => String::new(),
        };
        self.dialog = Some(Dialog::Path {
            kind,
            value,
            error: String::new(),
        });
        iced::widget::operation::focus("document-path")
    }
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open => return self.path_dialog(PathKind::OpenLegacy),
            Message::NewUtf8 => {
                self.find_dialog = None;
                self.session = DocumentSession::Utf8(Utf8Session::new_at(None));
                self.session_generation += 1;
                self.projection = Projection::for_session(&self.session);
                self.utf8_top_row = 0;
                self.preferred_column = None;
                self.opening = false;
                self.notice.clear();
                self.recovery_id = RecoveryStore::new_id();
                self.recovery_written = false;
                self.recovery_pending = false;
            }
            Message::OpenUtf8 => return self.path_dialog(PathKind::OpenUtf8),
            Message::ShowRecovery => self.offer_recovery(true, true),
            Message::RecoveryTick => self.flush_recovery(),
            Message::Recover => {
                if let Some(Dialog::Recovery { candidate, error }) = &mut self.dialog {
                    let store = self
                        .recovery_store
                        .as_ref()
                        .expect("recovery dialog requires a store");
                    let legacy = candidate.is_legacy();
                    let result = if legacy {
                        store.restore_legacy(candidate).map(|outcome| {
                            (DocumentSession::Legacy(outcome.session), outcome.detached)
                        })
                    } else {
                        store.restore(candidate).map(|outcome| {
                            (DocumentSession::Utf8(outcome.session), outcome.detached)
                        })
                    };
                    match result {
                        Ok((session, detached)) => {
                            self.recovery_id = candidate.id.clone();
                            self.recovery_written = true;
                            self.recovery_pending = false;
                            self.session = session;
                            self.session_generation += 1;
                            self.projection = Projection::for_session(&self.session);
                            self.utf8_top_row = 0;
                            self.preferred_column = None;
                            self.opening = false;
                            self.dialog = None;
                            self.notice = if detached {
                                "Original file changed or is missing. Recovered copy needs Save As."
                                    .into()
                            } else if legacy {
                                "Unsaved legacy copy recovered. Save it when ready.".into()
                            } else {
                                "Unsaved UTF-8 draft recovered. Save it when ready.".into()
                            };
                        }
                        Err(problem) => *error = format!("Recovery failed: {problem}"),
                    }
                }
            }
            Message::SkipRecovery => self.dialog = None,
            Message::DiscardRecovery => {
                if let Some(Dialog::Recovery { candidate, error }) = &mut self.dialog {
                    match self
                        .recovery_store
                        .as_ref()
                        .expect("recovery dialog requires a store")
                        .remove(&candidate.id)
                    {
                        Ok(()) => self.dialog = None,
                        Err(problem) => *error = format!("Could not discard copy: {problem}"),
                    }
                }
            }
            Message::Save(done) => {
                if self.session.path().is_none() {
                    return self.path_dialog(PathKind::SaveAs { done });
                }
                match self.session.save() {
                    Ok(()) => {
                        self.notice.clear();
                        if let Err(error) = self.clear_recovery() {
                            self.notice = format!("Saved, but recovery cleanup failed: {error}");
                            return Task::none();
                        }
                        if done {
                            return self.proceed(Next::Opening);
                        }
                    }
                    Err(e) => self.notice = format!("Save failed: {e}"),
                }
            }
            Message::SaveAs(done) => return self.path_dialog(PathKind::SaveAs { done }),
            Message::Help => {
                self.keyboard.reset();
                self.dialog = Some(Dialog::Help);
            }
            Message::PathChanged(value) => {
                if let Some(Dialog::Path {
                    value: current,
                    error,
                    ..
                }) = &mut self.dialog
                {
                    *current = value;
                    error.clear();
                }
            }
            Message::Submit => {
                let mut opened_document = false;
                if let Some(Dialog::Path { kind, value, error }) = &mut self.dialog {
                    if value.is_empty() {
                        *error = "Enter a file path".into();
                        return Task::none();
                    }
                    let path = PathBuf::from(&*value);
                    let result = match kind {
                        PathKind::OpenLegacy => match Session::open(&path) {
                            Ok(opened) => {
                                self.session = DocumentSession::Legacy(opened);
                                Ok(())
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                self.session = DocumentSession::Legacy(Session::new_at(path));
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        PathKind::OpenUtf8 => match Utf8Session::open(&path) {
                            Ok(opened) => {
                                self.session = DocumentSession::Utf8(opened);
                                Ok(())
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                self.session =
                                    DocumentSession::Utf8(Utf8Session::new_at(Some(path)));
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        PathKind::SaveAs { .. } => self.session.save_as(&path),
                    };
                    match result {
                        Ok(()) => {
                            self.find_dialog = None;
                            opened_document =
                                matches!(kind, PathKind::OpenLegacy | PathKind::OpenUtf8);
                            let saved_copy = matches!(kind, PathKind::SaveAs { .. });
                            if matches!(kind, PathKind::OpenLegacy | PathKind::OpenUtf8) {
                                self.session_generation += 1;
                                self.utf8_top_row = 0;
                                self.preferred_column = None;
                                self.recovery_id = RecoveryStore::new_id();
                                self.recovery_written = false;
                                self.recovery_pending = false;
                            }
                            let done = matches!(kind, PathKind::SaveAs { done: true });
                            self.opening = false;
                            self.dialog = None;
                            self.notice.clear();
                            self.projection = Projection::for_session(&self.session);
                            if saved_copy && let Err(problem) = self.clear_recovery() {
                                self.notice =
                                    format!("Saved, but recovery cleanup failed: {problem}");
                                return Task::none();
                            }
                            if done {
                                return self.proceed(Next::Opening);
                            }
                        }
                        Err(e) => {
                            *error = if e.kind() == std::io::ErrorKind::AlreadyExists {
                                "File exists. Choose a new path for Save As.".into()
                            } else {
                                format!("{e}. Document retained; no new file was published.")
                            }
                        }
                    }
                }
                if opened_document {
                    self.offer_recovery(false, false);
                }
            }
            Message::Cancel => {
                self.dialog = None;
                self.keyboard.reset();
                self.notice.clear();
            }
            Message::Discard => {
                if let Some(Dialog::Discard(next)) = self.dialog.take() {
                    let _ = self.clear_recovery();
                    return self.proceed(next);
                }
            }
            Message::Paste(generation, Some(text))
                if generation == self.session_generation
                    && !self.opening
                    && self.dialog.is_none()
                    && self.session.is_utf8() =>
            {
                let result = self.session.insert_text(&text);
                self.record_edit(result);
            }
            Message::Paste(_, _) => {}
            Message::Event(Event::Window(window::Event::CloseRequested), id) => {
                return self.begin(Next::Close(id));
            }
            Message::Event(Event::Window(window::Event::Unfocused), _) => {
                self.window_focused = false;
                self.keyboard.reset();
            }
            Message::Event(Event::Window(window::Event::Focused), _) => self.window_focused = true,
            Message::Event(Event::Keyboard(event), id) => {
                use keyboard::{Event::KeyPressed, Key, key::Named};
                if self.window_focused
                    && let KeyPressed {
                        key: Key::Named(key),
                        repeat: false,
                        ..
                    } = &event
                {
                    match key {
                        Named::F11 => {
                            self.fullscreen = !self.fullscreen;
                            return window::set_mode(
                                id,
                                if self.fullscreen {
                                    window::Mode::Fullscreen
                                } else {
                                    window::Mode::Windowed
                                },
                            );
                        }
                        Named::F12 => {
                            self.classic_font = !self.classic_font;
                            return Task::none();
                        }
                        _ => {}
                    }
                }
                if self.find_dialog.is_some() && self.window_focused {
                    self.handle_find_event(&event);
                    return Task::none();
                }
                if self.dialog.is_some() {
                    if matches!(
                        event,
                        KeyPressed {
                            key: Key::Named(Named::Escape),
                            ..
                        }
                    ) {
                        return self.update(Message::Cancel);
                    }
                    if self.window_focused
                        && let KeyPressed {
                            key: Key::Character(key),
                            modifiers,
                            repeat: false,
                            ..
                        } = &event
                        && !modifiers.control()
                        && !modifiers.alt()
                        && !modifiers.logo()
                    {
                        let message = match &self.dialog {
                            Some(Dialog::Discard(_)) if key.eq_ignore_ascii_case("y") => {
                                Some(Message::Discard)
                            }
                            Some(Dialog::Discard(_)) if key.eq_ignore_ascii_case("n") => {
                                Some(Message::Cancel)
                            }
                            Some(Dialog::Recovery { .. }) if key.eq_ignore_ascii_case("r") => {
                                Some(Message::Recover)
                            }
                            Some(Dialog::Recovery { .. }) if key.eq_ignore_ascii_case("l") => {
                                Some(Message::SkipRecovery)
                            }
                            Some(Dialog::Recovery { .. }) if key.eq_ignore_ascii_case("d") => {
                                Some(Message::DiscardRecovery)
                            }
                            _ => None,
                        };
                        if let Some(message) = message {
                            return self.update(message);
                        }
                    }
                } else if self.window_focused {
                    if self.opening {
                        if let KeyPressed {
                            key,
                            repeat: false,
                            modifiers,
                            ..
                        } = event
                        {
                            if modifiers.control() || modifiers.alt() || modifiers.logo() {
                                return Task::none();
                            }
                            match key.as_ref(){
                                Key::Character(s) if s.eq_ignore_ascii_case("d")=>return self.update(Message::Open),
                                Key::Character(s) if s.eq_ignore_ascii_case("x")=>return self.begin(Next::Close(id)),
                                Key::Character(s) if s.eq_ignore_ascii_case("j")=>return self.update(Message::Help),
                                Key::Named(Named::F1)=>return self.update(Message::Help),
                                Key::Named(Named::F2)=>return self.update(Message::NewUtf8),
                                Key::Named(Named::F3)=>return self.update(Message::OpenUtf8),
                                Key::Named(Named::F4)=>return self.update(Message::ShowRecovery),
                                _=>self.notice="This opening-menu command is not implemented. D opens a document.".into(),
                            }
                        }
                    } else {
                        match self.keyboard.handle_mode(&event, self.session.is_utf8()) {
                            Some(Action::Edit(command)) => {
                                let result = self.session.apply_command(&command);
                                self.record_edit(result);
                            }
                            Some(Action::Find) => {
                                self.find_dialog = Some(FindDialog::Term(String::new()));
                                self.notice.clear();
                            }
                            Some(Action::Replace) => {
                                self.find_dialog = Some(FindDialog::ReplaceTerm(String::new()));
                                self.notice.clear();
                            }
                            Some(Action::RepeatFind) => {
                                let term = match &self.session {
                                    DocumentSession::Legacy(session) => {
                                        session.editor.last_find_text()
                                    }
                                    DocumentSession::Utf8(_) => None,
                                };
                                let result = self
                                    .session
                                    .apply_command(&openstar_core::Command::RepeatFind);
                                self.record_edit(result);
                                if let (Some(term), DocumentSession::Legacy(session)) =
                                    (term, &self.session)
                                {
                                    if session.editor.replacement_pending() {
                                        self.find_dialog = Some(FindDialog::ReplaceConfirm);
                                    } else if session.editor.last_find_result() == Some(false) {
                                        self.find_dialog = Some(FindDialog::Notice {
                                            term,
                                            repeated: true,
                                            replace: session.editor.last_search_replaces(),
                                        });
                                    }
                                }
                            }
                            Some(Action::Text(text)) => {
                                let result = self.session.insert_text(&text);
                                self.record_edit(result);
                            }
                            Some(Action::Backspace) => {
                                let result = self.session.backspace();
                                self.record_edit(result);
                            }
                            Some(Action::Delete) => {
                                let result = self.session.delete();
                                self.record_edit(result);
                            }
                            Some(Action::Undo) => {
                                let result = self.session.undo();
                                self.record_history(result, "Nothing to undo");
                            }
                            Some(Action::Redo) => {
                                let result = self.session.redo();
                                self.record_history(result, "Nothing to redo");
                            }
                            Some(Action::MoveVertical(direction)) => {
                                self.move_utf8_vertical(direction);
                            }
                            Some(Action::Paste) => {
                                let generation = self.session_generation;
                                return clipboard::read()
                                    .map(move |text| Message::Paste(generation, text));
                            }
                            Some(Action::Save) => return self.update(Message::Save(false)),
                            Some(Action::SaveAs) => return self.update(Message::SaveAs(false)),
                            Some(Action::SaveAndDone) => return self.update(Message::Save(true)),
                            Some(Action::Done) => return self.begin(Next::Opening),
                            Some(Action::Help) => return self.update(Message::Help),
                            Some(Action::Notice(message)) => self.notice = message.into(),
                            None => {}
                        }
                    }
                }
            }
            Message::Event(
                Event::InputMethod(iced::advanced::input_method::Event::Commit(text)),
                _,
            ) if self.dialog.is_none()
                && self.window_focused
                && !self.opening
                && self.session.is_utf8() =>
            {
                let result = self.session.insert_text(&text);
                self.record_edit(result);
            }
            Message::Event(Event::InputMethod(_), _)
                if self.dialog.is_none() && !self.session.is_utf8() =>
            {
                self.notice = "IME is not supported in the legacy ASCII prototype".into();
            }
            _ => {}
        }
        Task::none()
    }
    fn record_edit(&mut self, result: Result<(), String>) {
        match result {
            Ok(()) => {
                self.projection = Projection::for_session(&self.session);
                self.preferred_column = None;
                self.keep_utf8_cursor_visible();
                self.notice.clear();
                self.mark_recovery_changed();
            }
            Err(message) => self.notice = message,
        }
    }

    fn record_history(&mut self, result: Result<bool, String>, empty_notice: &str) {
        match result {
            Ok(true) => self.record_edit(Ok(())),
            Ok(false) => self.notice = empty_notice.into(),
            Err(message) => self.notice = message,
        }
    }

    fn offer_recovery(&mut self, all: bool, announce_empty: bool) {
        let Some(store) = &self.recovery_store else {
            self.notice = "Recovery storage is unavailable".into();
            return;
        };
        let path = if all { None } else { self.session.path() };
        let scan = if let Some(path) = path {
            store.find_for_mode(path, !self.session.is_utf8())
        } else {
            store.find(None)
        };
        match scan {
            Ok(scan) => {
                if let Some(candidate) = scan.candidate {
                    self.dialog = Some(Dialog::Recovery {
                        candidate: Box::new(candidate),
                        error: String::new(),
                    });
                } else if announce_empty {
                    self.notice = "No unsaved draft found".into();
                }
                if scan.invalid_files > 0 {
                    self.notice = format!(
                        "{} unreadable recovery file(s) retained in recovery storage",
                        scan.invalid_files
                    );
                }
            }
            Err(error) => self.notice = format!("Recovery scan failed: {error}"),
        }
    }

    fn mark_recovery_changed(&mut self) {
        if !self.session.modified() {
            if let Err(error) = self.clear_recovery() {
                self.notice = format!("Recovery cleanup failed: {error}");
            }
            return;
        }
        self.recovery_pending = true;
        if !self.recovery_written {
            self.flush_recovery();
        }
    }

    fn flush_recovery(&mut self) {
        if !self.recovery_pending || !self.session.modified() {
            return;
        }
        let Some(store) = &self.recovery_store else {
            return;
        };
        let result = match &self.session {
            DocumentSession::Legacy(session) => store.write_legacy(&self.recovery_id, session),
            DocumentSession::Utf8(session) => store.write(&self.recovery_id, session),
        };
        match result {
            Ok(()) => {
                self.recovery_written = true;
                self.recovery_pending = false;
            }
            Err(error) => self.notice = format!("Recovery snapshot failed: {error}"),
        }
    }

    fn clear_recovery(&mut self) -> std::io::Result<()> {
        if self.recovery_written
            && let Some(store) = &self.recovery_store
        {
            store.remove(&self.recovery_id)?;
        }
        self.recovery_written = false;
        self.recovery_pending = false;
        Ok(())
    }

    fn keep_utf8_cursor_visible(&mut self) {
        if !self.session.is_utf8() {
            return;
        }
        let row = self.projection.cursor.0;
        if row < self.utf8_top_row {
            self.utf8_top_row = row;
        } else if row >= self.utf8_top_row + UTF8_VISIBLE_ROWS {
            self.utf8_top_row = row + 1 - UTF8_VISIBLE_ROWS;
        }
    }

    fn move_utf8_vertical(&mut self, direction: isize) {
        let column = self.preferred_column.unwrap_or(self.projection.cursor.1);
        let Some(target) = self.projection.utf8_vertical_target(direction, column) else {
            return;
        };
        let DocumentSession::Utf8(session) = &mut self.session else {
            return;
        };
        session
            .editor
            .set_cursor(target)
            .expect("projection cursor stops are grapheme boundaries");
        self.projection = Projection::utf8(&session.editor);
        self.preferred_column = Some(column);
        self.keep_utf8_cursor_visible();
        self.notice.clear();
        self.mark_recovery_changed();
    }
    fn dialog_view(&self, viewport: iced::Size) -> Element<'_, Message> {
        let surface::Typography {
            font,
            size,
            row_height: cell_height,
            grid_width,
            origin_x,
            origin_y,
            column_width,
        } = surface::typography(viewport, self.classic_font);
        let dialog = self
            .dialog
            .as_ref()
            .expect("dialog view requires an open dialog");
        let command = match dialog {
            Dialog::Path {
                kind: PathKind::OpenLegacy,
                ..
            } => "D",
            Dialog::Path {
                kind: PathKind::OpenUtf8,
                ..
            } => "F3",
            Dialog::Path {
                kind: PathKind::SaveAs { .. },
                ..
            } => "K",
            Dialog::Discard(_) => "?",
            Dialog::Recovery { .. } => "R",
            Dialog::Help => unreachable!("help uses the screen grid"),
        };
        let mut screen = Screen::dialogue(command);
        match dialog {
            Dialog::Path { kind, error, .. } => {
                let (description, prompt) = match kind {
                    PathKind::OpenLegacy => (
                        "Type the name of the document to create or change.",
                        "Document to open?",
                    ),
                    PathKind::OpenUtf8 => (
                        "Type the name of the UTF-8 document to open or create.",
                        "Document to open?",
                    ),
                    PathKind::SaveAs { .. } => (
                        "Save a copy under a new name. Existing files stay unchanged.",
                        "New file name?",
                    ),
                };
                screen.put(2, 0, description, 7);
                screen.put(3, 0, "Include the drive and directory if needed.", 7);
                screen.put(8, 0, prompt, 7);
                screen.put(9, 0, "Enter done    Esc cancel    Backspace erase left", 7);
                screen.put(10, 0, "Ctrl+A select all    Ctrl+V paste", 7);
                screen.put(13, 0, error, 7);
            }
            Dialog::Discard(_) => {
                screen.put(3, 0, "The document has unsaved changes.", 7);
                screen.put(
                    5,
                    0,
                    "Save your work before leaving, or discard these changes.",
                    7,
                );
                screen.put(8, 0, "Discard changes (Y/N)?", 7);
                screen.put(10, 0, "Y discard    N or Esc keep editing", 7);
            }
            Dialog::Recovery { candidate, error } => {
                screen.put(
                    2,
                    0,
                    if candidate.is_legacy() {
                        "An unsaved document copy is available."
                    } else {
                        "An unsaved UTF-8 draft is available."
                    },
                    7,
                );
                let path = candidate
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "Untitled document".into());
                for (line, chunk) in path
                    .chars()
                    .collect::<Vec<_>>()
                    .chunks(79)
                    .take(3)
                    .enumerate()
                {
                    screen.put(4 + line, 0, &chunk.iter().collect::<String>(), 7);
                }
                if path.chars().count() > 79 * 3 {
                    screen.put(6, 76, "...", 7);
                }
                screen.put(
                    8,
                    0,
                    "Restore the copy without writing the original file?",
                    7,
                );
                screen.put(10, 0, "R restore    L later    D discard copy", 7);
                screen.put(
                    12,
                    0,
                    "A changed or missing original opens without a file name.",
                    7,
                );
                screen.put(14, 0, error, 7);
            }
            Dialog::Help => unreachable!("help uses the screen grid"),
        }
        let canvas = canvas(surface::Surface {
            screen,
            focused: self.window_focused,
            classic_font: self.classic_font,
        })
        .width(Fill)
        .height(Fill);
        if let Dialog::Path { value, .. } = dialog {
            let input = text_input("", value)
                .id("document-path")
                .on_input(Message::PathChanged)
                .on_submit(Message::Submit)
                .font(font)
                .size(size)
                .line_height(iced::Pixels(cell_height))
                .padding(0)
                .width(grid_width - 19.0 * column_width)
                .style(|_, _| iced::widget::text_input::Style {
                    background: iced::Background::Color(iced::Color::BLACK),
                    border: iced::Border::default(),
                    icon: iced::Color::WHITE,
                    placeholder: iced::Color::from_rgb8(170, 170, 170),
                    value: iced::Color::from_rgb8(170, 170, 170),
                    selection: iced::Color::from_rgb8(170, 170, 170),
                });
            let overlay = column![
                space().height(origin_y + 8.0 * cell_height),
                row![space().width(origin_x + 18.0 * column_width), input]
            ];
            stack![canvas, overlay].width(Fill).height(Fill).into()
        } else {
            canvas.into()
        }
    }

    fn view(&self) -> Element<'_, Message> {
        if self
            .dialog
            .as_ref()
            .is_some_and(|dialog| !matches!(dialog, Dialog::Help))
        {
            return responsive(|viewport| self.dialog_view(viewport)).into();
        }
        let mut screen = if self.opening {
            let mut screen = Screen::opening();
            screen.put(
                13,
                0,
                "F2 new UTF-8 document   F3 open UTF-8 file   F4 recovery",
                7,
            );
            screen
        } else {
            let label = self
                .session
                .path()
                .map(|p| {
                    let name = p.file_name().unwrap_or_default().to_string_lossy();
                    let drive = match p.components().next() {
                        Some(std::path::Component::Prefix(prefix)) => match prefix.kind() {
                            std::path::Prefix::Disk(d) | std::path::Prefix::VerbatimDisk(d) => {
                                format!("{}:", d as char)
                            }
                            _ => String::new(),
                        },
                        _ => String::new(),
                    };
                    format!("{drive}{name}")
                })
                .unwrap_or_else(|| "UNTITLED".into());
            match &self.session {
                DocumentSession::Legacy(session) => Screen::editor(
                    &self.projection,
                    &session.editor.snapshot(),
                    &label,
                    self.keyboard.prefix(),
                ),
                DocumentSession::Utf8(_) => Screen::utf8_editor(
                    &self.projection,
                    &label,
                    self.keyboard.prefix(),
                    self.utf8_top_row,
                ),
            }
        };
        let show_help = matches!(self.dialog, Some(Dialog::Help));
        if show_help {
            if self.opening {
                screen.opening_help();
            } else {
                screen.editor_help(self.session.is_utf8());
            }
        } else if let Some(find) = &self.find_dialog {
            match find {
                FindDialog::Term(term) => screen.find_prompt(term, None),
                FindDialog::Options { term, flags } => screen.find_prompt(term, Some(flags)),
                FindDialog::ReplaceTerm(term) => screen.replace_prompt(term, None, None),
                FindDialog::ReplaceWith { term, value } => {
                    screen.replace_prompt(term, Some(value), None)
                }
                FindDialog::ReplaceOptions { term, value, flags } => {
                    screen.replace_prompt(term, Some(value), Some(flags))
                }
                FindDialog::ReplaceConfirm => screen.replace_question(),
                FindDialog::Notice {
                    term,
                    repeated,
                    replace,
                } => {
                    screen.find_notice(term, *repeated);
                    if *replace && !*repeated {
                        screen.cells[0][1].glyph = 'A';
                    }
                }
            }
            if !self.notice.is_empty() {
                screen.put(9, 3, &self.notice, 7);
            }
        }
        if !show_help && self.find_dialog.is_none() && !self.notice.is_empty() {
            screen.cells[22].fill(openstar_desktop::screen::Cell {
                glyph: ' ',
                attribute: 0x70,
            });
            screen.put(22, 0, &self.notice, 0x70);
        } else if !show_help && self.projection.hidden_tail_bytes > 0 && !self.opening {
            screen.put(
                22,
                0,
                "Data after EOF is preserved but not displayed.",
                0x70,
            );
        }
        canvas(surface::Surface {
            screen,
            focused: self.window_focused,
            classic_font: self.classic_font,
        })
        .width(Fill)
        .height(Fill)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::{
        Key, Location, Modifiers,
        key::{Named, NativeCode, Physical},
    };
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static SERIAL: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "openstar-app-recovery-{}-{}",
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

    fn key(key: Key, control: bool, text: Option<&str>) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            modified_key: key.clone(),
            key,
            physical_key: Physical::Unidentified(NativeCode::Unidentified),
            location: Location::Standard,
            modifiers: if control {
                Modifiers::CTRL
            } else {
                Modifiers::empty()
            },
            text: text.map(Into::into),
            repeat: false,
        })
    }

    #[test]
    fn help_and_discard_prompts_keep_keyboard_navigation() {
        let mut app = App::new_with_recovery(None, None);
        app.window_focused = true;
        let id = window::Id::unique();
        let _ = app.update(Message::Event(
            key(Key::Character("j".into()), false, Some("j")),
            id,
        ));
        assert!(matches!(app.dialog, Some(Dialog::Help)));
        let _ = app.view();
        let _ = app.update(Message::Event(
            key(Key::Named(Named::Escape), false, None),
            id,
        ));
        assert!(app.dialog.is_none());

        app.dialog = Some(Dialog::Discard(Next::Opening));
        let _ = app.update(Message::Event(
            key(Key::Character("n".into()), false, Some("n")),
            id,
        ));
        assert!(app.dialog.is_none());
    }

    #[test]
    fn legacy_find_prompt_options_repeat_and_failure_flow_through_app() {
        let scratch = Scratch::new();
        let path = scratch.0.join("find.doc");
        let mut carrier = b"CAT cat CAT".to_vec();
        carrier.resize(128, 0x1a);
        fs::write(&path, &carrier).unwrap();
        let mut app = App::new_with_recovery(None, None);
        app.session = DocumentSession::Legacy(Session::open(&path).unwrap());
        app.projection = Projection::for_session(&app.session);
        app.opening = false;
        app.window_focused = true;
        let id = window::Id::unique();
        for event in [
            key(Key::Character("q".into()), true, None),
            key(Key::Character("f".into()), false, Some("f")),
            key(Key::Character("c".into()), false, Some("cat")),
            key(Key::Named(Named::Enter), false, None),
            key(Key::Character("u".into()), false, Some("u")),
            key(Key::Named(Named::Enter), false, None),
        ] {
            let _ = app.update(Message::Event(event, id));
        }
        let DocumentSession::Legacy(session) = &app.session else {
            panic!()
        };
        assert_eq!(session.editor.snapshot().cursor_byte, 0);
        assert!(app.find_dialog.is_none());
        let _ = app.update(Message::Event(
            key(Key::Character("l".into()), true, None),
            id,
        ));
        let DocumentSession::Legacy(session) = &app.session else {
            panic!()
        };
        assert_eq!(session.editor.snapshot().cursor_byte, 4);
        let _ = app.update(Message::Event(
            key(Key::Character("l".into()), true, None),
            id,
        ));
        let _ = app.update(Message::Event(
            key(Key::Character("l".into()), true, None),
            id,
        ));
        assert!(matches!(
            app.find_dialog,
            Some(FindDialog::Notice { repeated: true, .. })
        ));
        let _ = app.update(Message::Event(
            key(Key::Named(Named::Escape), false, None),
            id,
        ));
        assert!(app.find_dialog.is_none());
        let DocumentSession::Legacy(session) = &app.session else {
            panic!()
        };
        assert_eq!(session.editor.snapshot().cursor_byte, 11);
        assert_eq!(fs::read(&path).unwrap(), carrier);
    }

    #[test]
    fn legacy_replace_prompt_answers_and_repeats_through_app() {
        let scratch = Scratch::new();
        let path = scratch.0.join("replace.doc");
        let mut carrier = b"CAT CATALOG cat".to_vec();
        carrier.resize(128, 0x1a);
        fs::write(&path, &carrier).unwrap();
        let mut app = App::new_with_recovery(None, None);
        app.session = DocumentSession::Legacy(Session::open(&path).unwrap());
        app.projection = Projection::for_session(&app.session);
        app.opening = false;
        app.window_focused = true;
        let id = window::Id::unique();
        for event in [
            key(Key::Character("q".into()), true, None),
            key(Key::Character("a".into()), false, Some("a")),
            key(Key::Character("c".into()), false, Some("cat")),
            key(Key::Named(Named::Enter), false, None),
            key(Key::Character("d".into()), false, Some("DOG")),
            key(Key::Named(Named::Enter), false, None),
            key(Key::Character("w".into()), false, Some("w")),
            key(Key::Character("u".into()), false, Some("u")),
            key(Key::Named(Named::Enter), false, None),
        ] {
            let _ = app.update(Message::Event(event, id));
        }
        assert!(matches!(app.find_dialog, Some(FindDialog::ReplaceConfirm)));
        let DocumentSession::Legacy(session) = &app.session else {
            panic!()
        };
        assert_eq!(session.editor.snapshot().cursor_byte, 0);
        assert!(session.editor.replacement_pending());
        let _ = app.update(Message::Event(
            key(Key::Character("y".into()), false, Some("y")),
            id,
        ));
        assert!(app.find_dialog.is_none());
        let _ = app.update(Message::Event(
            key(Key::Character("l".into()), true, None),
            id,
        ));
        assert!(matches!(app.find_dialog, Some(FindDialog::ReplaceConfirm)));
        let DocumentSession::Legacy(session) = &app.session else {
            panic!()
        };
        assert_eq!(session.editor.snapshot().cursor_byte, 12);
        let _ = app.update(Message::Event(
            key(Key::Character("y".into()), false, Some("y")),
            id,
        ));
        assert!(app.find_dialog.is_none());
        app.session.save().unwrap();
        let mut expected = b"DOG CATALOG DOG".to_vec();
        expected.resize(128, 0x1a);
        assert_eq!(fs::read(path).unwrap(), expected);
    }

    #[test]
    fn app_snapshots_unsaved_utf8_text_and_offers_it_after_restart() {
        let scratch = Scratch::new();
        let root = scratch.0.join("recovery");
        let mut app = App::new_with_recovery(Some(RecoveryStore::new(root.clone())), None);
        let _ = app.update(Message::NewUtf8);
        let result = app.session.insert_text("æøå");
        app.record_edit(result);
        assert!(app.recovery_written);
        let result = app.session.insert_text(" B");
        app.record_edit(result);
        assert!(app.recovery_pending);
        let first = RecoveryStore::new(root.clone())
            .find(None)
            .unwrap()
            .candidate
            .unwrap();
        assert_eq!(
            RecoveryStore::new(root.clone())
                .restore(&first)
                .unwrap()
                .session
                .editor
                .text(),
            "æøå"
        );
        let _ = app.update(Message::RecoveryTick);
        drop(app);

        let mut restarted = App::new_with_recovery(Some(RecoveryStore::new(root.clone())), None);
        assert!(matches!(restarted.dialog, Some(Dialog::Recovery { .. })));
        let _ = restarted.update(Message::Recover);
        let DocumentSession::Utf8(session) = &restarted.session else {
            panic!("Recovery must open the UTF-8 editor")
        };
        assert_eq!(session.editor.text(), "æøå B");
        assert!(restarted.session.modified());
        let copy = scratch.0.join("recovered.txt");
        restarted.session.save_as(&copy).unwrap();
        restarted.clear_recovery().unwrap();
        assert_eq!(fs::read(copy).unwrap(), "æøå B".as_bytes());
        assert!(
            RecoveryStore::new(root)
                .find(None)
                .unwrap()
                .candidate
                .is_none()
        );
    }

    #[test]
    fn app_restores_unsaved_legacy_carrier_after_restart_without_writing_source() {
        let scratch = Scratch::new();
        let root = scratch.0.join("recovery");
        let path = scratch.0.join("chapter.doc");
        let mut original = b"ALPHA\r\nBETA".to_vec();
        original.resize(128, 0x1a);
        fs::write(&path, &original).unwrap();
        let mut app = App::new_with_recovery(Some(RecoveryStore::new(root.clone())), None);
        app.session = DocumentSession::Legacy(Session::open(&path).unwrap());
        app.projection = Projection::for_session(&app.session);
        app.opening = false;
        let result = app.session.insert_text("X");
        app.record_edit(result);
        assert!(app.recovery_written);
        let result = app.session.insert_text("Y");
        app.record_edit(result);
        assert!(app.recovery_pending);
        let _ = app.update(Message::RecoveryTick);
        assert_eq!(fs::read(&path).unwrap(), original);
        drop(app);

        let mut restarted = App::new_with_recovery(Some(RecoveryStore::new(root.clone())), None);
        assert!(matches!(restarted.dialog, Some(Dialog::Recovery { .. })));
        let _ = restarted.update(Message::Recover);
        let DocumentSession::Legacy(session) = &restarted.session else {
            panic!("Recovery must open the legacy editor")
        };
        let mut expected = b"XYALPHA\r\nBETA".to_vec();
        expected.resize(128, 0x1a);
        assert_eq!(session.editor.document().bytes(), expected);
        assert!(session.editor.snapshot().modified);
        assert_eq!(fs::read(&path).unwrap(), original);
        let _ = restarted.update(Message::Save(false));
        assert_eq!(fs::read(&path).unwrap(), expected);
        assert_eq!(
            Session::open(&path).unwrap().editor.document().bytes(),
            expected
        );
        assert_eq!(fs::read(scratch.0.join("chapter.BAK")).unwrap(), original);
        assert!(
            RecoveryStore::new(root)
                .find(None)
                .unwrap()
                .candidate
                .is_none()
        );
    }

    #[test]
    fn modern_mode_accepts_ime_commit_and_ignores_late_clipboard_result() {
        let mut app = App::new_with_recovery(None, None);
        let _ = app.update(Message::NewUtf8);
        app.window_focused = true;
        let id = window::Id::unique();
        let _ = app.update(Message::Event(
            Event::InputMethod(iced::advanced::input_method::Event::Commit("æøå".into())),
            id,
        ));
        let DocumentSession::Utf8(session) = &app.session else {
            panic!("F2 must select UTF-8 mode")
        };
        assert_eq!(session.editor.text(), "æøå");
        assert!(app.title().ends_with(" *"));

        let old_generation = app.session_generation;
        let _ = app.update(Message::NewUtf8);
        let _ = app.update(Message::Paste(old_generation, Some("stale".into())));
        let DocumentSession::Utf8(session) = &app.session else {
            panic!()
        };
        assert!(session.editor.text().is_empty());
    }

    #[test]
    fn vertical_movement_keeps_the_preferred_column_and_scrolls_only_at_edges() {
        let mut app = App::new_with_recovery(None, None);
        let _ = app.update(Message::NewUtf8);
        let result = app.session.insert_text("123456789\nx\nabcdefghi");
        app.record_edit(result);
        assert_eq!(app.projection.cursor, (2, 9));
        app.move_utf8_vertical(-1);
        assert_eq!(app.projection.cursor, (1, 1));
        app.move_utf8_vertical(-1);
        assert_eq!(app.projection.cursor, (0, 9));
        assert_eq!(app.preferred_column, Some(9));
        let result = app
            .session
            .apply_command(&openstar_core::Command::PreviousCharacter);
        app.record_edit(result);
        assert_eq!(app.preferred_column, None);

        let _ = app.update(Message::NewUtf8);
        let result = app.session.insert_text(&"line\n".repeat(20));
        app.record_edit(result);
        assert_eq!(app.projection.cursor, (20, 0));
        assert_eq!(app.utf8_top_row, 9);
        app.move_utf8_vertical(-1);
        assert_eq!(app.utf8_top_row, 9);
        for _ in 0..11 {
            app.move_utf8_vertical(-1);
        }
        assert_eq!(app.projection.cursor, (8, 0));
        assert_eq!(app.utf8_top_row, 8);
        let screen = Screen::utf8_editor(&app.projection, "test.txt", None, app.utf8_top_row);
        assert_eq!(screen.cursor, Some((11, 0)));
    }
}
