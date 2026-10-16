// SPDX-License-Identifier: GPL-3.0-only
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use iced::{
    Element, Event, Fill, Font, Subscription, Task, Theme, clipboard, keyboard,
    widget::{button, canvas, column, container, row, text, text_input},
    window,
};
use northstar_desktop::{
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
                notice = "Usage: northstar-desktop [--utf8] FILE".into();
            }
        }
        let projection = Projection::for_session(&session);
        let mut app = Self {
            session,
            projection,
            keyboard: Keyboard::default(),
            dialog: None,
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
        if app.opening || app.session.is_utf8() {
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
            "NorthStar: {path}{}",
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
                    let result = self
                        .recovery_store
                        .as_ref()
                        .expect("recovery dialog requires a store")
                        .restore(candidate);
                    match result {
                        Ok(outcome) => {
                            self.recovery_id = candidate.id.clone();
                            self.recovery_written = true;
                            self.recovery_pending = false;
                            self.session = DocumentSession::Utf8(outcome.session);
                            self.session_generation += 1;
                            self.projection = Projection::for_session(&self.session);
                            self.utf8_top_row = 0;
                            self.preferred_column = None;
                            self.opening = false;
                            self.dialog = None;
                            self.notice = if outcome.detached {
                                "Original file changed or is missing. Recovered copy needs Save As."
                                    .into()
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
                let mut opened_utf8 = false;
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
                            opened_utf8 = matches!(kind, PathKind::OpenUtf8);
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
                if opened_utf8 {
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
        match store.find(path) {
            Ok(scan) => {
                if let Some(candidate) = scan.candidate {
                    self.dialog = Some(Dialog::Recovery {
                        candidate: Box::new(candidate),
                        error: String::new(),
                    });
                } else if announce_empty {
                    self.notice = "No unsaved UTF-8 draft found".into();
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
        if !self.session.is_utf8() {
            return;
        }
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
        let DocumentSession::Utf8(session) = &self.session else {
            return;
        };
        let Some(store) = &self.recovery_store else {
            return;
        };
        match store.write(&self.recovery_id, session) {
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
    fn view(&self) -> Element<'_, Message> {
        if let Some(dialog) = &self.dialog {
            let content:Element<'_,Message>=match dialog {
                Dialog::Path{kind,value,error}=>{
                    let (heading,hint)=match kind{
                        PathKind::OpenLegacy=>("Name of WordStar file to edit?","Full path; a missing file starts a new legacy document. Read limit: 1 MiB."),
                        PathKind::OpenUtf8=>("Name of UTF-8 file to edit?","Full path; a missing file starts a new UTF-8 document. Read limit: 1 MiB."),
                        PathKind::SaveAs{..}=>("Name of new file?","Save As creates a new file. Existing files are not overwritten."),
                    };
                    column![text(heading).size(24),text(hint),text_input("",value).id("document-path").on_input(Message::PathChanged).on_submit(Message::Submit).padding(8),
                        text(error),row![button("Enter: continue").on_press(Message::Submit),button("Esc: cancel").on_press(Message::Cancel)].spacing(12)].spacing(20).into()
                },
                Dialog::Discard(_)=>column![text("Unsaved changes").size(24),text("Keep editing to save, or discard your changes and continue."),
                    row![button("Keep editing").on_press(Message::Cancel),button("Discard changes").on_press(Message::Discard)].spacing(18)].spacing(24).into(),
                Dialog::Recovery{candidate,error}=>column![text("RECOVER UNSAVED UTF-8 DRAFT").size(24),
                    text(candidate.path.as_ref().map(|path|path.display().to_string()).unwrap_or_else(||"Untitled document".into())),
                    text("Restore the separate copy without writing the original. If the original changed or disappeared, the copy opens without a file name and needs Save As."),
                    text(error),
                    row![button("Restore copy").on_press(Message::Recover),button("Later").on_press(Message::SkipRecovery),button("Discard copy").on_press(Message::DiscardRecovery)].spacing(18)].spacing(20).into(),
                Dialog::Help=>column![text("NORTHSTAR HELP").size(24),
                    text(if self.session.is_utf8() {
                        "UTF-8: type or paste with Ctrl-V, move with arrows or Ctrl-S/D/E/X, use Backspace/Delete, Enter and Ctrl-Q R. Ctrl-Z undoes; Ctrl-Y or Ctrl-Shift-Z redoes. Ctrl-K S saves; Ctrl-K D saves and returns to the menu. Ctrl-Shift-S saves a copy."
                    } else {
                        "WordStar: Ctrl-S/Ctrl-D and Ctrl-E/Ctrl-X move, Ctrl-G deletes a character, Ctrl-Q R starts document, Ctrl-V toggles insert, Ctrl-B aligns. Ctrl-K S saves; Ctrl-Shift-S saves a copy."
                    }),
                    text("F11 toggles full screen; F12 changes font. The opening menu uses D for WordStar files, F2 for a new UTF-8 document, F3 to open a UTF-8 file, F4 for recovery, and X to exit."),
                    text(if self.session.is_utf8() {
                        "Modern UTF-8 keeps exact text bytes. The screen wraps words without inserting line breaks. Movement and deletion follow graphemes. Wide or unsupported glyphs show a square placeholder. A separate recovery copy captures unsaved edits. Overwrite remains unfinished."
                    } else {
                        "Legacy editing supports printable ASCII only. Norwegian letters, IME, clipboard text, deletion, vertical movement, blocks and undo remain unfinished. Visible original menu entries are not a promise of implemented behavior."
                    }),
                    text("The display is 80x25. Legacy comparison covers selected DOS screen states only. '?' page numbers are unknown; square cells are unrendered tokens."),
                    button("Esc: back").on_press(Message::Cancel)].spacing(22).into(),
            };
            return container(container(content).max_width(900).padding(30))
                .center(Fill)
                .into();
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
        if !self.notice.is_empty() {
            screen.cells[22].fill(northstar_desktop::screen::Cell {
                glyph: ' ',
                attribute: 0x70,
            });
            screen.put(22, 0, &self.notice, 0x70);
        } else if self.projection.hidden_tail_bytes > 0 && !self.opening {
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
    use std::{
        fs,
        sync::atomic::{AtomicUsize, Ordering},
    };

    struct Scratch(PathBuf);
    impl Scratch {
        fn new() -> Self {
            static SERIAL: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "northstar-app-recovery-{}-{}",
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
            .apply_command(&northstar_core::Command::PreviousCharacter);
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
