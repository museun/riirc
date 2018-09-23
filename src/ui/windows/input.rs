use super::*;
use std::sync::RwLock;

pub struct Input {
    parent: Rc<Window>,
    window: Rc<Window>,
    ctx: Rc<Context>,
    buffer: InputBuffer<Window>,
    history: RwLock<ui::History>,
}

impl Input {
    pub fn new(parent: Rc<Window>, ctx: Rc<Context>) -> Self {
        let (h, w) = parent.get_max_yx();
        let window = parent
            .subwin(1, w, h - 1, 0)
            .expect("create input subwindow");
        window.nodelay(true);
        window.keypad(true);
        let width = window.get_max_x() as usize;

        let window = Rc::new(window.into());
        Self {
            parent,
            window: Rc::clone(&window),
            ctx,
            history: RwLock::new(ui::History::new()),
            buffer: InputBuffer::new(width, window),
        }
    }

    pub fn read_input(&mut self) -> ui::ReadType {
        use pancurses::Input::*;
        match self.window.getch() {
            Some(Character(ch)) => self.handle_input_key(ch),
            // TODO discriminate between bad keys better
            Some(KeyBTab) => self.handle_input_key('\u{ECED}'),
            Some(ch) => self.handle_other_key(ch),
            _ => ui::ReadType::None,
        }
    }

    pub fn add_history(&mut self) {
        self.buffer.add_history()
    }

    pub fn clear_history(&mut self) {
        trace!("clearing history");
        self.buffer.clear_history();
    }

    pub fn clear_input(&mut self) {
        trace!("clearing input");
        self.buffer.clear();
        self.window.clear();
    }

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
            | KeyF11 | KeyF12 => return ui::ReadType::FKey(input),
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
            self.ctx.state.config().borrow().keybinds.get(&keybind)
        } {
            trace!("req: {:?}", req);
            if let Some(cmd) = ui::Request::parse(*req) {
                self.ctx.state.queue(cmd);
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

impl_recv!(Input);

use std::cmp::{max, min};

// TODO utf-8 this
pub struct InputBuffer<M>
where
    M: MoveableCursor,
{
    history: ui::History,
    width: usize,
    buf: Vec<char>,
    position: usize,
    window: Rc<M>,
}

impl<M> InputBuffer<M>
where
    M: MoveableCursor,
{
    pub fn new(width: usize, window: Rc<M>) -> Self {
        InputBuffer {
            history: ui::History::new(),
            width,
            buf: vec![],
            position: 0,
            window,
        }
    }

    // TODO move this out of this
    pub fn add_history(&mut self) {
        let line = self.line().iter().cloned().collect::<String>();
        self.history.append(line);
    }

    pub fn clear_history(&mut self) {
        self.history.clear()
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.position = 0;
        self.window.clear();
        self.window.move_cursor(0);
    }

    pub fn display(&self) -> &[char] {
        if self.buf.len() <= self.width {
            &self.buf
        } else {
            &self.buf[self.buf.len() - self.width..]
        }
    }

    pub fn line(&self) -> &[char] {
        &self.buf
    }

    pub fn handle_command(&mut self, cmd: &ui::Command) {
        use super::ui::{Command::*, Move::*};

        macro_rules! check {
            ($m:expr) => {
                if self.buf.is_empty() {
                    return;
                }

                // don't even try if we're at the 'wrong end'
                match $m {
                    Backward | BackwardWord | StartOfLine => if self.position == 0 {
                        return;
                    },
                    Forward | ForwardWord | EndOfLine => if self.position == self.buf.len() {
                        return;
                    },
                    _ => return,
                }
            };
        }

        match cmd {
            Delete(mv) => {
                check!(mv);

                let start = self.position;
                self.move_cursor(mv);
                let end = self.position;

                let (low, high) = (min(start, end), max(start, end));
                let range = low..high;
                for _n in range.clone() {
                    self.window.delete_at(low);
                }

                self.buf.drain(range);
                self.window.move_cursor(low);
                self.move_cursor(&ui::Move::Exact(low));
            }

            SwapCase(mv) => {
                check!(mv);

                let start = self.position;
                self.move_cursor(mv);
                let end = self.position;

                let (low, high) = (min(start, end), max(start, end));
                let range = low..high;
                for _n in range.clone() {
                    self.window.delete_at(low);
                }

                let m = if self.buf[start].is_ascii_uppercase() {
                    char::to_ascii_lowercase
                } else {
                    char::to_ascii_uppercase
                };

                let list = self.buf.drain(range).map(|c| m(&c)).collect::<Vec<_>>();
                for (n, c) in list.iter().enumerate() {
                    self.window.insert_at(start + n, *c);
                    self.buf.insert(start + n, *c);
                }
                self.window.move_cursor(start);
                self.move_cursor(&ui::Move::Exact(start));
            }

            Insert(index, ch) => {
                if *index < self.buf.len() {
                    self.buf.insert(*index, *ch);
                } else {
                    self.buf.push(*ch);
                }

                self.position = *index;
                self.window.insert_at(*index, *ch);
            }

            Append(ch) => {
                self.handle_command(&ui::Command::Insert(self.position, *ch));
                self.position += 1;
            }

            Move(mv) => self.move_cursor(mv),

            Recall(mv) => {
                let history = match match mv {
                    Forward => self.history.forward(),
                    Backward => self.history.backward(),
                    _ => unreachable!(),
                } {
                    Some(history) => history.clone(),
                    None => return,
                };

                self.clear();
                self.buf = history.chars().collect();
                self.position = self.buf.len();
                for (n, ch) in self.buf.iter().enumerate() {
                    self.window.insert_at(n, *ch);
                }
            }
        }
    }

    fn move_cursor(&mut self, mv: &ui::Move) {
        use super::ui::Move::*;

        let end = self.buf.len();
        match mv {
            EndOfLine => self.position = end,
            StartOfLine => self.position = 0,

            Forward => {
                if self.position != end {
                    self.position += 1;
                }
            }

            ForwardWord => {
                if self.position == end {
                    return;
                }

                // skip punc/space if we're on it
                if let Some(c) = self.buf.get(self.position) {
                    if c.is_ascii_whitespace() || c.is_ascii_punctuation() {
                        self.position += 1
                    }
                }

                let mut iter = self.buf[self.position..].iter().peekable();
                while let Some(c) = iter.next() {
                    if c.is_ascii_punctuation() {
                        break;
                    }

                    self.position += 1;
                    if c.is_ascii_whitespace() {
                        if let Some(c) = iter.peek() {
                            if c.is_ascii_whitespace() {
                                continue;
                            }
                        }
                        break;
                    }
                }
            }

            Backward => {
                if self.position != 0 {
                    self.position -= 1;
                }
            }

            BackwardWord => {
                if self.position == 0 {
                    return;
                }

                self.position -= 1;
                let mut found = false;
                for c in self.buf[..self.position].iter().rev() {
                    if c.is_ascii_whitespace() && found {
                        break;
                    }

                    if c.is_alphanumeric() {
                        found = true;
                    }
                    self.position -= 1;
                }
            }

            Exact(sz) => self.position = min(*sz, end),
        }

        self.window.move_cursor(self.position);
    }
}
