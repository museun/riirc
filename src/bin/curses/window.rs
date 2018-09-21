use super::*;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

pub enum Request {
    Resize(i32, i32),
}

pub trait WindowBuilder<T: WindowBuilder<T> + Window<T>> {
    fn create(parent: CWindow) -> CWindow;
    fn new(window: InnerWindow<T>) -> T;
}

pub trait Window<T>
where
    T: WindowBuilder<T> + Window<T>,
{
    fn window(&self) -> InnerWindow<T>;
}

pub struct InnerWindow<T>
where
    T: WindowBuilder<T> + Window<T>,
{
    queue: MessageQueue<Request>,
    parent: CWindow,
    window: CWindow,
    state: Arc<State>,

    _t: PhantomData<T>,
}

impl<T> InnerWindow<T>
where
    T: WindowBuilder<T> + Window<T>,
{
    pub fn new(factory: fn(CWindow) -> CWindow, parent: CWindow, state: Arc<State>) -> T {
        T::new(InnerWindow {
            queue: MessageQueue::new(),
            parent,
            window: factory(parent),
            state,

            _t: PhantomData,
        })
    }

    pub fn parent(&self) -> CWindow {
        Arc::clone(&self.parent)
    }

    pub fn window(&self) -> CWindow {
        Arc::clone(&self.window)
    }

    pub fn state(&self) -> Arc<State> {
        Arc::clone(&self.state)
    }
}

impl<T> Deref for InnerWindow<T>
where
    T: WindowBuilder<T> + Window<T>,
{
    type Target = CWindow;
    fn deref(&self) -> &Self::Target {
        &self.window()
    }
}

fn insert_into(window: CWindow, color: impl Into<ColorPair>, s: impl AsRef<str>, eol: bool) {
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

impl<T> Outputter for InnerWindow<T>
where
    T: WindowBuilder<T> + Window<T>,
{
    fn output(&self, output: impl Into<Output>, eol: bool) {
        let window = self.window();
        let output = output.into();
        for (r, cp) in output.colors.iter() {
            insert_into(
                Arc::clone(&window),
                *cp,
                &output.data[r.start..r.end],
                false,
            )
        }
        if eol {
            self.window().addch('\n');
        }
    }
}

impl<T> MessageReceiver<Request> for InnerWindow<T>
where
    T: WindowBuilder<T> + Window<T>,
{
    fn queue(&self, req: impl Into<Request>) {
        self.queue.enqueue(req.into())
    }
}

impl<T> inputbuffer::MoveableCursor for InnerWindow<T>
where
    T: WindowBuilder<T> + Window<T>,
{
    fn move_cursor(&self, pos: usize) {
        self.mv(self.get_cur_y(), pos as i32);
    }

    fn clear(&self) {
        self.window().clear();
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
