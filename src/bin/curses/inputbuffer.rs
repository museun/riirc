use std::cmp::{max, min};
use std::sync::Arc;

use super::boundedset::BoundedSet;

pub trait MoveableCursor {
    fn move_cursor(&self, pos: usize);
    fn clear(&self);
    fn delete_at(&self, pos: usize);
    fn insert_at(&self, pos: usize, ch: char) {}
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Delete(Move),
    SwapCase(Move),
    Insert(usize, char),
    Append(char),
    Move(Move),

    // these aren't really movement
    Recall(Move),
}

#[derive(Debug, PartialEq)]
pub enum Move {
    EndOfLine,
    StartOfLine,
    ForwardWord,
    Forward,
    BackwardWord,
    Backward,
    Exact(usize),
}

#[derive(Debug)]
struct History {
    queue: BoundedSet<String>,
    pos: i32,
}

impl History {
    pub fn new() -> Self {
        Self {
            queue: BoundedSet::new(32),
            pos: -1,
        }
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.pos = -1;
    }

    pub fn append(&mut self, data: impl Into<String>) {
        self.queue.insert(data.into());
        self.pos = -1;
    }

    pub fn prev(&mut self) -> Option<&String> {
        if self.queue.len() == 0 || self.pos as usize == self.queue.len() {
            return None;
        }

        self.pos += 1;
        self.queue.iter().rev().nth(self.pos as usize)
    }

    pub fn next(&mut self) -> Option<&String> {
        if self.queue.len() == 0 {
            return None;
        }

        if self.pos > 0 {
            self.pos -= 1;
        }

        self.queue.iter().rev().nth(self.pos as usize)
    }
}

// TODO utf-8 this

pub struct InputBuffer<M>
where
    M: MoveableCursor,
{
    history: History,

    width: usize,
    buf: Vec<char>,
    position: usize,
    window: Arc<M>,
}

impl<M> ::std::fmt::Debug for InputBuffer<M>
where
    M: MoveableCursor,
{
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "{}:{} {:?}", self.position, self.buf.len(), self.buf)
    }
}

impl<M> InputBuffer<M>
where
    M: MoveableCursor,
{
    pub fn new(width: usize, window: Arc<M>) -> Self {
        InputBuffer {
            history: History::new(),
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

    pub fn handle_command(&mut self, cmd: &Command) {
        use self::{Command::*, Move::*};

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
                for n in range.clone() {
                    self.window.delete_at(low);
                }

                self.buf.drain(range);
                self.window.move_cursor(low);
                self.move_cursor(&self::Move::Exact(low));
            }

            SwapCase(mv) => {
                check!(mv);

                let start = self.position;
                self.move_cursor(mv);
                let end = self.position;

                let (low, high) = (min(start, end), max(start, end));
                let range = low..high;
                for n in range.clone() {
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
                self.move_cursor(&self::Move::Exact(start));
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
                self.handle_command(&Command::Insert(self.position, *ch));
                self.position += 1;
            }

            Move(mv) => self.move_cursor(mv),

            Recall(mv) => {
                let history = match match mv {
                    Forward => self.history.next(),
                    Backward => self.history.prev(),
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

    fn move_cursor(&mut self, mv: &Move) {
        use self::Move::*;

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
