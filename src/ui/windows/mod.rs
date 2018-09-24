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

macro_rules! impl_recv {
    ($t:ty) => {
        impl ui::MessageReceiver<ui::Request> for $t {
            fn queue(&self, data: impl Into<ui::Request>) {
                self.ctx.queue.enqueue(data.into())
            }
        }
        
        impl ui::Outputter for $t {
            fn output(&self, output: ui::Output, eol: bool) {
                fn insert_into(
                    window: &Window,
                    color: impl Into<ui::ColorPair>,
                    s: impl AsRef<str>,
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
                }        

                let window = &self.window;
                for (r, cp) in output.colors.iter() {
                    insert_into(&window, *cp, &output.data[r.start..r.end])
                }
                if eol {
                    self.window.addch('\n');
                }
                window.refresh();
            }
        
            fn clear(&self) {
                self.window.clear();
                self.window.refresh();
            }
        }
    };
}

import!(
    container,
    input,
    nicklist,
    output
);

pub use self::container::Container;

pub struct Context {
    pub(crate) state: Rc<ui::State>,
    pub(crate) queue: Rc<ui::MessageQueue<ui::Request>>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum ReadType {
    Line(String),
    FKey(pancurses::Input),
    None,
}

