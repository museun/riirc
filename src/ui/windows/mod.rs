use super::ui;
use std::ops::Deref;
use std::rc::Rc;

pub struct Window(pancurses::Window);

impl Deref for Window {
    type Target = pancurses::Window;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<pancurses::Window> for Window {
    fn from(window: pancurses::Window) -> Self {
        Window(window)
    }
}

pub trait MoveableCursor {
    fn move_cursor(&self, pos: usize);
    fn clear(&self);
    fn delete_at(&self, pos: usize);
    fn insert_at(&self, pos: usize, ch: char);
}

impl MoveableCursor for Window {
    fn move_cursor(&self, pos: usize) {
        self.mv(self.get_cur_y(), pos as i32);
    }
    fn clear(&self) {
        self.0.clear();
    }
    fn delete_at(&self, pos: usize) {
        self.mv(self.get_cur_y(), pos as i32);
        self.delch();
    }
    fn insert_at(&self, pos: usize, ch: char) {
        self.mv(self.get_cur_y(), pos as i32);
        self.insch(ch);
        self.mv(self.get_cur_y(), pos as i32 + 1);
    }
}

/*
pub trait Outputter {
    fn output(&self, output: Output, eol: bool);
    fn clear(&self);
}*/

macro_rules! impl_recv {
    ($t:ty) => {
        impl ui::MessageReceiver<Request> for $t {
            fn queue(&self, data: impl Into<Request>) {
                self.ctx.queue.enqueue(data.into())
            }
        }
        
        impl ui::Outputter for $t {
            fn output(&self, output: ui::Output, eol: bool) {
                fn insert_into(
                    window: &Window,
                    color: impl Into<ui::ColorPair>,
                    s: impl AsRef<str>,
                    eol: bool,
                ) {
                    let (y, x) = window.get_cur_yx();
        
                    let color = color.into();
                    let s = s.as_ref();
                    window.addstr(s);
        
                    let (ny, nx) = window.get_cur_yx();
                    window.mvchgat(
                        y,
                        x,
                        s.len() as i32,
                        if color.bold {
                            pancurses::A_BOLD
                        } else {
                            pancurses::A_NORMAL
                        },
                        color.fg.into(),
                    );
                    window.mv(ny, nx);
                    if eol {
                        window.addch('\n');
                    }
                    window.refresh();
                }
        
                let window = &self.window;
                for (r, cp) in output.colors.iter() {
                    insert_into(&window, *cp, &output.data[r.start..r.end], false)
                }
                if eol {
                    self.window.addch('\n');
                }
            }
        
            fn clear(&self) {
                self.window.clear();
                self.window.refresh();
            }
        }
    };
}

mod container;
mod input;
mod nicklist;
mod output;
use self::{input::*, nicklist::*, output::*};

pub use self::container::Container;

pub struct Context<T> {
    pub(crate) state: Rc<ui::State>,
    pub(crate) queue: Rc<ui::MessageQueue<T>>,
}

impl Context<Request> {
    pub fn read_queue(&self) -> Vec<Request> {
        self.queue.read_all()
    }
}

pub enum Request {
    Write(Option<Target>, ui::Output),
    Writeln(Option<Target>, ui::Output),
    Clear(Option<Target>),
}

impl From<ui::Output> for Request {
    fn from(output: ui::Output) -> Self {
        Request::Writeln(None, output)
    }
}

impl From<(Target, ui::Output)> for Request {
    fn from(output: (Target, ui::Output)) -> Self {
        Request::Writeln(Some(output.0), output.1)
    }
}

pub enum Target {
    Container,
    Output,
    Input,
    Nicklist,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ReadType {
    Line(String),
    FKey(pancurses::Input),
    None,
}

