// SPDX-License-Identifier: GPL-3.0-only
use iced::keyboard::{Event, Key, key::Named};
use northstar_core::Command;

#[derive(Debug, PartialEq, Eq)]
pub enum Action {
    Edit(Command),
    Text(String),
    Backspace,
    Delete,
    MoveVertical(isize),
    Paste,
    Save,
    SaveAs,
    SaveAndDone,
    Done,
    Help,
    Notice(&'static str),
}

#[derive(Default)]
pub struct Keyboard {
    prefix: Option<char>,
}

impl Keyboard {
    pub fn reset(&mut self) {
        self.prefix = None;
    }
    pub fn pending(&self) -> bool {
        self.prefix.is_some()
    }
    pub fn prefix(&self) -> Option<char> {
        self.prefix
    }

    pub fn handle(&mut self, event: &Event) -> Option<Action> {
        self.handle_mode(event, false)
    }

    pub fn handle_mode(&mut self, event: &Event, utf8: bool) -> Option<Action> {
        let Event::KeyPressed {
            key,
            modifiers,
            text,
            repeat,
            ..
        } = event
        else {
            return None;
        };
        let letter = match key.as_ref() {
            Key::Character(s) => s.to_ascii_lowercase(),
            _ => String::new(),
        };
        // Modifier presses must not consume a pending Ctrl-Q sequence.
        if matches!(
            key,
            Key::Named(Named::Control | Named::Shift | Named::Alt | Named::Super | Named::AltGraph)
        ) {
            return None;
        }
        if *key == Key::Named(Named::Escape) {
            self.reset();
            return Some(Action::Notice("Command cancelled"));
        }
        if let Some(prefix) = self.prefix {
            if *repeat {
                return None;
            }
            self.reset();
            if modifiers.alt() || modifiers.logo() {
                return Some(Action::Notice("Unsupported command"));
            }
            return Some(match (prefix, letter.as_str()) {
                ('Q', "r") => Action::Edit(Command::DocumentStart),
                ('K', "s") => Action::Save,
                ('K', "d") => Action::SaveAndDone,
                ('K', "q") => Action::Done,
                _ => Action::Notice("This command is not implemented"),
            });
        }
        if modifiers.control() && !modifiers.alt() && !modifiers.logo() {
            return match letter.as_str() {
                "s" if modifiers.shift() && !repeat => Some(Action::SaveAs),
                "s" if modifiers.shift() => None,
                "q" | "k" => {
                    if !repeat {
                        self.prefix = Some(if letter == "q" { 'Q' } else { 'K' });
                    }
                    None
                }
                "s" => Some(Action::Edit(Command::PreviousCharacter)),
                "d" => Some(Action::Edit(Command::NextCharacter)),
                "e" if utf8 => Some(Action::MoveVertical(-1)),
                "x" if utf8 => Some(Action::MoveVertical(1)),
                "v" if !repeat => Some(if utf8 {
                    Action::Paste
                } else {
                    Action::Edit(Command::ToggleInsert)
                }),
                "b" if !repeat => Some(Action::Edit(Command::Reformat)),
                "j" if !repeat => Some(Action::Help),
                "v" | "b" | "j" => None,
                _ => Some(Action::Notice("This control command is not implemented")),
            };
        }
        // Modern text uses the produced text, including AltGr combinations.
        // Legacy input retains its strict ASCII boundary.
        if modifiers.logo() || (modifiers.alt() && !utf8) {
            return Some(Action::Notice(
                "Alt/AltGr input is not supported in this prototype",
            ));
        }
        match key {
            Key::Named(Named::ArrowLeft) => Some(Action::Edit(Command::PreviousCharacter)),
            Key::Named(Named::ArrowRight) => Some(Action::Edit(Command::NextCharacter)),
            Key::Named(Named::ArrowUp) if utf8 => Some(Action::MoveVertical(-1)),
            Key::Named(Named::ArrowDown) if utf8 => Some(Action::MoveVertical(1)),
            Key::Named(Named::Backspace) => Some(Action::Backspace),
            Key::Named(Named::Delete) => Some(Action::Delete),
            Key::Named(Named::Enter) => Some(Action::Edit(Command::Enter)),
            Key::Named(Named::Insert) if !repeat && !utf8 => {
                Some(Action::Edit(Command::ToggleInsert))
            }
            Key::Named(Named::F1) if !repeat => Some(Action::Help),
            Key::Named(Named::F7) if !repeat => Some(Action::Edit(Command::Reformat)),
            Key::Named(Named::F9) if !repeat => Some(Action::Save),
            Key::Named(Named::F10) if !repeat => Some(Action::SaveAndDone),
            Key::Named(Named::Insert | Named::F1 | Named::F7 | Named::F9 | Named::F10) => None,
            _ => match text {
                Some(text) if !text.is_empty() => Some(if utf8 {
                    Action::Text(text.to_string())
                } else {
                    Action::Edit(Command::InsertAscii {
                        text: text.to_string(),
                    })
                }),
                _ => Some(Action::Notice(
                    "This key is not implemented; press F1 for available commands",
                )),
            },
        }
    }
}
