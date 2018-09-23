use super::*;
use std::sync::Arc;

pub struct InputWindow {
    parent: CWindow,
    window: CWindow,
    buffer: ui::InputBuffer<CWindow>,
    state: Arc<ui::State>,
}

impl InputWindow {
    pub fn new(parent: CWindow, state: Arc<ui::State>) -> Self {
        let (h, w) = parent.get_max_yx();
        let window = parent
            .subwin(1, w, h - 1, 0)
            .expect("create input subwindow");
        window.nodelay(true);
        window.keypad(true);

        let max = window.get_max_x() as usize;
        let window: CWindow = window.into();

        let buffer = ui::InputBuffer::new(max, Arc::new(window.clone()));

        Self {
            parent,
            window,
            buffer,
            state,
        }
    }
}

impl window::Window for InputWindow {
    fn window(&self) -> CWindow {
        self.window
    }
}

impl ui::Outputter for InputWindow {
    fn output(&self, output: ui::Output, eol: bool) {
        let window = self.window();
        for (r, cp) in output.colors.iter() {
            insert_into(&window, *cp, &output.data[r.start..r.end], false)
        }
        if eol {
            self.window().addch('\n');
        }
    }

    fn clear(&self) {
        self.window().clear();
    }
}

impl ui::InputWindow for InputWindow {
    fn add_history(&self) {
        self.buffer.add_history();
    }

    fn clear_history(&self) {
        self.buffer.clear_history();
    }

    fn read_input(&self) -> ui::ReadType {
        use pancurses::Input::*;
        match self.window.getch() {
            Some(Character(ch)) => self.handle_input_key(ch),
            // TODO discriminate between bad keys better
            Some(KeyBTab) => self.handle_input_key('\u{ECED}'),
            Some(ch) => self.handle_other_key(ch),
            _ => ui::ReadType::None,
        }
    }

    fn clear_input(&self) {
        trace!("clearing input");
        self.buffer.clear();
        self.window.clear();
    }
}

struct FKey(ui::FKey);

impl From<pancurses::Input> for FKey {
    fn from(input: pancurses::Input) -> Self {
        use pancurses::Input::*;

        FKey(match input {
            KeyF1 => ui::FKey::F1,
            KeyF2 => ui::FKey::F2,
            KeyF3 => ui::FKey::F3,
            KeyF4 => ui::FKey::F4,
            KeyF5 => ui::FKey::F5,
            KeyF6 => ui::FKey::F6,
            KeyF7 => ui::FKey::F7,
            KeyF8 => ui::FKey::F8,
            KeyF9 => ui::FKey::F9,
            KeyF10 => ui::FKey::F10,
            KeyF11 => ui::FKey::F11,
            KeyF12 => ui::FKey::F12,
            _ => unreachable!(),
        })
    }
}

impl InputWindow {
    fn handle_other_key(&mut self, input: pancurses::Input) -> ui::ReadType {
        use pancurses::Input::*;

        let cmd = match input {
            KeyHome => &ui::Command::Move(ui::Move::StartOfLine),
            KeyEnd => &ui::Command::Move(ui::Move::EndOfLine),

            KeyUp => &ui::Command::Recall(ui::Move::Backward),
            KeyDown => &ui::Command::Recall(ui::Move::Forward),

            KeyLeft => &ui::Command::Move(ui::Move::Backward),
            KeySMessage => &ui::Command::Move(ui::Move::BackwardWord),

            KeyRight => &ui::Command::Move(ui::Move::Forward),
            KeySResume => &ui::Command::Move(ui::Move::ForwardWord),

            KeyDC => &ui::Command::Delete(ui::Move::Forward),

            KeyF1 | KeyF2 | KeyF3 | KeyF4 | KeyF5 | KeyF6 | KeyF7 | KeyF8 | KeyF9 | KeyF10
            | KeyF11 | KeyF12 => return ui::ReadType::FKey(FKey::from(input).0),
            key => {
                debug!("unknown input: {:?}", key);
                return ui::ReadType::None;
            }
        };

        self.buffer.handle_command(cmd);
        ui::ReadType::None
    }

    fn handle_modified_key(&mut self, key: &ui::Key) -> ui::ReadType {
        use super::ui::{KeyKind::*, Mod::*};

        match (&key.modifier, &key.kind) {
            (None, Backspace) => {
                self.buffer
                    .handle_command(&ui::Command::Delete(ui::Move::Backward));
                self.window.refresh();
                return ui::ReadType::None;
            }
            (None, Enter) => {
                let buf = self.buffer.line().into_iter().collect();
                return ui::ReadType::Line(buf);
            }
            _ => {}
        }

        if let Some(req) = {
            let keybind = ui::KeyType::from(*key);
            self.state.config().read().unwrap().keybinds.get(&keybind)
        } {
            trace!("req: {:?}", req);
            if let Some(cmd) = ui::Request::parse(*req) {
                self.state.queue(cmd);
            }
            if let Some(cmd) = ui::Command::parse(*req) {
                self.buffer.handle_command(&cmd);
            }
        }
        ui::ReadType::None
    }

    fn handle_input_key(&mut self, ch: char) -> ui::ReadType {
        if let Some(key) = ui::Key::parse(ch as u16) {
            match (&key.modifier, &key.kind) {
                (ui::Mod::None, ui::KeyKind::Other(_))
                | (ui::Mod::None, ui::KeyKind::Char(_))
                | (ui::Mod::Shift, ui::KeyKind::Char(_)) => {}
                _ => return self.handle_modified_key(&key),
            };
        }

        // TODO don't do this here
        // why not?
        let window = self.buffer.display();
        for (i, ch) in window.iter().enumerate() {
            self.window.mvaddch(0, i as i32, *ch);
        }

        self.buffer.handle_command(&ui::Command::Append(ch));
        ui::ReadType::None
    }
}
