use super::*;

pub struct Output {
    parent: Rc<Window>,
    window: Window,
    ctx: Rc<Context>,
}

impl Output {
    pub fn new(parent: Rc<Window>, ctx: Rc<Context>) -> Self {
        let (h, w) = parent.get_max_yx();
        let window = parent
            .subwin(h - 1, w, 0, 0)
            .expect("create output subwindow");

        window.setscrreg(0, window.get_max_y());
        window.scrollok(true);

        Self {
            parent,
            window: window.into(),
            ctx,
        }
    }
}

//        self.window.resize(rows, cols);
//   pub fn clear(&self) {
//        self.window.erase();
//        self.window.refresh();
//    }

impl_recv!(Output);
