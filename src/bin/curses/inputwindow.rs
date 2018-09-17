use pancurses;
use std::sync::Arc;

use super::*;

pub struct InputWindow {
    window: pancurses::Window,
    buffer: InputBuffer,
    queue: Arc<MessageQueue>,
}

impl InputWindow {
    pub fn new(window: pancurses::Window, queue: Arc<MessageQueue>) -> Self {
        let max = window.get_max_x() as usize;
        window.nodelay(true);
        window.keypad(true);

        Self {
            window,
            buffer: InputBuffer::new(max),
            queue,
        }
    }

    pub fn read_input(&mut self) -> Result<ReadType, Error> {
        use pancurses::Input::*;

        while let Some(ch) = self.window.getch() {
            match ch {
                Character(ch) if ch.is_control() => {
                    if ch as u8 == 0x0A {
                        let buf = self.buffer.get_line().into_iter().collect();
                        return Ok(ReadType::Line(buf));
                    } else {
                        self.handle_control_key(ch)
                    }
                }
                Character(ch) => self.handle_input_key(ch),

                KeyLeft => self.handle_key_left(false),
                KeySLeft => self.handle_key_left(true),

                KeyRight => self.handle_key_right(false),
                KeySRight => self.handle_key_right(true),

                KeyDC => self.handle_delete_key(false),

                KeyF1 | KeyF2 | KeyF3 | KeyF4 | KeyF5 | KeyF6 | KeyF7 | KeyF8 | KeyF9 | KeyF10
                | KeyF11 | KeyF12 => return Ok(ReadType::FKey(ch.into())),

                _ => return Err(Error::UnknownInput(ch)),
            };
        }

        Ok(ReadType::None)
    }

    fn handle_control_key(&mut self, ch: char) {
        trace!("got control: 0x{:02X}", ch as u8);
        match ch as u8 {
            // backspace
            0x08 => self.handle_delete_key(true),

            // C-l
            0x0C => self.queue.push(Request::Clear(true)),

            // C-0 to C-9
            0x37...0x40 => self
                .queue
                .push(Request::SwitchBuffer((ch as u8 - 0x37) as usize)),

            // C-n
            0x0E => self.queue.push(Request::NextBuffer),

            // C-p
            0x10 => self.queue.push(Request::PrevBuffer),

            // C-w
            0x17 => {}

            _ => {}
        };
    }

    // TODO probably check to see if this is in a printable range
    fn handle_input_key(&mut self, ch: char) {
        if let 0x37...0x40 = ch as u8 {
            self.handle_control_key(ch);
            return;
        }

        let window = self.buffer.display();
        for (i, ch) in window.iter().enumerate() {
            self.window.mvaddch(0, i as i32, *ch);
        }

        self.window.addch(ch);
        self.buffer.push(ch);
    }

    fn handle_delete_key(&mut self, backspace: bool) {
        let pos = self.window.get_cur_x();
        let pos = if backspace {
            self.window.mv(self.window.get_cur_y(), pos - 1);
            pos - 1
        } else {
            pos
        };
        self.buffer.delete(pos as usize, false);
        self.window.delch();
    }

    fn handle_key_left(&mut self, _shift: bool) {
        let pos = self.window.get_cur_x();
        self.window.mv(0, pos - 1);
    }

    fn handle_key_right(&mut self, _shift: bool) {
        let pos = self.window.get_cur_x();
        if (pos as usize) < self.buffer.len() {
            self.window.mv(0, pos + 1);
        }
    }

    pub fn clear_input(&mut self) {
        self.buffer.clear();
        self.window.clear();
    }
}

#[derive(Debug, PartialEq)]
pub enum Error {
    UnknownInput(pancurses::Input),
}

#[derive(Debug, PartialEq)]
pub enum ReadType {
    Line(String),
    FKey(FKey),
    None,
}

impl From<pancurses::Input> for FKey {
    fn from(input: pancurses::Input) -> FKey {
        use self::FKey::*;
        use pancurses::Input::*;

        match input {
            KeyF1 => F1,
            KeyF2 => F2,
            KeyF3 => F3,
            KeyF4 => F4,
            KeyF5 => F5,
            KeyF6 => F6,
            KeyF7 => F7,
            KeyF8 => F8,
            KeyF9 => F9,
            KeyF10 => F10,
            KeyF11 => F11,
            KeyF12 => F12,
            _ => panic!("not an function key"),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum FKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}
