use super::*;
use pancurses;
use std::sync::Arc;

pub struct OutputWindow {
    parent: Arc<pancurses::Window>,
    window: pancurses::Window,
}

impl OutputWindow {
    pub fn new(parent: Arc<pancurses::Window>) -> Self {
        let bounds = parent.get_max_yx();

        let window = parent
            .subwin(bounds.0 - 1, bounds.1 - bounds.1 / 5, 0, 0)
            .expect("create output subwindow");
        window.setscrreg(0, window.get_max_y());
        window.scrollok(true);

        Self { parent, window }
    }

    pub fn clear(&self) {
        self.window.erase();
        self.window.refresh();
    }
}

impl Window for OutputWindow {
    fn window(&self) -> &pancurses::Window {
        &self.window
    }
}
