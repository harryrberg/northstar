// SPDX-License-Identifier: GPL-3.0-only
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use iced::{
    Element, Event, Fill, Font, Subscription, Task, Theme, keyboard,
    widget::{button, canvas, column, container, row, text, text_input},
    window,
};
use northstar_desktop::{
    input::{Action, Keyboard},
    projection::Projection,
    screen::Screen,
    session::Session,
};
use std::path::PathBuf;
mod surface;

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
    session: Session,
    projection: Projection,
    keyboard: Keyboard,
    dialog: Option<Dialog>,
    notice: String,
    window_focused: bool,
    opening: bool,
    fullscreen: bool,
    classic_font: bool,
}
#[derive(Debug, Clone, Copy)]
enum Next {
    Opening,
    Close(window::Id),
}
#[derive(Debug, Clone, Copy)]
enum PathKind {
    Open,
    SaveAs { done: bool },
}
enum Dialog {
    Path {
        kind: PathKind,
        value: String,
        error: String,
    },
    Discard(Next),
    Help,
}
#[derive(Debug, Clone)]
pub(crate) enum Message {
    Event(Event, window::Id),
    Open,
    SaveAs(bool),
    Help,
    PathChanged(String),
    Submit,
    Cancel,
    Discard,
}
impl App {
    fn new() -> Self {
        let mut session = Session::default();
        let mut notice = String::new();
        let mut opening = true;
        let mut args = std::env::args_os().skip(1);
        if let Some(path) = args.next() {
            if args.next().is_some() {
                notice = "Usage: northstar-desktop [FILE]".into();
            } else {
                match Session::open(&PathBuf::from(path)) {
                    Ok(opened) => {
                        session = opened;
                        opening = false;
                    }
                    Err(e) => notice = format!("Open failed: {e}"),
                }
            }
        }
        let projection = Projection::new(&session.editor);
        Self {
            session,
            projection,
            keyboard: Keyboard::default(),
            dialog: None,
            notice,
            window_focused: false,
            opening,
            fullscreen: false,
            classic_font: false,
        }
    }
    fn title(&self) -> String {
        let path = self
            .session
            .path
            .as_ref()
            .map(|p| p.to_string_lossy())
            .unwrap_or("Untitled".into());
        format!(
            "NorthStar: {path}{}",
            if self.session.editor.snapshot().modified {
                " *"
            } else {
                ""
            }
        )
    }
    fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, status, id| {
            if matches!(event, Event::Window(_)) || status == iced::event::Status::Ignored {
                Some(Message::Event(event, id))
            } else {
                None
            }
        })
    }
    fn begin(&mut self, next: Next) -> Task<Message> {
        self.keyboard.reset();
        if self.session.editor.snapshot().modified {
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
                self.session = Session::default();
                self.projection = Projection::new(&self.session.editor);
                self.dialog = None;
                self.opening = true;
                self.notice.clear();
                Task::none()
            }
        }
    }
    fn path_dialog(&mut self, kind: PathKind) -> Task<Message> {
        self.keyboard.reset();
        let value = match kind {
            PathKind::Open => String::new(),
            PathKind::SaveAs { .. } => self
                .session
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
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
            Message::Open => return self.path_dialog(PathKind::Open),
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
                if let Some(Dialog::Path { kind, value, error }) = &mut self.dialog {
                    if value.is_empty() {
                        *error = "Enter a file path".into();
                        return Task::none();
                    }
                    let path = PathBuf::from(&*value);
                    let result = match kind {
                        PathKind::Open => match Session::open(&path) {
                            Ok(opened) => {
                                self.session = opened;
                                Ok(())
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                self.session = Session::new_at(path);
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        PathKind::SaveAs { .. } => self.session.save_as(&path),
                    };
                    match result {
                        Ok(()) => {
                            let done = matches!(kind, PathKind::SaveAs { done: true });
                            self.opening = false;
                            self.dialog = None;
                            self.notice.clear();
                            self.projection = Projection::new(&self.session.editor);
                            if done {
                                return self.proceed(Next::Opening);
                            }
                        }
                        Err(e) => {
                            *error = if e.kind() == std::io::ErrorKind::AlreadyExists {
                                "File exists. This prototype saves to NEW files only; choose another path.".into()
                            } else {
                                format!(
                                    "{e}. Document retained; a failed write may leave an incomplete new file."
                                )
                            }
                        }
                    }
                }
            }
            Message::Cancel => {
                self.dialog = None;
                self.keyboard.reset();
                self.notice.clear();
            }
            Message::Discard => {
                if let Some(Dialog::Discard(next)) = self.dialog.take() {
                    return self.proceed(next);
                }
            }
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
                                _=>self.notice="This opening-menu command is not implemented. D opens a document.".into(),
                            }
                        }
                    } else {
                        match self.keyboard.handle(&event) {
                            Some(Action::Edit(command)) => {
                                match self.session.editor.apply(&command) {
                                    Ok(()) => {
                                        self.projection = Projection::new(&self.session.editor);
                                        self.notice.clear();
                                    }
                                    Err(e) => self.notice = e.to_string(),
                                }
                            }
                            Some(Action::SaveAs) => return self.update(Message::SaveAs(false)),
                            Some(Action::SaveAndDone) => return self.update(Message::SaveAs(true)),
                            Some(Action::Done) => return self.begin(Next::Opening),
                            Some(Action::Help) => return self.update(Message::Help),
                            Some(Action::Notice(message)) => self.notice = message.into(),
                            None => {}
                        }
                    }
                }
            }
            Message::Event(Event::InputMethod(_), _) if self.dialog.is_none() => {
                self.notice = "IME is not supported in the legacy ASCII prototype".into()
            }
            _ => {}
        }
        Task::none()
    }
    fn view(&self) -> Element<'_, Message> {
        if let Some(dialog) = &self.dialog {
            let content:Element<'_,Message>=match dialog {
                Dialog::Path{kind,value,error}=>{
                    let (heading,hint)=match kind{
                        PathKind::Open=>("Name of file to edit?","Full path; a missing file starts a new document. Read limit: 1 MiB."),
                        PathKind::SaveAs{..}=>("Name of file to save?","Prototype: save to a new path. Existing files are never overwritten."),
                    };
                    column![text(heading).size(24),text(hint),text_input("",value).id("document-path").on_input(Message::PathChanged).on_submit(Message::Submit).padding(8),
                        text(error),row![button("Enter: continue").on_press(Message::Submit),button("Esc: cancel").on_press(Message::Cancel)].spacing(12)].spacing(20).into()
                },
                Dialog::Discard(_)=>column![text("Unsaved changes").size(24),text("Keep editing to save, or discard your changes and continue."),
                    row![button("Keep editing").on_press(Message::Cancel),button("Discard changes").on_press(Message::Discard)].spacing(18)].spacing(24).into(),
                Dialog::Help=>column![text("NORTHSTAR HELP").size(24),
                    text("^S / ^D   Left / right       ^Q R   Start of document\n^V        Insert on/off      ^B     Align paragraph\nEnter     Break / next line  ^K S   Save to a new file\n^K D      Save and done      ^K Q   Done, with unsaved-change check"),
                    text("F1 help, F7 align, F9 save, F10 save and done. F11 toggles full screen; F12 switches between IBM Plex Mono and the classic VGA font. Left/Right and Insert also work. The opening menu uses D to open/create and X to exit."),
                    text("Printable ASCII only. Norwegian letters, IME, clipboard text, deletion, vertical movement, blocks and undo remain unfinished. Visible original menu entries are not a promise of implemented behavior."),
                    text("Margins 1/65, right justification off. The display follows retained 80x25 DOS observations. Pagination, scrolling and unsupported control glyphs are not yet qualified. '?' page numbers are unknown; square cells are unrendered tokens."),
                    button("Esc: back").on_press(Message::Cancel)].spacing(22).into(),
            };
            return container(container(content).max_width(900).padding(30))
                .center(Fill)
                .into();
        }
        let mut screen = if self.opening {
            Screen::opening()
        } else {
            let label = self
                .session
                .path
                .as_ref()
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
            Screen::editor(
                &self.projection,
                &self.session.editor.snapshot(),
                &label,
                self.keyboard.prefix(),
            )
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
