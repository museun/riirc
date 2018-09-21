use pancurses;
use std::sync::Arc;

use super::inputbuffer::{Command, Move, MoveableCursor};
use super::*;

pub struct InputWindow {
    window: Arc<InnerWindow<InputWindow>>,
    buffer: InputBuffer<InnerWindow<InputWindow>>,
}

impl WindowBuilder<Self> for InputWindow {
    fn create(parent: CWindow) -> CWindow {
        let (h, w) = parent.get_max_yx();

        let window = parent
            .subwin(1, w, h - 1, 0)
            .expect("create input subwindow");
        window.nodelay(true);
        window.keypad(true);

        Arc::new(window)
    }

    fn new(window: InnerWindow<Self>) -> Self {
        let window = Arc::new(window);

        let max = window.window().get_max_x() as usize;
        let buffer = InputBuffer::new(max, Arc::clone(&window));

        Self { window, buffer }
    }
}

impl Window<Self> for InputWindow {
    fn window(&self) -> InnerWindow<Self> {
        self.window
    }
}

impl InputWindow {
    pub fn read_input(&mut self) -> ReadType {
        use pancurses::Input::*;
        match self.window.getch() {
            Some(Character(ch)) => self.handle_input_key(ch),
            // TODO discriminate between bad keys better
            Some(KeyBTab) => self.handle_input_key('\u{ECED}'),
            Some(ch) => self.handle_other_key(ch),
            _ => ReadType::None,
        }
    }

    fn handle_other_key(&mut self, input: pancurses::Input) -> ReadType {
        use pancurses::Input::*;

        let cmd = match input {
            KeyHome => &Command::Move(Move::StartOfLine),
            KeyEnd => &Command::Move(Move::EndOfLine),

            KeyUp => &Command::Recall(Move::Backward),
            KeyDown => &Command::Recall(Move::Forward),

            KeyLeft => &Command::Move(Move::Backward),
            KeySMessage => &Command::Move(Move::BackwardWord),

            KeyRight => &Command::Move(Move::Forward),
            KeySResume => &Command::Move(Move::ForwardWord),

            KeyDC => &Command::Delete(Move::Forward),

            KeyF1 | KeyF2 | KeyF3 | KeyF4 | KeyF5 | KeyF6 | KeyF7 | KeyF8 | KeyF9 | KeyF10
            | KeyF11 | KeyF12 => return ReadType::FKey(input),
            key => {
                debug!("unknown input: {:?}", key);
                return ReadType::None;
            }
        };

        self.buffer.handle_command(cmd);
        ReadType::None
    }

    fn handle_modified_key(&mut self, key: &Key) -> ReadType {
        use self::{KeyKind::*, Mod::*};
        use std::convert::TryFrom;

        match (&key.modifier, &key.kind) {
            (None, Backspace) => {
                self.buffer.handle_command(&Command::Delete(Move::Backward));
                self.window.refresh();
                return ReadType::None;
            }
            (None, Enter) => {
                let buf = self.buffer.line().into_iter().collect();
                return ReadType::Line(buf);
            }
            _ => {}
        }

        if let Some(req) = {
            let keybind = KeyType::from(*key);
            self.window
                .state()
                .config()
                .read()
                .unwrap()
                .keybinds
                .get(&keybind)
        } {
            trace!("req: {:?}", req);
            if let Ok(cmd) = Request::try_from(*req) {
                self.window.state().queue(cmd);
            }
            if let Ok(cmd) = inputbuffer::Command::try_from(*req) {
                self.buffer.handle_command(&cmd);
            }
        }
        ReadType::None
    }

    fn handle_input_key(&mut self, ch: char) -> ReadType {
        if let Some(key) = Key::parse(ch as u16) {
            match (&key.modifier, &key.kind) {
                (Mod::None, KeyKind::Other(_))
                | (Mod::None, KeyKind::Char(_))
                | (Mod::Shift, KeyKind::Char(_)) => {}
                _ => return self.handle_modified_key(&key),
            };
        }

        // TODO don't do this here
        // why not?
        let window = self.buffer.display();
        for (i, ch) in window.iter().enumerate() {
            self.window.mvaddch(0, i as i32, *ch);
        }

        self.buffer.handle_command(&Command::Append(ch));
        ReadType::None
    }

    pub fn add_history(&mut self) {
        self.buffer.add_history();
    }

    pub fn clear_history(&mut self) {
        self.buffer.clear_history();
    }

    pub fn clear_input(&mut self) {
        trace!("clearing input");
        self.buffer.clear();
        self.window.clear();
    }
}

#[derive(Debug, PartialEq)]
pub enum ReadType {
    Line(String),
    FKey(pancurses::Input),
    None,
}
